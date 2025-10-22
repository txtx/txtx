use clarity::address::AddressHashMode;
use clarity::types::chainstate::StacksAddress;
use clarity::util::secp256k1::Secp256k1PublicKey;
use clarity::{codec::StacksMessageCodec, util::secp256k1::MessageSignature};
use std::collections::{HashMap, VecDeque};
use txtx_addon_kit::constants::{SignerKey};
use txtx_addon_kit::types::commands::{CommandExecutionResult, CommandSpecification};
use txtx_addon_kit::types::types::RunbookSupervisionContext;

use crate::codec::codec::expect_stacks_public_key;
use crate::{
    codec::codec::{
        StacksTransaction, TransactionAuth, TransactionAuthField, TransactionAuthFlags,
        TransactionPublicKeyEncoding, TransactionSpendingCondition, Txid,
    },
    constants::MESSAGE_BYTES,
};
use txtx_addon_kit::channel;
use txtx_addon_kit::types::frontend::{
    ActionItemRequest, ActionItemRequestType, ActionItemRequestUpdate, ActionItemStatus, Actions,
    BlockEvent, OpenModalData,
};
use txtx_addon_kit::types::signers::{
    consolidate_signer_activate_result, consolidate_signer_result, CheckSignabilityOk,
    SignerActionErr, SignerActionsFutureResult, SignerActivateFutureResult, SignerImplementation,
    SignerInstance, SignerSignFutureResult, SignerSpecification, SignersState,
};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{ConstructDid, Did};

use crate::constants::{
    ActionItemKey::CheckBalance, ActionItemKey::ProvidePublicKey, ACTION_OPEN_MODAL, CHECKED_ADDRESS,
    CHECKED_PUBLIC_KEY, FORMATTED_TRANSACTION, IS_SIGNABLE, NETWORK_ID, PUBLIC_KEYS,
    REQUIRED_SIGNATURE_COUNT, RPC_API_URL,
};
use crate::rpc::StacksRpc;

use super::get_addition_actions_for_address;

