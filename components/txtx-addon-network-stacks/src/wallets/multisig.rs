use std::collections::HashMap;
use std::str::FromStr;

use clarity::address::AddressHashMode;
use clarity::types::chainstate::StacksAddress;
use clarity::util::secp256k1::Secp256k1PublicKey;
use txtx_addon_kit::types::commands::{
    CommandExecutionContext, CommandExecutionResult, CommandSpecification,
};
use txtx_addon_kit::types::frontend::{
    ActionItemRequest, ActionItemRequestType, ActionItemStatus, Actions, BlockEvent, OpenModalData,
    ProvideSignedTransactionRequest,
};
use txtx_addon_kit::types::wallets::{
    return_synchronous_result, WalletActivabilityFutureResult, WalletActivateFutureResult,
    WalletImplementation, WalletInstance, WalletSignFutureResult, WalletSpecification,
    WalletsState,
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
    ) -> WalletActivabilityFutureResult {
        let root_uuid = uuid.clone();
        let signers = get_signers(args, wallets_instances);

        let args = args.clone();
        let wallets_instances = wallets_instances.clone();
        let defaults = defaults.clone();
        let execution_context = execution_context.clone();
        let instance_name = instance_name.to_string();
        let expected_address = None;
        let rpc_api_url = match args.get_defaulting_string(RPC_API_URL, &defaults) {
            Ok(value) => value,
            Err(diag) => return Err((wallets, diag)),
        };
        let network_id = match args.get_defaulting_string(NETWORK_ID, &defaults) {
            Ok(value) => value,
            Err(diag) => return Err((wallets, diag)),
        };

        let future = async move {
            let mut consolidated_actions = Actions::none();

            let modal =
                BlockEvent::new_modal("Stacks Multisig Configuration assistant", "", vec![]);
            let mut open_modal_action = vec![ActionItemRequest::new(
                &Uuid::new_v4(),
                &Some(root_uuid.value()),
                0,
                "Compute multisig address",
                "",
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
                Err(diag) => return Err((wallets, diag)),
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
                let (updated_wallets, mut actions) = future.await?;
                wallets = updated_wallets;

                println!("NEW_ACTIONS: {:?}", actions);
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
                                wallets.push_wallet_state(wallet_state);
                                return Err((
                                    wallets,
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
                        Ok(response) => ActionItemStatus::Success(Some(response.balance.clone())),
                        Err(e) => {
                            let diag = diagnosed_error!(
                                "unable to retrieve balance {}: {}",
                                stacks_address,
                                e.to_string()
                            );
                            ActionItemStatus::Error(diag)
                        }
                    };

                    actions.push_status_update_construct_uuid(
                        &root_uuid,
                        status_update,
                        ACTION_ITEM_CHECK_BALANCE,
                    );
                    actions.push_status_update_construct_uuid(
                        &root_uuid,
                        ActionItemStatus::Success(Some(stacks_address)),
                        ACTION_ITEM_PROVIDE_PUBLIC_KEY,
                    );
                    consolidated_actions = actions;
                } else {
                    println!("Unable to compute Stacks address");
                }
            } else {
                let validate_modal_action = ActionItemRequest::new(
                    &Uuid::new_v4(),
                    &Some(root_uuid.value()),
                    0,
                    "CONFIRM",
                    "",
                    ActionItemStatus::Todo,
                    ActionItemRequestType::ValidateModal,
                    "modal",
                );
                consolidated_actions.push_sub_group(vec![validate_modal_action]);
            }

            wallets.push_wallet_state(wallet_state);
            Ok((wallets, consolidated_actions))
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
            Err(diag) => return Err((wallets, diag)),
        };
        let network_id = match args.get_defaulting_string(NETWORK_ID, defaults) {
            Ok(value) => value,
            Err(diag) => return Err((wallets, diag)),
        };

        let signers = get_signers(args, wallets_instances);

        let args = args.clone();
        let wallets_instances = wallets_instances.clone();
        let defaults = defaults.clone();
        let progress_tx = progress_tx.clone();

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
                let (updated_wallets, _) = future.await?;
                wallets = updated_wallets;
            }

            wallet_state.insert(PUBLIC_KEYS, public_key.clone());

            let version = match network_id.as_str() {
                "mainnet" => AddressHashMode::SerializeP2SH.to_version_mainnet(),
                _ => AddressHashMode::SerializeP2SH.to_version_testnet(),
            };
            wallet_state.insert("hash_flag", Value::uint(version.into()));
            wallet_state.insert("multi_sig", Value::bool(true));
            wallets.push_wallet_state(wallet_state);

            result.outputs.insert("signers".into(), Value::array(signers_uuids));
            result.outputs.insert("public_key".into(), public_key);

            Ok((wallets, result))
        };
        Ok(Box::pin(future))
    }

    fn check_signability(
        origin_uuid: &ConstructUuid,
        title: &str,
        payload: &Value,
        _spec: &WalletSpecification,
        args: &ValueStore,
        mut wallet_state: ValueStore,
        mut wallets: WalletsState,
        wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        defaults: &AddonDefaults,
        execution_context: &CommandExecutionContext,
    ) -> Result<(WalletsState, Actions), (WalletsState, Diagnostic)> {
        let mut transaction_bytes_to_sign = payload.clone();
        let mut signers = get_signers(args, wallets_instances);
        let (tx_cursor_key, mut cursor) = get_current_tx_cursor_key(origin_uuid, &wallet_state);
        let (signer_uuid, signer_wallet_instance) = signers.remove(cursor);

        if let Ok(signed_transaction_bytes) = args.get_expected_value(SIGNED_TRANSACTION_BYTES) {
            let signer_wallet_state = wallets.pop_wallet_state(&signer_uuid).unwrap();
            let (updated_wallets, actions) =
                (signer_wallet_instance.specification.check_signability)(
                    &signer_uuid,
                    &format!("{} - {}", title, signer_wallet_instance.name),
                    &payload,
                    &signer_wallet_instance.specification,
                    &args,
                    signer_wallet_state,
                    wallets,
                    &wallets_instances,
                    &defaults,
                    &execution_context,
                )?;
            wallets = updated_wallets;
            if actions.has_pending_actions() {
                wallets.push_wallet_state(wallet_state);
                return Ok((wallets, actions));
            }

            // We store the signature
            wallet_state.insert(&tx_cursor_key, signed_transaction_bytes.clone());
            // Was this the last signature?
            if cursor == signers.len() {
                // We are done!
                wallet_state.insert(
                    &origin_uuid.value().to_string(),
                    signed_transaction_bytes.clone(),
                );
                wallets.push_wallet_state(wallet_state);
                return Ok((wallets, Actions::none()));
            }

            // We increment the tx_cursor_key
            cursor += 1;
            wallet_state.insert(&get_tx_cursor(origin_uuid), Value::uint(cursor as u64));
            wallets.push_wallet_state(wallet_state);

            // We update the transaction_bytes_to_sign
            transaction_bytes_to_sign = signed_transaction_bytes.clone();
        }

        // Retrieve the last signature, and propose it to the next one
        let network_id = match args.get_defaulting_string(NETWORK_ID, defaults) {
            Ok(value) => value,
            Err(diag) => return Err((wallets, diag)),
        };

        let request = ActionItemRequest::new(
            &Uuid::new_v4(),
            &Some(origin_uuid.value()),
            0,
            title,
            "", //payload,
            ActionItemStatus::Todo,
            ActionItemRequestType::ProvideSignedTransaction(ProvideSignedTransactionRequest {
                check_expectation_action_uuid: Some(origin_uuid.value()), // todo: this is the wrong uuid
                payload: transaction_bytes_to_sign.clone(),
                namespace: "stacks".to_string(),
                network_id,
            }),
            ACTION_ITEM_PROVIDE_SIGNED_TRANSACTION,
        );

        Ok((wallets, Actions::new_sub_group_of_items(vec![request])))
    }

    fn sign(
        origin_uuid: &ConstructUuid,
        _title: &str,
        _payload: &Value,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        wallet_state: ValueStore,
        mut wallets: WalletsState,
        _wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        _defaults: &AddonDefaults,
    ) -> WalletSignFutureResult {
        let mut result = CommandExecutionResult::new();
        let key = origin_uuid.value().to_string();

        let signed_transaction = match wallet_state.get_expected_value(&key) {
            Ok(value) => value,
            Err(diag) => return Err((wallets, diag)),
        };

        result
            .outputs
            .insert(SIGNED_TRANSACTION_BYTES.into(), signed_transaction.clone());

        wallets.push_wallet_state(wallet_state);
        return_synchronous_result(Ok((wallets, result)))
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
        let wallet_spec = wallets_instances.get(&uuid).unwrap().clone();
        signers.push((uuid, wallet_spec));
    }
    signers
}
