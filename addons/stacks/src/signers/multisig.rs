use clarity::address::AddressHashMode;
use clarity::types::chainstate::StacksAddress;
use clarity::util::secp256k1::Secp256k1PublicKey;
use clarity::{codec::StacksMessageCodec, util::secp256k1::MessageSignature};
use std::collections::HashMap;
use txtx_addon_kit::types::commands::{CommandExecutionResult, CommandSpecification};
use txtx_addon_kit::types::types::RunbookSupervisionContext;

use crate::typing::StacksValue;
use crate::{
    codec::codec::{
        StacksTransaction, TransactionAuth, TransactionAuthField, TransactionAuthFlags,
        TransactionPublicKeyEncoding, TransactionSpendingCondition, Txid,
    },
    constants::{MESSAGE_BYTES, SIGNED_MESSAGE_BYTES},
};
use txtx_addon_kit::types::frontend::{
    ActionItemRequest, ActionItemRequestType, ActionItemRequestUpdate, ActionItemStatus, Actions,
    BlockEvent, OpenModalData,
};
use txtx_addon_kit::types::signers::{
    consolidate_signer_activate_result, consolidate_signer_result, CheckSignabilityOk,
    SignerActionErr, SignerActionsFutureResult, SignerActivateFutureResult, SignerImplementation,
    SignerInstance, SignerSignFutureResult, SignerSpecification, SignersState,
};
use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{ConstructDid, Did, ValueStore};
use txtx_addon_kit::{channel, AddonDefaults};