lazy_static! {
    pub static ref STACKS_MULTISIG: SignerSpecification = define_signer! {
        StacksConnect => {
            name: "Stacks Multisig",
            matcher: "multisig",
            documentation:txtx_addon_kit::indoc! {r#"The `multisig` signer creates an ordered, `n` of `n` multisig.
            Each of the specified signers can be any other supported signer type, and will be prompted to sign in the appropriate order."#},
            inputs: [
                signers: {
                    documentation: "A list of signers that make up the multisig.",
                    typing: Type::array(Type::string()),
                    optional: false,
                    tainting: true,
                    sensitive: false
                },
                expected_address: {
                    documentation: "The multisig address that is expected to be created from combining the public keys of all parties. Omitting this field will allow any address to be used for this signer.",
                    typing: Type::string(),
                    optional: true,
                    tainting: true,
                    sensitive: false
                },
                required_signatures: {
                    documentation: "The number of signatures required. This value must be between 1 and the number of signers. If this value is equal to the number of signers, an `n` of `n` multisig address is generated. If this value is less than the number of signers, an `m` of `n` multisig address is generated. If omitted, the number of signers will be used.",
                    typing: Type::integer(),
                    optional: true,
                    tainting: true,
                    sensitive: false
                }
            ],
            outputs: [
                signers: {
                    documentation: "The list of signers that make up the multisig.",
                    typing: Type::array(Type::string())
                },
                address: {
                    documentation: "The address of the account generated from the public key.",
                    typing: Type::array(Type::string())
                }
            ],
            example: txtx_addon_kit::indoc! {r#"
                signer "alice" "stacks::web_wallet" {
                    expected_address = "ST2JHG361ZXG51QTKY2NQCVBPPRRE2KZB1HR05NNC"
                }

                signer "bob" "stacks::web_wallet" {
                    expected_address = "ST2NEB84ASENDXKYGJPQW86YXQCEFEX2ZQPG87ND"
                }

                signer "alice_and_bob" "stacks::multisig" {
                    signers = [signer.alice, signer.bob]
                }
            "#}
        }
    };
}

pub struct StacksConnect;
impl SignerImplementation for StacksConnect {
    fn check_instantiability(
        _ctx: &SignerSpecification,
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
        _spec: &SignerSpecification,
        values: &ValueStore,
        mut signer_state: ValueStore,
        mut signers: SignersState,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        supervision_context: &RunbookSupervisionContext,
        auth_ctx: &txtx_addon_kit::types::AuthorizationContext,
        is_balance_check_required: bool,
        _is_public_key_required: bool,
    ) -> SignerActionsFutureResult {
        use txtx_addon_kit::types::frontend::ReviewInputRequest;

        use crate::constants::RPC_API_AUTH_TOKEN;

        let root_construct_did = construct_did.clone();
        let multisig_signer_instances = get_multisig_signer_instances(values, signers_instances);

        let values = values.clone();
        let signers_instances = signers_instances.clone();
        let supervision_context = supervision_context.clone();
        let auth_ctx = auth_ctx.clone();
        let instance_name = instance_name.to_string();
        let expected_address: Option<String> = None;

        let signer_count = multisig_signer_instances.len() as u16;
        let required_signature_count: u16 = values
            .get_uint("required_signatures")
            .unwrap()
            .and_then(|count| Some(count.try_into().unwrap_or(signer_count).max(1)))
            .unwrap_or(signer_count);

        signer_state
            .insert(REQUIRED_SIGNATURE_COUNT, Value::integer(required_signature_count as i128));

        let instance_name = instance_name.clone();
        let future = async move {
            let rpc_api_url = values
                .get_expected_string(RPC_API_URL)
                .map_err(|e| (signers.clone(), signer_state.clone(), e))?;

            let rpc_api_auth_token =
                values.get_string(RPC_API_AUTH_TOKEN).and_then(|t| Some(t.to_owned()));

            let network_id = values
                .get_expected_string(NETWORK_ID)
                .map_err(|e| (signers.clone(), signer_state.clone(), e))?;

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
                ActionItemKey::ProvidePublicKey,
            )];
            let ref mut res = get_addition_actions_for_address(
                &expected_address,
                &root_construct_did,
                &instance_name,
                &network_id,
                &rpc_api_url,
                &rpc_api_auth_token,
                false,
                is_balance_check_required,
                false,
            )
            .await
            .map_err(|e| (signers.clone(), signer_state.clone(), e))?;

            open_modal_action.append(res);

            consolidated_actions.push_sub_group(None, open_modal_action);
            consolidated_actions.push_modal(modal);

            // Modal configuration
            let mut checked_public_keys = HashMap::new();
            for (signer_did, signer_instance) in multisig_signer_instances.iter() {
                let signer_signer_state = signers.pop_signer_state(&signer_did).unwrap();
                let future = (signer_instance.specification.check_activability)(
                    &signer_did,
                    &signer_instance.name,
                    &signer_instance.specification,
                    &values,
                    signer_signer_state,
                    signers,
                    &signers_instances,
                    &supervision_context,
                    &auth_ctx,
                    false,
                    true,
                )?;
                let (updated_signers, mut actions) = match future.await {
                    Ok(res) => consolidate_signer_result(Ok(res), None).unwrap(),
                    Err(e) => return Err(e),
                };
                signers = updated_signers;
                consolidated_actions.append(&mut actions);

                let signer_signer_state = signers.get_signer_state(&signer_did).unwrap();

                if let Ok(checked_public_key) =
                    signer_signer_state.get_expected_value(CHECKED_PUBLIC_KEY)
                {
                    checked_public_keys.insert(signer_did, checked_public_key.clone());
                }
            }

            if multisig_signer_instances.len() == checked_public_keys.len() {
                let mut ordered_public_keys = vec![];
                let mut ordered_parsed_public_keys = vec![];
                for (signer_uuid, _) in multisig_signer_instances.iter() {
                    if let Some(public_key) = checked_public_keys.remove(signer_uuid) {
                        ordered_public_keys.push(public_key.clone());
                        let bytes = public_key.expect_buffer_bytes();
                        let public_key = Secp256k1PublicKey::from_slice(&bytes).map_err(|e| {
                            (
                                signers.clone(),
                                signer_state.clone(),
                                diagnosed_error!("unable to parse public key {}", e.to_string()),
                            )
                        })?;
                        ordered_parsed_public_keys.push(public_key);
                    }
                }
                signer_state.insert(CHECKED_PUBLIC_KEY, Value::array(ordered_public_keys));

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
                    signer_state.insert(CHECKED_ADDRESS, Value::string(stacks_address.to_string()));

                    let mut actions = Actions::none();
                    if is_balance_check_required {
                        let stacks_rpc = StacksRpc::new(&rpc_api_url, &rpc_api_auth_token);
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
                                ActionItemKey::CheckBalance,
                            )
                            .set_type(ReviewInputRequest::new("", &value).to_action_type())
                            .set_status(status_update),
                        );
                    }
                    actions.push_action_item_update(
                        ActionItemRequestUpdate::from_context(
                            &root_construct_did,
                            ActionItemKey::ProvidePublicKey,
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

            Ok((signers, signer_state, consolidated_actions))
        };
        Ok(Box::pin(future))
    }

    #[cfg(not(feature = "wasm"))]
    fn activate(
        _construct_id: &ConstructDid,
        _spec: &SignerSpecification,
        values: &ValueStore,
        mut signer_state: ValueStore,
        mut signers: SignersState,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        progress_tx: &channel::Sender<BlockEvent>,
    ) -> SignerActivateFutureResult {
        use txtx_addon_kit::constants::SignerKey;

        let values = values.clone();
        let public_key = signer_state
            .get_expected_value(CHECKED_PUBLIC_KEY)
            .map_err(|e| (signers.clone(), signer_state.clone(), e))?
            .to_owned();
        let address = signer_state
            .get_expected_value(CHECKED_ADDRESS)
            .map_err(|e| (signers.clone(), signer_state.clone(), e))?
            .to_owned();
        let network_id = values
            .get_expected_string(NETWORK_ID)
            .map_err(|e| (signers.clone(), signer_state.clone(), e))?
            .to_owned();

        let multisig_signer_instances = get_multisig_signer_instances(&values, signers_instances);

        let signers_instances = signers_instances.clone();
        let progress_tx = progress_tx.clone();

        #[cfg(not(feature = "wasm"))]
        let future = async move {
            let mut result = CommandExecutionResult::new();

            // Modal configuration
            let mut signers_uuids = vec![];
            for (signer_did, signer_instance) in multisig_signer_instances.into_iter() {
                signers_uuids.push(Value::string(signer_did.value().to_string()));
                let signer_signer_state = signers.pop_signer_state(&signer_did).unwrap();
                let future = (signer_instance.specification.activate)(
                    &signer_did,
                    &signer_instance.specification,
                    &values,
                    signer_signer_state,
                    signers,
                    &signers_instances,
                    &progress_tx,
                )?;
                let (updated_signers, _) =
                    consolidate_signer_activate_result(Ok(future.await?), None).unwrap();
                signers = updated_signers;
            }

            signer_state.insert(PUBLIC_KEYS, public_key.clone());

            let version = match network_id.as_str() {
                "mainnet" => AddressHashMode::SerializeP2SH.to_version_mainnet(),
                _ => AddressHashMode::SerializeP2SH.to_version_testnet(),
            };
            signer_state.insert("hash_flag", Value::integer(version.into()));
            signer_state.insert("multi_sig", Value::bool(true));
            signer_state.insert("signers", Value::array(signers_uuids.clone()));

            result.outputs.insert("signers".into(), Value::array(signers_uuids));
            result.outputs.insert(ActionItemKey::ProvidePublicKey.to_string(), public_key.clone());
            result.outputs.insert("address".into(), address.clone());

            Ok((signers, signer_state, result))
        };
        Ok(Box::pin(future))
    }

    fn check_signability(
        origin_uuid: &ConstructDid,
        title: &str,
        description: &Option<String>,
        payload: &Value,
        _spec: &SignerSpecification,
        args: &ValueStore,
        mut signer_state: ValueStore,
        mut signers: SignersState,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        supervision_context: &RunbookSupervisionContext,
    ) -> Result<CheckSignabilityOk, SignerActionErr> {
        let multisig_signer_instances =
            get_multisig_signer_instances(&signer_state, signers_instances);
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
            consolidated_actions.append(&mut Actions::append_item(
                action,
                Some("Review and sign the transactions from the list below"),
                Some("Transaction Signing"),
            ));
            consolidated_actions.push_modal(modal);
        }

        let payload_bytes = payload.expect_buffer_bytes();
        let unsigned_tx =
            StacksTransaction::consensus_deserialize(&mut &payload_bytes[..]).unwrap();
        let (signature_count, actions_count, payloads) = generate_ordered_multisig_payloads(
            &origin_uuid.to_string(),
            unsigned_tx.clone(),
            &multisig_signer_instances,
            &signers,
        )
        .map_err(|e| (signers.clone(), signer_state.clone(), diagnosed_error!("{}", e)))?;

        let required_signature_count =
            signer_state.get_expected_uint(REQUIRED_SIGNATURE_COUNT).unwrap();
        let expected_actions_count = multisig_signer_instances.len() as u64;

        if signature_count >= required_signature_count && actions_count == expected_actions_count {
            let tx = generate_signed_ordered_multisig_tx(
                &origin_uuid.value().to_string(),
                unsigned_tx.clone(),
                &multisig_signer_instances,
                &signers,
                required_signature_count,
            )
            .map_err(|e| (signers.clone(), signer_state.clone(), diagnosed_error!("{}", e)))?;

            tx.verify().map_err(|e| {
                (
                    signers.clone(),
                    signer_state.clone(),
                    diagnosed_error!("multisig generated invalid Stacks transaction: {}", e),
                )
            })?;
            let mut signed_tx_bytes = vec![];
            tx.consensus_serialize(&mut signed_tx_bytes).unwrap();

            signer_state.insert_scoped_value(
                &origin_uuid.value().to_string(),
                SignerKey::SignedTransactionBytes,
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
                &multisig_signer_instances,
                &mut signers,
                required_signature_count,
                signature_count,
            );

            for (signer_uuid, signer_wallet_instance) in multisig_signer_instances.into_iter() {
                let mut signer_wallet_state = signers.pop_signer_state(&signer_uuid).unwrap();
                let transaction = payloads.get(&signer_uuid).unwrap();
                let mut bytes = vec![];
                transaction.consensus_serialize(&mut bytes).unwrap();
                let payload = Value::buffer(bytes);
                let formatted_payload = transaction.format_for_display();

                signer_wallet_state.insert_scoped_value(
                    &origin_uuid.to_string(),
                    FORMATTED_TRANSACTION,
                    formatted_payload,
                );

                let (mut updated_wallets, signer_wallet_state, mut actions) =
                    (signer_wallet_instance.specification.check_signability)(
                        &origin_uuid,
                        &format!("{} - {}", title, signer_wallet_instance.name),
                        description,
                        &payload,
                        &signer_wallet_instance.specification,
                        &args,
                        signer_wallet_state.clone(),
                        signers,
                        &signers_instances,
                        &supervision_context,
                    )?;
                updated_wallets.push_signer_state(signer_wallet_state.clone());
                consolidated_actions.append(&mut actions);
                signers = updated_wallets;
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

        Ok((signers, signer_state, consolidated_actions))
    }

    #[cfg(not(feature = "wasm"))]
    fn sign(
        origin_uuid: &ConstructDid,
        _title: &str,
        payload: &Value,
        _spec: &SignerSpecification,
        values: &ValueStore,
        signer_state: ValueStore,
        mut signers: SignersState,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
    ) -> SignerSignFutureResult {
        use txtx_addon_kit::{constants::SIGNED_MESSAGE_BYTES, futures::future};

        use crate::typing::StacksValue;

        if let Some(signed_transaction_bytes) = signer_state
            .get_scoped_value(&origin_uuid.value().to_string(), SignerKey::SignedTransactionBytes)
        {
            let mut result = CommandExecutionResult::new();
            result
                .outputs
                .insert(SignerKey::SignedTransactionBytes, signed_transaction_bytes.clone());

            return Ok(Box::pin(future::ready(Ok((signers, signer_state, result)))));
        }

        let multisig_signer_instances =
            get_multisig_signer_instances(&signer_state, signers_instances);
        let args = values.clone();
        let signers_instances = signers_instances.clone();

        let payload = payload.clone();

        let future = async move {
            let mut result = CommandExecutionResult::new();

            let transaction_payload_bytes = payload.expect_buffer_bytes();
            let mut transaction =
                StacksTransaction::consensus_deserialize(&mut &transaction_payload_bytes[..])
                    .unwrap();
            let mut presign_input = transaction.sign_begin();

            for (signer_did, signer_instance) in multisig_signer_instances.into_iter() {
                let signer_state = signers.pop_signer_state(&signer_did).unwrap();

                let payload = StacksValue::signature(
                    TransactionSpendingCondition::make_sighash_presign(
                        &presign_input,
                        &TransactionAuthFlags::AuthStandard,
                        transaction.get_tx_fee(),
                        transaction.get_origin_nonce(),
                    )
                    .to_bytes()
                    .to_vec(),
                );

                let future = (signer_instance.specification.sign)(
                    &signer_did,
                    &signer_instance.name,
                    &payload,
                    &signer_instance.specification,
                    &args,
                    signer_state,
                    signers,
                    &signers_instances,
                )?;

                let (updated_signers, updated_results) =
                    consolidate_signer_activate_result(Ok(future.await?), None).unwrap();
                signers = updated_signers;
                let updated_message = updated_results.outputs.get(MESSAGE_BYTES).unwrap().clone();
                let signature = updated_results.outputs.get(SIGNED_MESSAGE_BYTES).unwrap().clone();

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

            result.outputs.insert(SignerKey::SignedTransactionBytes.to_string(), transaction_bytes);

            Ok((signers, signer_state, result))
        };
        Ok(Box::pin(future))
    }
}

fn get_multisig_signer_instances(
    args: &ValueStore,
    signers_instances: &HashMap<ConstructDid, SignerInstance>,
) -> Vec<(ConstructDid, SignerInstance)> {
    let signers_uuid = args.get_expected_array("signers").unwrap();
    let mut signers = Vec::new();
    for signer_uuid in signers_uuid.iter() {
        let uuid = signer_uuid.as_string().unwrap();
        let uuid = ConstructDid(Did::from_hex_string(uuid));
        let signer_instance = signers_instances.get(&uuid).unwrap().clone();
        signers.push((uuid, signer_instance));
    }
    signers
}

/// Takes an unsigned [StacksTransaction], a set of signers, and a [SignersState] for each signer
/// that _could_ contain a signature, and generates a next payload for each signer.
///
/// Each signer's payload will be identical to the original transaction, except the multisig auth's
/// `fields` will be updated.
/// Here is an example set of fields for three signers across multiple states:
/// ```ignore
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
    multisig_signer_instances: &Vec<(ConstructDid, SignerInstance)>,
    signers: &SignersState,
) -> Result<(u64, u64, HashMap<ConstructDid, StacksTransaction>), String> {
    let mut payloads = HashMap::new();
    let mut signature_count = 0;
    let mut actions_count = 0;

    // Loop over each signer to compute what their auth's `fields` should be
    for (this_signer_idx, (this_signer_uuid, _)) in multisig_signer_instances.iter().enumerate() {
        let this_signer_state = signers.get_signer_state(&this_signer_uuid).unwrap();

        let stored_signature =
            this_signer_state.get_scoped_value(origin_uuid, SignerKey::SignedTransactionBytes);
        // along the way, track how many signers have completed signatures
        signature_count += stored_signature
            // if we have a signature for this signer, check if it's null. if null, this signer was skipped so don't add it to our count
            .map(|v| v.as_null().and_then(|_| Some(0)).unwrap_or(1))
            .unwrap_or(0);
        // track all actions, regardless of if it was a skip or a signature
        actions_count += if stored_signature.is_some() { 1 } else { 0 };

        // this signer's fields depend on the previous signers in the order, so we look back through the signers
        // note: the first signer can't look back and will thus have empty `fields`
        let mut this_signer_fields = VecDeque::new();
        if this_signer_idx > 0 {
            let mut idx = this_signer_idx - 1;

            while let Some((previous_signer_uuid, instance_of_previous_signer)) =
                multisig_signer_instances.get(idx)
            {
                let state_of_previous_signer =
                    signers.get_signer_state(&previous_signer_uuid).unwrap();

                // check if the previous signer has provided a signature.
                // if so, we need to push on the signature portion of the previous signer's auth `field`.
                // if not, we should use the previous signer's public key
                let field = extract_auth_field_from_signer_state(
                    state_of_previous_signer,
                    idx,
                    origin_uuid,
                )
                .map_err(|e| {
                    format!(
                        "error with multisig signer {}: {}",
                        instance_of_previous_signer.name, e
                    )
                })?;

                this_signer_fields.push_front(field);
                if idx == 0 {
                    break;
                }
                idx -= 1;
            }
        }
        let fields = this_signer_fields.drain(..).collect::<Vec<TransactionAuthField>>();

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
        payloads.insert(this_signer_uuid.clone(), tx);
    }

    Ok((signature_count, actions_count, payloads))
}

/// Takes an unsigned [StacksTransaction], a set of signers , and a [SignersState] for each signer
/// that _could_ contain a signature, and generates a signed [StacksTransaction] containing each
/// signer's signature (if available) or public key.
///
/// Building off the example from [generate_ordered_multisig_payloads], the fields for the signed tx would be:
/// ```ignore
/// [alice_pubkey, bob_signature, charlie_signature]
/// ```
fn generate_signed_ordered_multisig_tx(
    origin_uuid: &str,
    mut tx: StacksTransaction,
    multisig_signer_instances: &Vec<(ConstructDid, SignerInstance)>,
    signers: &SignersState,
    required_signature_count: u64,
) -> Result<StacksTransaction, String> {
    let mut fields = vec![];
    for (signer_idx, (signer_uuid, signer_instance)) in multisig_signer_instances.iter().enumerate()
    {
        let signer_state = signers.get_signer_state(&signer_uuid).unwrap();

        let field = extract_auth_field_from_signer_state(signer_state, signer_idx, origin_uuid)
            .map_err(|e| format!("error with multisig signer {}: {}", signer_instance.name, e))?;

        fields.push(field);
    }

    let TransactionAuth::Standard(TransactionSpendingCondition::Multisig(mut spending_condition)) =
        tx.auth
    else {
        return Err("expected multisig spending condition for multisig transaction".into());
    };
    spending_condition.fields = fields;
    spending_condition.signatures_required = required_signature_count.try_into().unwrap();
    tx.auth = TransactionAuth::Standard(TransactionSpendingCondition::Multisig(spending_condition));
    Ok(tx)
}

fn extract_auth_field_from_signer_state(
    signer_state: &ValueStore,
    multisig_signer_idx: usize,
    origin_uuid: &str,
) -> Result<TransactionAuthField, String> {
    let field = match signer_state.get_scoped_value(origin_uuid, SignerKey::SignedTransactionBytes) {
        Some(&Value::Null) | None => {
            let stacks_public_key = expect_stacks_public_key(signer_state, CHECKED_PUBLIC_KEY)?;
            TransactionAuthField::PublicKey(stacks_public_key)
        }
        Some(signed_tx_bytes) => {
            // we're expecting the auth value to contain a multisig spending condition
            // note: if this poses a problem, we may want to just default to providing
            // the public key
            extract_auth_field_from_signed_tx_bytes(&signed_tx_bytes, multisig_signer_idx)?
                .ok_or(format!("missing expected auth field"))?
        }
    };
    Ok(field)
}

fn extract_auth_field_from_signed_tx_bytes(
    signed_tx_bytes: &Value,
    index: usize,
) -> Result<Option<TransactionAuthField>, String> {
    let bytes = signed_tx_bytes.expect_buffer_bytes();
    let signed_tx = StacksTransaction::consensus_deserialize(&mut &bytes[..])
        .map_err(|e| format!("signed stacks transaction is invalid: {e}"))?;
    let field = match signed_tx.auth {
        TransactionAuth::Standard(TransactionSpendingCondition::Multisig(spending_condition)) => {
            let Some(auth_field) = spending_condition.fields.get(index) else {
                return Ok(None);
            };
            Some(auth_field.clone())
        }
        _ => None,
    };
    Ok(field)
}

fn set_signer_states(
    origin_uuid: &str,
    multisig_signer_instances: &Vec<(ConstructDid, SignerInstance)>,
    signers: &mut SignersState,
    required_signatures: u64,
    signature_count: u64,
) {
    let signer_count = multisig_signer_instances.len() as u64;
    let remaining_signatures_required = required_signatures - signature_count;

    let mut previous_signer_action_completed = true;
    for (signer_idx, (signer_uuid, _inst)) in multisig_signer_instances.iter().enumerate() {
        let mut signing_command_state = signers.pop_signer_state(&signer_uuid).unwrap();

        // if this signer has a signature stored and it is null, the user skipped this signature
        let this_signer_skipped = signing_command_state
            .get_scoped_value(origin_uuid, SignerKey::SignedTransactionBytes)
            .and_then(|v| Some(v.as_null().is_some()))
            .unwrap_or(false);
        // if this signer has a signature stored and it _isn't_ null, we have a real signature and weren't skipped
        let this_signer_signed = signing_command_state
            .get_scoped_value(origin_uuid, SignerKey::SignedTransactionBytes)
            .and_then(|v| Some(v.as_null().is_none()))
            .unwrap_or(false);

        // if the signer has already signed, they can't sign again and are not skippable
        if this_signer_signed || this_signer_skipped {
            signing_command_state.insert_scoped_value(
                &origin_uuid,
                SignerKey::SignatureSkippable,
                Value::bool(false),
            );

            signing_command_state.insert_scoped_value(
                &origin_uuid,
                IS_SIGNABLE,
                Value::bool(false),
            );
        } else {
            // if this signer is skipped, will the remaining number of possible signers be enough to reach required signatures?
            // if so, this one is safe to skip. if not, not skippable
            let next_signer_idx = (signer_idx + 1) as u64;
            let eligible_signers_after_this_signer = signer_count - next_signer_idx;
            signing_command_state.insert_scoped_value(
                &origin_uuid,
                SignerKey::SignatureSkippable,
                Value::bool(
                    previous_signer_action_completed
                        && (eligible_signers_after_this_signer >= remaining_signatures_required),
                ),
            );

            // only signable if our previous signer has taken action (skip or sign), and we still need signatures
            signing_command_state.insert_scoped_value(
                &origin_uuid,
                IS_SIGNABLE,
                Value::bool(previous_signer_action_completed && remaining_signatures_required > 0),
            );
        }
        previous_signer_action_completed = this_signer_signed || this_signer_skipped;

        signers.push_signer_state(signing_command_state);
    }
}
