use std::collections::HashMap;
use std::str::FromStr;

use clarity::address::AddressHashMode;
use clarity::types::chainstate::StacksAddress;
use clarity::util::secp256k1::Secp256k1PublicKey;
use clarity::{codec::StacksMessageCodec, util::secp256k1::MessageSignature};
use txtx_addon_kit::types::commands::{
    CommandExecutionContext, CommandExecutionResult, CommandSpecification,
};

use crate::{
    codec::codec::{
        StacksTransaction, TransactionAuth, TransactionAuthField, TransactionAuthFlags,
        TransactionPublicKeyEncoding, TransactionSpendingCondition, Txid,
    },
    constants::{MESSAGE_BYTES, SIGNATURE_BYTES},
    typing::STACKS_SIGNATURE,
};
use txtx_addon_kit::types::frontend::{
    ActionItemRequest, ActionItemRequestType, ActionItemRequestUpdate, ActionItemStatus, Actions,
    BlockEvent, OpenModalData, ProvideSignedTransactionRequest,
};
use txtx_addon_kit::types::wallets::{
    consolidate_wallet_activate_result, consolidate_wallet_result, CheckSignabilityOk,
    WalletActionErr, WalletActionsFutureResult, WalletActivateFutureResult, WalletImplementation,
    WalletInstance, WalletSignFutureResult, WalletSpecification, WalletsState,
};
use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{ConstructUuid, ValueStore};
use txtx_addon_kit::uuid::Uuid;
use txtx_addon_kit::{channel, AddonDefaults};

use crate::constants::{
    ACTION_ITEM_CHECK_BALANCE, ACTION_ITEM_PROVIDE_PUBLIC_KEY,
    ACTION_ITEM_PROVIDE_SIGNED_TRANSACTION, CHECKED_PUBLIC_KEY, NETWORK_ID, PUBLIC_KEYS,
    RPC_API_URL, SIGNED_TRANSACTION_BYTES,
};
use crate::rpc::StacksRpc;

use super::get_addition_actions_for_address;