use crate::constants::{
    ACTION_ITEM_CHECK_BALANCE, ACTION_ITEM_PROVIDE_PUBLIC_KEY, ACTION_OPEN_MODAL,
    CHECKED_PUBLIC_KEY, NETWORK_ID, PUBLIC_KEYS, RPC_API_URL, SIGNED_TRANSACTION_BYTES,
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
                interpolable: true,
                sensitive: false
            },
            expected_address: {
              documentation: "The multisig address that is expected to be created from combining the public keys of all parties. Omitting this field will allow any address to be used for this signer.",
                typing: Type::string(),
                optional: true,
                interpolable: true,
                sensitive: false
            }
          ],
          outputs: [
              public_key: {
                documentation: "The public key of the generated multisig signer.",
                typing: Type::array(Type::buffer())
              },
              signers: {
                documentation: "The list of signers that make up the multisig.",
                typing: Type::array(Type::string())
              }
          ],
          example: txtx_addon_kit::indoc! {r#"
            signer "alice" "stacks::connect" {
                expected_address = "ST2JHG361ZXG51QTKY2NQCVBPPRRE2KZB1HR05NNC"
            }

            signer "bob" "stacks::connect" {
                expected_address = "ST2NEB84ASENDXKYGJPQW86YXQCEFEX2ZQPG87ND"
            }

            signer "alice_and_bob" "stacks::multisig" {
                signers = [signer.alice, signer.bob]
            }
    "#},
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
        args: &ValueStore,
        mut signer_state: ValueStore,
        mut signers: SignersState,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        defaults: &AddonDefaults,
        supervision_context: &RunbookSupervisionContext,
        is_balance_check_required: bool,
        _is_public_key_required: bool,
    ) -> SignerActionsFutureResult {
        use txtx_addon_kit::types::frontend::ReviewInputRequest;

        use crate::constants::RPC_API_AUTH_TOKEN;

        let root_construct_did = construct_did.clone();
        let signers_instantiated = get_signers(args, signers_instances);

        let args = args.clone();
        let signers_instances = signers_instances.clone();
        let defaults = defaults.clone();
        let supervision_context = supervision_context.clone();
        let instance_name = instance_name.to_string();
        let expected_address: Option<String> = None;
        let rpc_api_url = match args.get_defaulting_string(RPC_API_URL, &defaults) {
            Ok(value) => value,
            Err(diag) => return Err((signers, signer_state, diag)),
        };
        let rpc_api_auth_token = args.get_defaulting_string(RPC_API_AUTH_TOKEN, &defaults).ok();
        let network_id = match args.get_defaulting_string(NETWORK_ID, &defaults) {
            Ok(value) => value,
            Err(diag) => return Err((signers, signer_state, diag)),
        };

        let future = async move {
            let mut consolidated_actions = Actions::none();

            // Modal configuration
            let modal =
                BlockEvent::new_modal("Stacks Multisig Configuration assistant", "", vec![]);
            let mut open_modal_action = vec![ActionItemRequest::new(
                &Some(root_construct_did.clone()),
                "Compute multisig address",
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
                &rpc_api_auth_token,
                false,
                is_balance_check_required,
                false,
            )
            .await;
            match additional_actions_res {
                Ok(ref mut res) => {
                    open_modal_action.append(res);
                }
                Err(diag) => return Err((signers, signer_state, diag)),
            }

            consolidated_actions.push_sub_group(None, open_modal_action);
            consolidated_actions.push_modal(modal);

            // Modal configuration
            let mut checked_public_keys = HashMap::new();
            for (signer_did, signer_instance) in signers_instantiated.iter() {
                let signer_signer_state = signers.pop_signer_state(&signer_did).unwrap();
                let future = (signer_instance.specification.check_activability)(
                    &signer_did,
                    &signer_instance.name,
                    &signer_instance.specification,
                    &args,
                    signer_signer_state,
                    signers,
                    &signers_instances,
                    &defaults,
                    &supervision_context,
                    false,
                    true,
                )?;
                let (updated_signers, mut actions) = match future.await {
                    Ok(res) => consolidate_signer_result(Ok(res)).unwrap(),
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

            if signers_instantiated.len() == checked_public_keys.len() {
                let mut ordered_public_keys = vec![];
                let mut ordered_parsed_public_keys = vec![];
                for (signer_uuid, _) in signers_instantiated.iter() {
                    if let Some(public_key) = checked_public_keys.remove(signer_uuid) {
                        ordered_public_keys.push(public_key.clone());
                        let bytes = public_key.expect_buffer_bytes();
                        let public_key = match Secp256k1PublicKey::from_slice(&bytes) {
                            Ok(public_key) => public_key,
                            Err(e) => {
                                return Err((
                                    signers,
                                    signer_state,
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
                signer_state.insert(CHECKED_PUBLIC_KEY, Value::array(ordered_public_keys));

                let version = if network_id.eq("mainnet") {
                    clarity_repl::clarity::address::C32_ADDRESS_VERSION_MAINNET_MULTISIG
                } else {
                    clarity_repl::clarity::address::C32_ADDRESS_VERSION_TESTNET_MULTISIG
                };

                if let Some(stacks_address) = StacksAddress::from_public_keys(
                    version,
                    &AddressHashMode::SerializeP2SH,
                    ordered_parsed_public_keys.len(),
                    &ordered_parsed_public_keys,
                )
                .map(|address| address.to_string())
                {
                    let mut actions = Actions::none();
                    if is_balance_check_required {
                        let stacks_rpc = StacksRpc::new(&rpc_api_url, rpc_api_auth_token);
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

            Ok((signers, signer_state, consolidated_actions))
        };
        Ok(Box::pin(future))
    }

    fn activate(
        _construct_id: &ConstructDid,
        _spec: &SignerSpecification,
        args: &ValueStore,
        mut signer_state: ValueStore,
        mut signers: SignersState,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        defaults: &AddonDefaults,
        progress_tx: &channel::Sender<BlockEvent>,
    ) -> SignerActivateFutureResult {
        let public_key = match signer_state.get_expected_value(CHECKED_PUBLIC_KEY) {
            Ok(value) => value.clone(),
            Err(diag) => return Err((signers, signer_state, diag)),
        };
        let network_id = match args.get_defaulting_string(NETWORK_ID, defaults) {
            Ok(value) => value,
            Err(diag) => return Err((signers, signer_state, diag)),
        };

        let signers_instantiated = get_signers(args, signers_instances);

        let args = args.clone();
        let signers_instances = signers_instances.clone();
        let defaults = defaults.clone();
        let progress_tx = progress_tx.clone();

        #[cfg(not(feature = "wasm"))]
        let future = async move {
            let mut result = CommandExecutionResult::new();

            // Modal configuration
            let mut signers_uuids = vec![];
            for (signer_did, signer_instance) in signers_instantiated.into_iter() {
                signers_uuids.push(Value::string(signer_did.value().to_string()));
                let signer_signer_state = signers.pop_signer_state(&signer_did).unwrap();
                let future = (signer_instance.specification.activate)(
                    &signer_did,
                    &signer_instance.specification,
                    &args,
                    signer_signer_state,
                    signers,
                    &signers_instances,
                    &defaults,
                    &progress_tx,
                )?;
                let (updated_signers, _) =
                    consolidate_signer_activate_result(Ok(future.await?)).unwrap();
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
            result.outputs.insert("public_key".into(), public_key);

            Ok((signers, signer_state, result))
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
        _spec: &SignerSpecification,
        args: &ValueStore,
        mut signer_state: ValueStore,
        mut signers: SignersState,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        defaults: &AddonDefaults,
        supervision_context: &RunbookSupervisionContext,
    ) -> Result<CheckSignabilityOk, SignerActionErr> {
        // let mut transaction_bytes_to_sign = payload.clone();
        let signers_instantiated = get_signers(&signer_state, signers_instances);
        // let (tx_cursor_key, mut cursor) = get_current_tx_cursor_key(origin_uuid, &signer_state);
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
            consolidated_actions.push_sub_group(None, open_modal_action);
            consolidated_actions.push_modal(modal);
        }

        let mut payload = args
            .get_expected_buffer_bytes(SIGNED_TRANSACTION_BYTES)
            .ok()
            .and_then(|buff| Some(Value::buffer(buff)))
            .unwrap_or(payload.clone());
        let mut all_signed = true;
        for (signer_uuid, signer_signer_instance) in signers_instantiated.into_iter() {
            let signer_signer_state = signers.pop_signer_state(&signer_uuid).unwrap();

            let (mut updated_signers, signer_signer_state, mut actions) =
                (signer_signer_instance.specification.check_signability)(
                    &origin_uuid,
                    &format!("{} - {}", title, signer_signer_instance.name),
                    description,
                    &payload,
                    &signer_signer_instance.specification,
                    &args,
                    signer_signer_state.clone(),
                    signers,
                    &signers_instances,
                    &defaults,
                    &supervision_context,
                )?;
            updated_signers.push_signer_state(signer_signer_state.clone());
            if actions.has_pending_actions() {
                payload = Value::null();
                all_signed = false;
            }
            consolidated_actions.append(&mut actions);
            signers = updated_signers;
        }

        if all_signed {
            let signed_buff = args.get_expected_buffer_bytes(SIGNED_TRANSACTION_BYTES).unwrap();
            let transaction =
                StacksTransaction::consensus_deserialize(&mut &signed_buff[..]).unwrap();
            transaction.verify().unwrap();

            signer_state.insert_scoped_value(
                &origin_uuid.value().to_string(),
                SIGNED_TRANSACTION_BYTES,
                Value::string(txtx_addon_kit::hex::encode(signed_buff)),
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
        _origin_uuid: &ConstructDid,
        _title: &str,
        payload: &Value,
        _spec: &SignerSpecification,
        args: &ValueStore,
        signer_state: ValueStore,
        mut signers: SignersState,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        defaults: &AddonDefaults,
    ) -> SignerSignFutureResult {
        use txtx_addon_kit::futures::future;

        if let Some(signed_transaction_bytes) = signer_state.get_value(SIGNED_TRANSACTION_BYTES) {
            let mut result = CommandExecutionResult::new();
            result
                .outputs
                .insert(SIGNED_TRANSACTION_BYTES.into(), signed_transaction_bytes.clone());

            return Ok(Box::pin(future::ready(Ok((signers, signer_state, result)))));
        }
        let signers_instantiated = get_signers(&signer_state, signers_instances);
        let args = args.clone();
        let signers_instances = signers_instances.clone();
        let defaults = defaults.clone();

        let payload = payload.clone();

        let future = async move {
            let mut result = CommandExecutionResult::new();

            let transaction_payload_bytes = payload.expect_buffer_bytes();
            let mut transaction =
                StacksTransaction::consensus_deserialize(&mut &transaction_payload_bytes[..])
                    .unwrap();
            let mut presign_input = transaction.sign_begin();

            for (signer_did, signer_instance) in signers_instantiated.into_iter() {
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
                    &defaults,
                )?;

                let (updated_signers, updated_results) =
                    consolidate_signer_activate_result(Ok(future.await?)).unwrap();
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

            result.outputs.insert(SIGNED_TRANSACTION_BYTES.into(), transaction_bytes);

            Ok((signers, signer_state, result))
        };
        Ok(Box::pin(future))
    }
}

fn get_signers(
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
