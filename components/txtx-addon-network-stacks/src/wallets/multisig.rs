use clarity::address::AddressHashMode;
use clarity::types::chainstate::{StacksAddress, StacksPublicKey};
use clarity::util::secp256k1::Secp256k1PublicKey;
use clarity::{codec::StacksMessageCodec, util::secp256k1::MessageSignature};
use std::collections::{HashMap, VecDeque};
use txtx_addon_kit::types::commands::{CommandExecutionResult, CommandSpecification};
use txtx_addon_kit::types::types::{PrimitiveValue, RunbookSupervisionContext};

use crate::{
    codec::codec::{
        StacksTransaction, TransactionAuth, TransactionAuthField, TransactionAuthFlags,
        TransactionPublicKeyEncoding, TransactionSpendingCondition, Txid,
    },
    constants::{MESSAGE_BYTES, SIGNED_MESSAGE_BYTES},
    typing::STACKS_SIGNATURE,
};
use txtx_addon_kit::types::frontend::{
    ActionItemRequest, ActionItemRequestType, ActionItemRequestUpdate, ActionItemStatus, Actions,
    BlockEvent, OpenModalData,
};
use txtx_addon_kit::types::wallets::{
    consolidate_wallet_activate_result, consolidate_wallet_result, CheckSignabilityOk,
    SigningCommandsState, WalletActionErr, WalletActionsFutureResult, WalletActivateFutureResult,
    WalletImplementation, WalletInstance, WalletSignFutureResult, WalletSpecification,
};
use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{ConstructDid, Did, ValueStore};
use txtx_addon_kit::{channel, AddonDefaults};

use crate::constants::{
    ACTION_ITEM_CHECK_BALANCE, ACTION_ITEM_PROVIDE_PUBLIC_KEY, ACTION_OPEN_MODAL,
    CHECKED_PUBLIC_KEY, NETWORK_ID, PUBLIC_KEYS, REQUIRED_SIGNATURE_COUNT, RPC_API_URL,
    SIGNED_TRANSACTION_BYTES,
};
use crate::rpc::StacksRpc;

use super::get_addition_actions_for_address;