lazy_static! {
    pub static ref STACKS_MULTISIG: WalletSpecification = define_wallet! {
        StacksConnect => {
          name: "Stacks Multisig",
          matcher: "multisig",
          documentation: "Coming soon",
          inputs: [
            signers: {
              documentation: "Coming soon",
                typing: Type::array(Type::string()),
                optional: false,
                interpolable: true
            },
            exepcted_address: {
              documentation: "Coming soon",
                typing: Type::string(),
                optional: true,
                interpolable: true
            },
            exepcted_public_key: {
                documentation: "Coming soon",
                  typing: Type::string(),
                  optional: true,
                  interpolable: true
              }
          ],
          outputs: [
              public_key: {
                documentation: "Coming soon",
                typing: Type::array(Type::buffer())
              },
              signers: {
                documentation: "Coming soon",
                typing: Type::array(Type::string())
              }
          ],
          example: txtx_addon_kit::indoc! {r#"
        // Coming soon
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
        uuid: &ConstructUuid,
        instance_name: &str,
        _spec: &WalletSpecification,
        args: &ValueStore,
        mut wallet_state: ValueStore,
        mut wallets: WalletsState,
        wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        defaults: &AddonDefaults,
        execution_context: &CommandExecutionContext,
        is_balance_check_required: bool,
        _is_public_key_required: bool,
    ) -> WalletActionsFutureResult {
        use txtx_addon_kit::types::{
            frontend::ReviewInputRequest, wallets::consolidate_wallet_future_result,
        };

        let root_uuid = uuid.clone();
        let signers = get_signers(args, wallets_instances);

        let args = args.clone();
        let wallets_instances = wallets_instances.clone();
        let defaults = defaults.clone();
        let execution_context = execution_context.clone();
        let instance_name = instance_name.to_string();
        let expected_address: Option<String> = None;
        let rpc_api_url = match args.get_defaulting_string(RPC_API_URL, &defaults) {
            Ok(value) => value,
            Err(diag) => return Err((wallets, wallet_state, diag)),
        };
        let network_id = match args.get_defaulting_string(NETWORK_ID, &defaults) {
            Ok(value) => value,
            Err(diag) => return Err((wallets, wallet_state, diag)),
        };

        let future = async move {
            let mut consolidated_actions = Actions::none();

            // Modal configuration
            let modal =
                BlockEvent::new_modal("Stacks Multisig Configuration assistant", "", vec![]);
            let mut open_modal_action = vec![ActionItemRequest::new(
                &Uuid::new_v4(),
                &Some(root_uuid.value()),
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
                &root_uuid,
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
                Err(diag) => return Err((wallets, wallet_state, diag)),
            }

            consolidated_actions.push_sub_group(open_modal_action);
            consolidated_actions.push_modal(modal);

            // Modal configuration
            let mut checked_public_keys = HashMap::new();
            for (wallet_uuid, wallet_instance) in signers.iter() {
                let signer_wallet_state = wallets.pop_wallet_state(&wallet_uuid).unwrap();
                let future = (wallet_instance.specification.check_activability)(
                    &wallet_uuid,
                    &wallet_instance.name,
                    &wallet_instance.specification,
                    &args,
                    signer_wallet_state,
                    wallets,
                    &wallets_instances,
                    &defaults,
                    &execution_context,
                    false,
                    true,
                )?;
                let (updated_wallets, mut actions) = match future.await {
                    Ok(res) => consolidate_wallet_result(Ok(res)).unwrap(),
                    Err(e) => return Err(e),
                };
                wallets = updated_wallets;
                consolidated_actions.append(&mut actions);

                let signer_wallet_state = wallets.get_wallet_state(&wallet_uuid).unwrap();

                if let Ok(checked_public_key) =
                    signer_wallet_state.get_expected_value(CHECKED_PUBLIC_KEY)
                {
                    checked_public_keys.insert(wallet_uuid, checked_public_key.clone());
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
                                    wallet_state,
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
                wallet_state.insert(CHECKED_PUBLIC_KEY, Value::array(ordered_public_keys));

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
                    let stacks_rpc = StacksRpc::new(&rpc_api_url);
                    let status_update = match stacks_rpc.get_balance(&stacks_address).await {
                        Ok(response) => {
                            ActionItemStatus::Success(Some(response.get_formatted_balance()))
                        }
                        Err(e) => {
                            let diag = diagnosed_error!(
                                "unable to retrieve balance {}: {}",
                                stacks_address,
                                e.to_string()
                            );
                            ActionItemStatus::Error(diag)
                        }
                    };
                    actions.push_action_item_update(
                        ActionItemRequestUpdate::from_context(
                            &root_uuid,
                            ACTION_ITEM_CHECK_BALANCE,
                        )
                        .set_status(status_update),
                    );
                    consolidated_actions = actions;
                } else {
                    println!("Unable to compute Stacks address");
                }
            } else {
                let validate_modal_action = ActionItemRequest::new(
                    &Uuid::new_v4(),
                    &Some(root_uuid.value()),
                    "CONFIRM",
                    None,
                    ActionItemStatus::Todo,
                    ActionItemRequestType::ValidateModal,
                    "modal",
                );
                consolidated_actions.push_group("", vec![validate_modal_action]);
            }

            Ok((wallets, wallet_state, consolidated_actions))
        };
        Ok(Box::pin(future))
    }

    fn activate(
        _uuid: &ConstructUuid,
        _spec: &WalletSpecification,
        args: &ValueStore,
        mut wallet_state: ValueStore,
        mut wallets: WalletsState,
        wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        defaults: &AddonDefaults,
        progress_tx: &channel::Sender<BlockEvent>,
    ) -> WalletActivateFutureResult {
        let public_key = match wallet_state.get_expected_value(CHECKED_PUBLIC_KEY) {
            Ok(value) => value.clone(),
            Err(diag) => return Err((wallets, wallet_state, diag)),
        };
        let network_id = match args.get_defaulting_string(NETWORK_ID, defaults) {
            Ok(value) => value,
            Err(diag) => return Err((wallets, wallet_state, diag)),
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
            for (wallet_uuid, wallet_instance) in signers.into_iter() {
                signers_uuids.push(Value::string(wallet_uuid.value().to_string()));
                let signer_wallet_state = wallets.pop_wallet_state(&wallet_uuid).unwrap();
                let future = (wallet_instance.specification.activate)(
                    &wallet_uuid,
                    &wallet_instance.specification,
                    &args,
                    signer_wallet_state,
                    wallets,
                    &wallets_instances,
                    &defaults,
                    &progress_tx,
                )?;
                let (updated_wallets, _) =
                    consolidate_wallet_activate_result(Ok(future.await?)).unwrap();
                wallets = updated_wallets;
            }

            wallet_state.insert(PUBLIC_KEYS, public_key.clone());

            let version = match network_id.as_str() {
                "mainnet" => AddressHashMode::SerializeP2SH.to_version_mainnet(),
                _ => AddressHashMode::SerializeP2SH.to_version_testnet(),
            };
            wallet_state.insert("hash_flag", Value::uint(version.into()));
            wallet_state.insert("multi_sig", Value::bool(true));
            wallet_state.insert("signers", Value::array(signers_uuids.clone()));

            result
                .outputs
                .insert("signers".into(), Value::array(signers_uuids));
            result.outputs.insert("public_key".into(), public_key);

            Ok((wallets, wallet_state, result))
        };
        #[cfg(feature = "wasm")]
        panic!("async commands are not enabled for wasm");
        #[cfg(not(feature = "wasm"))]
        Ok(Box::pin(future))
    }

    fn check_signability(
        origin_uuid: &ConstructUuid,
        title: &str,
        description: &Option<String>,
        payload: &Value,
        _spec: &WalletSpecification,
        args: &ValueStore,
        mut wallet_state: ValueStore,
        mut wallets: WalletsState,
        wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        defaults: &AddonDefaults,
        execution_context: &CommandExecutionContext,
    ) -> Result<CheckSignabilityOk, WalletActionErr> {
            let signer_wallet_state = wallets.pop_wallet_state(&signer_uuid).unwrap();
            let (mut updated_wallets, signer_wallet_state, mut actions) =
                (signer_wallet_instance.specification.check_signability)(
                    &signer_uuid,
                    &format!("{} - {}", title, signer_wallet_instance.name),
                    description,
                    &payload,
                    &signer_wallet_instance.specification,
                    &args,
                    signer_wallet_state,
                    wallets,
                    &wallets_instances,
                    &defaults,
                    &execution_context,
                )?;
            updated_wallets.push_wallet_state(signer_wallet_state.clone());
            if actions.has_pending_actions() {
                consolidated_actions.append(&mut actions);
                return Ok((updated_wallets, wallet_state, consolidated_actions));
            }
        Ok((wallets, wallet_state, consolidated_actions))
    }

    #[cfg(not(feature = "wasm"))]
    fn sign(
        _origin_uuid: &ConstructUuid,
        _title: &str,
        payload: &Value,
        _spec: &WalletSpecification,
        args: &ValueStore,
        wallet_state: ValueStore,
        mut wallets: WalletsState,
        wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        defaults: &AddonDefaults,
    ) -> WalletSignFutureResult {
        use txtx_addon_kit::futures::future;

        if let Some(signed_transaction_bytes) = wallet_state.get_value(SIGNED_TRANSACTION_BYTES) {
            let mut result = CommandExecutionResult::new();
            result.outputs.insert(
                SIGNED_TRANSACTION_BYTES.into(),
                signed_transaction_bytes.clone(),
            );

            return Ok(Box::pin(future::ready(Ok((wallets, wallet_state, result)))));
        }
        let signers = get_signers(&wallet_state, wallets_instances);
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

            for (wallet_uuid, wallet_instance) in signers.into_iter() {
                let wallet_state = wallets.pop_wallet_state(&wallet_uuid).unwrap();

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
                    &wallet_uuid,
                    &wallet_instance.name,
                    &payload,
                    &wallet_instance.specification,
                    &args,
                    wallet_state,
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
                    .get(SIGNATURE_BYTES)
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

            Ok((wallets, wallet_state, result))
        };
        Ok(Box::pin(future))
    }
}

fn get_current_tx_cursor_key(
    origin_uuid: &ConstructUuid,
    wallet_state: &ValueStore,
) -> (String, usize) {
    let cursor = wallet_state
        .get_expected_uint(&get_tx_cursor(origin_uuid))
        .unwrap_or(0) as usize;
    (get_tx_cursor_key(origin_uuid, cursor), cursor)
}

fn get_tx_cursor(origin_uuid: &ConstructUuid) -> String {
    format!("{}:cursor", origin_uuid.value())
}

fn get_tx_cursor_key(origin_uuid: &ConstructUuid, cursor: usize) -> String {
    format!("{}:{}", origin_uuid.value(), cursor)
}

fn get_signers(
    args: &ValueStore,
    wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
) -> Vec<(ConstructUuid, WalletInstance)> {
    let signers_uuid = args.get_expected_array("signers").unwrap();
    let mut signers = Vec::new();
    for signer_uuid in signers_uuid.iter() {
        let uuid = signer_uuid.as_string().unwrap();
        let uuid = ConstructUuid::from_uuid(&Uuid::from_str(uuid).unwrap());
        let wallet_instance = wallets_instances.get(&uuid).unwrap().clone();
        signers.push((uuid, wallet_instance));
    }
    signers
}