lazy_static! {
    pub static ref STACKS_MULTISIG: WalletSpecification = define_wallet! {
        StacksConnect => {
          name: "Stacks Multisig",
          matcher: "multisig",
          documentation:txtx_addon_kit::indoc! {r#"The `multisig` wallet creates an ordered, `n` of `n` multisig.
          Each of the specified signers can be any other supported wallet type, and will be prompted to sign in the appropriate order."#},
          inputs: [
            signers: {
              documentation: "A list of signers that make up the multisig.",
                typing: Type::array(Type::string()),
                optional: false,
                interpolable: true,
                sensitive: false
            },
            expected_address: {
              documentation: "The multisig address that is expected to be created from combining the public keys of all parties. Omitting this field will allow any address to be used for this wallet.",
                typing: Type::string(),
                optional: true,
                interpolable: true,
                sensitive: false
            },
            required_signatures: {
              documentation: "The number of signatures required. This value must be between 1 and the number of signers. If this value is equal to the number of signers, an `n` of `n` multisig address is generated. If this value is less than the number of signers, an `m` of `n` multisig address is generated. If omitted, the number of signers will be used.",
                typing: Type::uint(),
                optional: true,
                interpolable: true,
                sensitive: false
            }
          ],
          outputs: [
              public_key: {
                documentation: "The public key of the generated multisig wallet.",
                typing: Type::array(Type::buffer())
              },
              signers: {
                documentation: "The list of signers that make up the multisig.",
                typing: Type::array(Type::string())
              }
          ],
          example: txtx_addon_kit::indoc! {r#"
            wallet "alice" "stacks::connect" {
                expected_address = "ST2JHG361ZXG51QTKY2NQCVBPPRRE2KZB1HR05NNC"
            }

            wallet "bob" "stacks::connect" {
                expected_address = "ST2NEB84ASENDXKYGJPQW86YXQCEFEX2ZQPG87ND"
            }

            wallet "alice_and_bob" "stacks::multisig" {
                signers = [wallet.alice, wallet.bob]
            }
    "#},
      }
    };
}

pub struct StacksConnect;
impl WalletImplementation for StacksConnect {
    fn check_instantiability(
        _ctx: &WalletSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    // Loop over the signers
    // Ensuring that they are all correctly activable.
    // When they are, collect the public keys
    // and build the stacks address + Check the balance
    #[cfg(not(feature = "wasm"))]
    fn check_activability(
        construct_did: &ConstructDid,
        instance_name: &str,
        _spec: &WalletSpecification,
        args: &ValueStore,
        mut signing_command_state: ValueStore,
        mut wallets: SigningCommandsState,
        wallets_instances: &HashMap<ConstructDid, WalletInstance>,
        defaults: &AddonDefaults,
        supervision_context: &RunbookSupervisionContext,
        is_balance_check_required: bool,
        _is_public_key_required: bool,
    ) -> WalletActionsFutureResult {
        use txtx_addon_kit::types::frontend::ReviewInputRequest;

        let root_construct_did = construct_did.clone();
        let signers = get_signers(args, wallets_instances);

        let args = args.clone();
        let wallets_instances = wallets_instances.clone();
        let defaults = defaults.clone();
        let supervision_context = supervision_context.clone();
        let instance_name = instance_name.to_string();
        let expected_address: Option<String> = None;
        let rpc_api_url = match args.get_defaulting_string(RPC_API_URL, &defaults) {
            Ok(value) => value,
            Err(diag) => return Err((wallets, signing_command_state, diag)),
        };
        let network_id = match args.get_defaulting_string(NETWORK_ID, &defaults) {
            Ok(value) => value,
            Err(diag) => return Err((wallets, signing_command_state, diag)),
        };

        let signer_count = signers.len() as u16;
        let required_signature_count: u16 = args
            .get_uint("required_signatures")
            .and_then(|count| Some(count.try_into().unwrap_or(signer_count).max(1)))
            .unwrap_or(signer_count);

        signing_command_state.insert(
            REQUIRED_SIGNATURE_COUNT,
            Value::uint(required_signature_count as u64),
        );

        let future = async move {
            let mut consolidated_actions = Actions::none();

            // Modal configuration
            let modal =
                BlockEvent::new_modal("Stacks Multisig Configuration assistant", "", vec![]);
            let mut open_modal_action = vec![ActionItemRequest::new(
                &Some(root_construct_did.clone()),
                "Computed multisig address",
                Some("Multisig addresses are computed by hashing the public keys of all participants.".into()),
                ActionItemStatus::Todo,
                ActionItemRequestType::OpenModal(OpenModalData {
                    modal_uuid: modal.uuid.clone(),
                    title: "OPEN ASSISTANT".into(),
                }),
                ACTION_ITEM_PROVIDE_PUBLIC_KEY,
            )];
            let mut additional_actions_res = get_addition_actions_for_address(
                &expected_address,
                &root_construct_did,
                &instance_name,
                &network_id,
                &rpc_api_url,
                false,
                is_balance_check_required,
                false,
            )
            .await;
            match additional_actions_res {
                Ok(ref mut res) => {
                    open_modal_action.append(res);
                }
                Err(diag) => return Err((wallets, signing_command_state, diag)),
            }

            consolidated_actions.push_sub_group(open_modal_action);
            consolidated_actions.push_modal(modal);

            // Modal configuration
            let mut checked_public_keys = HashMap::new();
            for (signing_construct_did, wallet_instance) in signers.iter() {
                let signer_signing_command_state = wallets
                    .pop_signing_command_state(&signing_construct_did)
                    .unwrap();
                let future = (wallet_instance.specification.check_activability)(
                    &signing_construct_did,
                    &wallet_instance.name,
                    &wallet_instance.specification,
                    &args,
                    signer_signing_command_state,
                    wallets,
                    &wallets_instances,
                    &defaults,
                    &supervision_context,
                    false,
                    true,
                )?;
                let (updated_wallets, mut actions) = match future.await {
                    Ok(res) => consolidate_wallet_result(Ok(res)).unwrap(),
                    Err(e) => return Err(e),
                };
                wallets = updated_wallets;
                consolidated_actions.append(&mut actions);

                let signer_signing_command_state = wallets
                    .get_signing_command_state(&signing_construct_did)
                    .unwrap();

                if let Ok(checked_public_key) =
                    signer_signing_command_state.get_expected_value(CHECKED_PUBLIC_KEY)
                {
                    checked_public_keys.insert(signing_construct_did, checked_public_key.clone());
                }
            }

            if signers.len() == checked_public_keys.len() {
                let mut ordered_public_keys = vec![];
                let mut ordered_parsed_public_keys = vec![];
                for (signer_uuid, _) in signers.iter() {
                    if let Some(public_key) = checked_public_keys.remove(signer_uuid) {
                        ordered_public_keys.push(public_key.clone());
                        let bytes = public_key.expect_buffer_bytes();
                        let public_key = match Secp256k1PublicKey::from_slice(&bytes) {
                            Ok(public_key) => public_key,
                            Err(e) => {
                                return Err((
                                    wallets,
                                    signing_command_state,
                                    diagnosed_error!(
                                        "unable to parse public key {}",
                                        e.to_string()
                                    ),
                                ));
                            }
                        };
                        ordered_parsed_public_keys.push(public_key);
                    }
                }
                signing_command_state.insert(CHECKED_PUBLIC_KEY, Value::array(ordered_public_keys));

                let version = if network_id.eq("mainnet") {
                    clarity_repl::clarity::address::C32_ADDRESS_VERSION_MAINNET_MULTISIG
                } else {
                    clarity_repl::clarity::address::C32_ADDRESS_VERSION_TESTNET_MULTISIG
                };

                if let Some(stacks_address) = StacksAddress::from_public_keys(
                    version,
                    &AddressHashMode::SerializeP2SH,
                    required_signature_count.into(),
                    &ordered_parsed_public_keys,
                )
                .map(|address| address.to_string())
                {
                    let mut actions = Actions::none();
                    if is_balance_check_required {
                        let stacks_rpc = StacksRpc::new(&rpc_api_url);
                        let (status_update, value) =
                            match stacks_rpc.get_balance(&stacks_address).await {
                                Ok(response) => (
                                    ActionItemStatus::Success(None),
                                    Value::string(response.get_formatted_balance()),
                                ),
                                Err(e) => {
                                    let diag = diagnosed_error!(
                                        "unable to retrieve balance {}: {}",
                                        stacks_address,
                                        e.to_string()
                                    );

                                    (ActionItemStatus::Error(diag), Value::string("N/A".into()))
                                }
                            };

                        actions.push_action_item_update(
                            ActionItemRequestUpdate::from_context(
                                &root_construct_did,
                                ACTION_ITEM_CHECK_BALANCE,
                            )
                            .set_type(ActionItemRequestType::ReviewInput(ReviewInputRequest {
                                input_name: "".into(),
                                value,
                            }))
                            .set_status(status_update),
                        );
                    }
                    actions.push_action_item_update(
                        ActionItemRequestUpdate::from_context(
                            &root_construct_did,
                            ACTION_ITEM_PROVIDE_PUBLIC_KEY,
                        )
                        .set_status(ActionItemStatus::Success(Some(stacks_address))),
                    );
                    consolidated_actions = actions;
                } else {
                    println!("Unable to compute Stacks address");
                }
            } else {
                let validate_modal_action = ActionItemRequest::new(
                    &Some(root_construct_did.clone()),
                    "CONFIRM",
                    None,
                    ActionItemStatus::Todo,
                    ActionItemRequestType::ValidateModal,
                    "modal",
                );
                consolidated_actions.push_group("", vec![validate_modal_action]);
            }

            Ok((wallets, signing_command_state, consolidated_actions))
        };
        Ok(Box::pin(future))
    }

    fn activate(
        _construct_id: &ConstructDid,
        _spec: &WalletSpecification,
        args: &ValueStore,
        mut signing_command_state: ValueStore,
        mut wallets: SigningCommandsState,
        wallets_instances: &HashMap<ConstructDid, WalletInstance>,
        defaults: &AddonDefaults,
        progress_tx: &channel::Sender<BlockEvent>,
    ) -> WalletActivateFutureResult {
        let public_key = match signing_command_state.get_expected_value(CHECKED_PUBLIC_KEY) {
            Ok(value) => value.clone(),
            Err(diag) => return Err((wallets, signing_command_state, diag)),
        };
        let network_id = match args.get_defaulting_string(NETWORK_ID, defaults) {
            Ok(value) => value,
            Err(diag) => return Err((wallets, signing_command_state, diag)),
        };

        let signers = get_signers(args, wallets_instances);

        let args = args.clone();
        let wallets_instances = wallets_instances.clone();
        let defaults = defaults.clone();
        let progress_tx = progress_tx.clone();

        #[cfg(not(feature = "wasm"))]
        let future = async move {
            let mut result = CommandExecutionResult::new();

            // Modal configuration
            let mut signers_uuids = vec![];
            for (signing_construct_did, wallet_instance) in signers.into_iter() {
                signers_uuids.push(Value::string(signing_construct_did.value().to_string()));
                let signer_signing_command_state = wallets
                    .pop_signing_command_state(&signing_construct_did)
                    .unwrap();
                let future = (wallet_instance.specification.activate)(
                    &signing_construct_did,
                    &wallet_instance.specification,
                    &args,
                    signer_signing_command_state,
                    wallets,
                    &wallets_instances,
                    &defaults,
                    &progress_tx,
                )?;
                let (updated_wallets, _) =
                    consolidate_wallet_activate_result(Ok(future.await?)).unwrap();
                wallets = updated_wallets;
            }

            signing_command_state.insert(PUBLIC_KEYS, public_key.clone());

            let version = match network_id.as_str() {
                "mainnet" => AddressHashMode::SerializeP2SH.to_version_mainnet(),
                _ => AddressHashMode::SerializeP2SH.to_version_testnet(),
            };
            signing_command_state.insert("hash_flag", Value::uint(version.into()));
            signing_command_state.insert("multi_sig", Value::bool(true));
            signing_command_state.insert("signers", Value::array(signers_uuids.clone()));

            result
                .outputs
                .insert("signers".into(), Value::array(signers_uuids));
            result.outputs.insert("public_key".into(), public_key);

            Ok((wallets, signing_command_state, result))
        };
        #[cfg(feature = "wasm")]
        panic!("async commands are not enabled for wasm");
        #[cfg(not(feature = "wasm"))]
        Ok(Box::pin(future))
    }

    fn check_signability(
        origin_uuid: &ConstructDid,
        title: &str,
        description: &Option<String>,
        payload: &Value,
        _spec: &WalletSpecification,
        args: &ValueStore,
        mut signing_command_state: ValueStore,
        mut wallets: SigningCommandsState,
        wallets_instances: &HashMap<ConstructDid, WalletInstance>,
        defaults: &AddonDefaults,
        supervision_context: &RunbookSupervisionContext,
    ) -> Result<CheckSignabilityOk, WalletActionErr> {
        let signers = get_signers(&signing_command_state, wallets_instances);
        let mut consolidated_actions = Actions::none();

        // Set up modal
        {
            let modal = BlockEvent::new_modal("Stacks Multisig Signing Assistant", "", vec![]);
            let action = ActionItemRequest::new(
                &Some(origin_uuid.clone()),
                "Sign Multisig Transaction",
                Some("All parties of the multisig must sign the transaction.".into()),
                ActionItemStatus::Todo,
                ActionItemRequestType::OpenModal(OpenModalData {
                    modal_uuid: modal.uuid.clone(),
                    title: "OPEN ASSISTANT".into(),
                }),
                ACTION_OPEN_MODAL,
            );
            let open_modal_action = vec![action];
            consolidated_actions.push_sub_group(open_modal_action);
            consolidated_actions.push_modal(modal);
        }

        let payload_bytes = payload.expect_buffer_bytes();
        let unsigned_tx =
            StacksTransaction::consensus_deserialize(&mut &payload_bytes[..]).unwrap();
        let (signature_count, payloads) = generate_ordered_multisig_payloads(
            &origin_uuid.to_string(),
            unsigned_tx.clone(),
            &signers,
            &wallets,
        )
        .map_err(|e| {
            (
                wallets.clone(),
                signing_command_state.clone(),
                diagnosed_error!("{}", e),
            )
        })?;

        let required_signature_count = signing_command_state
            .get_expected_uint(REQUIRED_SIGNATURE_COUNT)
            .unwrap();

        if signature_count >= required_signature_count {
            println!(
                "=> {:?} of {:?} signatures acquired",
                signature_count, required_signature_count
            );
            let tx = generate_signed_ordered_multisig_tx(
                &origin_uuid.value().to_string(),
                unsigned_tx.clone(),
                &signers,
                &wallets,
                required_signature_count,
            )
            .map_err(|e| {
                (
                    wallets.clone(),
                    signing_command_state.clone(),
                    diagnosed_error!("{}", e),
                )
            })?;

            println!("verifying tx: {:?}", tx);
            tx.verify().map_err(|e| {
                (
                    wallets.clone(),
                    signing_command_state.clone(),
                    diagnosed_error!("multisig generated invalid Stacks transaction: {}", e),
                )
            })?;
            let mut signed_tx_bytes = vec![];
            tx.consensus_serialize(&mut signed_tx_bytes).unwrap();

            signing_command_state.insert_scoped_value(
                &origin_uuid.value().to_string(),
                SIGNED_TRANSACTION_BYTES,
                Value::string(txtx_addon_kit::hex::encode(signed_tx_bytes)),
            );
            // we know that there are no pending actions because we're in all_signed,
            // so we don't want to include the actions to open the modal
            consolidated_actions = Actions::none();
            // update "open modal assistant" button status
            consolidated_actions.push_action_item_update(
                ActionItemRequestUpdate::from_context(&origin_uuid, ACTION_OPEN_MODAL).set_status(
                    ActionItemStatus::Success(Some(format!("All signers participated"))),
                ),
            );
        } else {
            set_signer_states(
                &origin_uuid.to_string(),
                &signers,
                &mut wallets,
                required_signature_count,
                signature_count,
            );

            for (signer_uuid, signer_wallet_instance) in signers.into_iter() {
                let signer_wallet_state = wallets.pop_signing_command_state(&signer_uuid).unwrap();
                let payload = payloads.get(&signer_uuid).unwrap();
                let (mut updated_wallets, signer_wallet_state, mut actions) =
                    (signer_wallet_instance.specification.check_signability)(
                        &origin_uuid,
                        &format!("{} - {}", title, signer_wallet_instance.name),
                        description,
                        &payload,
                        &signer_wallet_instance.specification,
                        &args,
                        signer_wallet_state.clone(),
                        wallets,
                        &wallets_instances,
                        &defaults,
                        &supervision_context,
                    )?;
                updated_wallets.push_signing_command_state(signer_wallet_state.clone());
                consolidated_actions.append(&mut actions);
                wallets = updated_wallets;
            }
            let validate_modal_action = ActionItemRequest::new(
                &Some(origin_uuid.clone()),
                "CONFIRM",
                None,
                ActionItemStatus::Todo,
                ActionItemRequestType::ValidateModal,
                "modal",
            );
            consolidated_actions.push_group("", vec![validate_modal_action]);
        }

        Ok((wallets, signing_command_state, consolidated_actions))
    }

    #[cfg(not(feature = "wasm"))]
    fn sign(
        origin_uuid: &ConstructDid,
        _title: &str,
        payload: &Value,
        _spec: &WalletSpecification,
        args: &ValueStore,
        signing_command_state: ValueStore,
        mut wallets: SigningCommandsState,
        wallets_instances: &HashMap<ConstructDid, WalletInstance>,
        defaults: &AddonDefaults,
    ) -> WalletSignFutureResult {
        use txtx_addon_kit::futures::future;

        if let Some(signed_transaction_bytes) = signing_command_state
            .get_scoped_value(&origin_uuid.value().to_string(), SIGNED_TRANSACTION_BYTES)
        {
            println!("multisig sign found signed tx in wallet state");
            let mut result = CommandExecutionResult::new();
            result.outputs.insert(
                SIGNED_TRANSACTION_BYTES.into(),
                signed_transaction_bytes.clone(),
            );

            return Ok(Box::pin(future::ready(Ok((
                wallets,
                signing_command_state,
                result,
            )))));
        }
        println!("multisig sign did not find signed tx in wallet state");

        let signers = get_signers(&signing_command_state, wallets_instances);
        let args = args.clone();
        let wallets_instances = wallets_instances.clone();
        let defaults = defaults.clone();

        let payload = payload.clone();

        let future = async move {
            let mut result = CommandExecutionResult::new();

            let transaction_payload_bytes = payload.expect_buffer_bytes();
            let mut transaction =
                StacksTransaction::consensus_deserialize(&mut &transaction_payload_bytes[..])
                    .unwrap();
            let mut presign_input = transaction.sign_begin();

            for (signing_construct_did, wallet_instance) in signers.into_iter() {
                let signing_command_state = wallets
                    .pop_signing_command_state(&signing_construct_did)
                    .unwrap();

                let payload = Value::buffer(
                    TransactionSpendingCondition::make_sighash_presign(
                        &presign_input,
                        &TransactionAuthFlags::AuthStandard,
                        transaction.get_tx_fee(),
                        transaction.get_origin_nonce(),
                    )
                    .to_bytes()
                    .to_vec(),
                    STACKS_SIGNATURE.clone(),
                );

                let future = (wallet_instance.specification.sign)(
                    &signing_construct_did,
                    &wallet_instance.name,
                    &payload,
                    &wallet_instance.specification,
                    &args,
                    signing_command_state,
                    wallets,
                    &wallets_instances,
                    &defaults,
                )?;

                let (updated_wallets, updated_results) =
                    consolidate_wallet_activate_result(Ok(future.await?)).unwrap();
                wallets = updated_wallets;
                let updated_message = updated_results.outputs.get(MESSAGE_BYTES).unwrap().clone();
                let signature = updated_results
                    .outputs
                    .get(SIGNED_MESSAGE_BYTES)
                    .unwrap()
                    .clone();

                match transaction.auth {
                    TransactionAuth::Standard(ref mut spending_condition) => {
                        match spending_condition {
                            TransactionSpendingCondition::Multisig(data) => {
                                let signature =
                                    MessageSignature::from_vec(&signature.expect_buffer_bytes())
                                        .unwrap();
                                let key_encoding = TransactionPublicKeyEncoding::Compressed;
                                data.fields
                                    .push(TransactionAuthField::Signature(key_encoding, signature));
                            }
                            _ => unreachable!(),
                        }
                    }
                    _ => unreachable!(),
                }
                presign_input = Txid::from_bytes(&updated_message.expect_buffer_bytes()).unwrap();
                result = updated_results;
            }

            let mut bytes = vec![];
            transaction.consensus_serialize(&mut bytes).unwrap(); // todo
            let transaction_bytes = Value::string(txtx_addon_kit::hex::encode(bytes));

            transaction.verify().unwrap();

            result
                .outputs
                .insert(SIGNED_TRANSACTION_BYTES.into(), transaction_bytes);

            Ok((wallets, signing_command_state, result))
        };
        Ok(Box::pin(future))
    }
}

fn get_signers(
    args: &ValueStore,
    wallets_instances: &HashMap<ConstructDid, WalletInstance>,
) -> Vec<(ConstructDid, WalletInstance)> {
    let signers_uuid = args.get_expected_array("signers").unwrap();
    let mut signers = Vec::new();
    for signer_uuid in signers_uuid.iter() {
        let uuid = signer_uuid.as_string().unwrap();
        let uuid = ConstructDid(Did::from_hex_string(uuid));
        let wallet_instance = wallets_instances.get(&uuid).unwrap().clone();
        signers.push((uuid, wallet_instance));
    }
    signers
}

/// Takes an unsigned [StacksTransaction], a set of signers, and a [WalletsState] for each signer
/// that _could_ contain a signature, and generates a next payload for each signer.
///
/// Each signer's payload will be identical to the original transaction, except the multisig auth's
/// `fields` will be updated.
/// Here is an example set of fields for three signers across multiple states:
/// ```no_run
/// // Before anyone has signed, the first signer should have no fields,
/// // and subsequent signers should apply their signature on top of signer n-1's
/// // pub key
/// Starting State
///     Alice   -> Some([])
///     Bob     -> Some([alice_pubkey])
///     Charlie -> Some([alice_pubkey, bob_pubkey])
/// ---
/// // If Bob signs before Alice, Alice can no longer sign without invalidating
/// // Bob's signature. Bob's signature is pushed onto Bob's fields, and Charlie's
/// // payload will contain Bob's signature.
/// Bob Signs
///     Alice   -> None
///     Bob     -> Some([alice_pubkey, bob_signature])
///     Charlie -> Some([alice_pubkey, bob_signature])
/// ---
/// Charlie Signs
///     Alice   -> None
///     Bob     -> Some([alice_pubkey, bob_signature])
///     Charlie -> Some([alice_pubkey, bob_signature, charlie_signature])
/// ```
///
fn generate_ordered_multisig_payloads(
    origin_uuid: &str,
    tx: StacksTransaction,
    signers: &Vec<(ConstructDid, WalletInstance)>,
    wallets: &SigningCommandsState,
) -> Result<(u64, HashMap<ConstructDid, Value>), String> {
    let mut payloads = HashMap::new();
    let mut signature_count = 0;
    println!("generate_ordered_multisig_payloads");

    // Loop over each signer to compute what their auth's `fields` should be
    for (this_signer_idx, (this_signer_uuid, _)) in signers.iter().enumerate() {
        let this_wallet_state = wallets
            .get_signing_command_state(&this_signer_uuid)
            .unwrap();

        // along the way, track how many signers have completed signatures
        signature_count += this_wallet_state
            .get_scoped_value(origin_uuid, SIGNED_TRANSACTION_BYTES)
            // if we have a signature for this signer, check if it's null. if null, this signer was skipped so don't add it to our count
            .map(|v| v.as_null().and_then(|_| Some(0)).unwrap_or(1))
            .unwrap_or(0);

        // this signer's fields depend on the previous signers in the order, so we look back through the signers
        // note: the first signer can't look back and will thus have empty `fields`
        let mut this_signer_fields = VecDeque::new();
        if this_signer_idx > 0 {
            let mut idx = this_signer_idx - 1;

            while let Some((previous_signer_uuid, previous_wallet_instance)) = signers.get(idx) {
                let previous_wallet_state = wallets
                    .get_signing_command_state(&previous_signer_uuid)
                    .unwrap();

                let signature =
                    previous_wallet_state.get_scoped_value(origin_uuid, SIGNED_TRANSACTION_BYTES);
                println!("signature for signer {}: {:?}", idx, signature);
                // check if the previous signer has provided a signature.
                // if so, we need to push on the signature portion of the previous signer's auth `field`.
                // if not, we should use the previous signer's public key
                let field = match previous_wallet_state
                    .get_scoped_value(origin_uuid, SIGNED_TRANSACTION_BYTES)
                {
                    Some(&Value::Primitive(PrimitiveValue::Null)) | None => {
                        let public_key = previous_wallet_state
                            .get_expected_value(CHECKED_PUBLIC_KEY)
                            .unwrap();
                        let stacks_public_key =
                            StacksPublicKey::from_hex(&public_key.expect_string()).unwrap();
                        TransactionAuthField::PublicKey(stacks_public_key)
                    }
                    Some(signed_tx_bytes) => {
                        let bytes = signed_tx_bytes.expect_buffer_bytes();
                        let signed_tx =
                            StacksTransaction::consensus_deserialize(&mut &bytes[..]).unwrap();

                        // we're expecting the auth value to contain a multisig spending condition
                        // note: if this poses a problem, we may want to just default to providing
                        // the public key
                        extract_auth_field_from_auth(&signed_tx.auth, idx).ok_or(format!(
                            "missing expected auth field for multisig signer {:?}",
                            previous_wallet_instance.name
                        ))?
                    }
                };
                this_signer_fields.push_front(field);
                if idx == 0 {
                    break;
                }
                idx -= 1;
            }
        }
        let fields = this_signer_fields
            .drain(..)
            .collect::<Vec<TransactionAuthField>>();

        let mut tx: StacksTransaction = tx.clone();
        let TransactionAuth::Standard(TransactionSpendingCondition::Multisig(
            mut spending_condition,
        )) = tx.auth
        else {
            return Err("expected multisig spending condition for multisig transaction".into());
        };

        spending_condition.fields = fields;
        tx.auth =
            TransactionAuth::Standard(TransactionSpendingCondition::Multisig(spending_condition));
        let mut bytes = vec![];
        tx.consensus_serialize(&mut bytes).unwrap();
        let retrieved = StacksTransaction::consensus_deserialize(&mut &bytes[..]).unwrap();
        println!("retrieved: {:?}", retrieved);
        let payload = Value::buffer(bytes, STACKS_SIGNATURE.clone());
        payloads.insert(this_signer_uuid.clone(), payload);
    }

    Ok((signature_count, payloads))
}

/// Takes an unsigned [StacksTransaction], a set of signers , and a [WalletsState] for each signer
/// that _could_ contain a signature, and generates a signed [StacksTransaction] containing each
/// signer's signature (if available) or public key.
///
/// Building off the example from [generate_ordered_multisig_payloads], the fields for the signed tx would be:
/// ```no_run
/// [alice_pubkey, bob_signature, charlie_signature]
/// ```
fn generate_signed_ordered_multisig_tx(
    origin_uuid: &str,
    mut tx: StacksTransaction,
    signers: &Vec<(ConstructDid, WalletInstance)>,
    wallets: &SigningCommandsState,
    required_signature_count: u64,
) -> Result<StacksTransaction, String> {
    let mut fields = vec![];
    let mut signatures_tracked = 0;
    for (signer_idx, (signer_uuid, wallet_instance)) in signers.iter().enumerate() {
        let signer_wallet_state = wallets.get_signing_command_state(&signer_uuid).unwrap();
        let field = match signer_wallet_state
            .get_scoped_value(origin_uuid, SIGNED_TRANSACTION_BYTES)
        {
            Some(&Value::Primitive(PrimitiveValue::Null)) | None => {
                let public_key = signer_wallet_state
                    .get_expected_value(CHECKED_PUBLIC_KEY)
                    .unwrap();
                let stacks_public_key =
                    StacksPublicKey::from_hex(&public_key.expect_string()).unwrap();
                TransactionAuthField::PublicKey(stacks_public_key)
            }
            Some(signed_tx_bytes) => {
                signatures_tracked += 1;
                let bytes = signed_tx_bytes.expect_buffer_bytes();
                let signed_tx = StacksTransaction::consensus_deserialize(&mut &bytes[..]).unwrap();
                // we're expecting the auth value to contain a multisig spending condition
                // note: if this poses a problem, we may want to just default to providing
                // the public key
                extract_auth_field_from_auth(&signed_tx.auth, signer_idx).ok_or(format!(
                    "missing expected auth field for multisig signer {:?}",
                    wallet_instance.name
                ))?
            }
        };
        fields.push(field);
        // if signatures_tracked == required_signature_count {
        //     break;
        // }
    }

    let TransactionAuth::Standard(TransactionSpendingCondition::Multisig(mut spending_condition)) =
        tx.auth
    else {
        return Err("expected multisig spending condition for multisig transaction".into());
    };
    spending_condition.fields = fields;
    spending_condition.signatures_required = required_signature_count.try_into().unwrap();
    tx.auth = TransactionAuth::Standard(TransactionSpendingCondition::Multisig(spending_condition));
    println!("signed ordered multisig: {:?}", tx);
    Ok(tx)
}

fn extract_auth_field_from_auth(
    auth: &TransactionAuth,
    index: usize,
) -> Option<TransactionAuthField> {
    match auth {
        TransactionAuth::Standard(TransactionSpendingCondition::Multisig(spending_condition)) => {
            let Some(auth_field) = spending_condition.fields.get(index) else {
                return None;
            };
            Some(auth_field.clone())
        }
        _ => None,
    }
}

fn set_signer_states(
    origin_uuid: &str,
    signers: &Vec<(ConstructDid, WalletInstance)>,
    wallets: &mut SigningCommandsState,
    required_signatures: u64,
    signature_count: u64,
) {
    let signer_count = signers.len() as u64;
    let remaining_signatures_required = required_signatures - signature_count;

    let downstream_signature_counts =
        signers
            .iter()
            .fold(VecDeque::new(), |mut acc, (signer_uuid, _)| {
                let len = acc.len();
                let increment = wallets
                    .get_signing_command_state(signer_uuid)
                    .unwrap()
                    .get_scoped_value(origin_uuid, SIGNED_TRANSACTION_BYTES)
                    // if we have a signature for this signer, check if it's null. if null, this signer was skipped so don't add it to our count
                    .map(|v| v.as_null().and_then(|_| Some(0)).unwrap_or(1))
                    .unwrap_or(0);

                if len == 0 {
                    acc.push_front(increment);
                } else {
                    let prev = acc[len - 1];
                    acc.push_front(increment + prev);
                }
                acc
            });

    println!(
        "downstream signature counts: {:?}",
        downstream_signature_counts
    );

    for (signer_idx, (signer_uuid, _)) in signers.iter().enumerate() {
        let mut signing_command_state = wallets.pop_signing_command_state(&signer_uuid).unwrap();

        let this_signer_signed = signing_command_state
            .get_scoped_value(origin_uuid, SIGNED_TRANSACTION_BYTES)
            .is_some();

        // if the signer has already signed, they can't sign again and are not skippable
        if this_signer_signed {
            signing_command_state.insert_scoped_value(
                &origin_uuid,
                "SIGNATURE_SKIPPABLE",
                Value::bool(false),
            );

            signing_command_state.insert_scoped_value(
                &origin_uuid,
                "SIGNATURE_SIGNABLE",
                Value::bool(false),
            );
        } else {
            // if this signer is skipped, will the remaining number of possible signers be enough to reach required signatures?
            // if so, this one is safe to skip. if not, not skippable
            let next_signer_idx = (signer_idx + 1) as u64;
            let eligible_signers_after_this_signer = signer_count - next_signer_idx;
            signing_command_state.insert_scoped_value(
                &origin_uuid,
                "SIGNATURE_SKIPPABLE",
                Value::bool(eligible_signers_after_this_signer >= remaining_signatures_required),
            );

            // look ahead at signers. if any of them signed, then this one is not signable
            let downstream_signatures = downstream_signature_counts.get(signer_idx).unwrap_or(&0);
            signing_command_state.insert_scoped_value(
                &origin_uuid,
                "SIGNATURE_SIGNABLE",
                Value::bool(downstream_signatures.eq(&0)),
            );
        }

        wallets.push_signing_command_state(signing_command_state);
    }
}
