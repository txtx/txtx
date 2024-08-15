use std::collections::HashMap;

use clarity::address::AddressHashMode;
use clarity::codec::StacksMessageCodec;
use clarity::types::chainstate::StacksAddress;
use clarity::util::secp256k1::Secp256k1PublicKey;
use clarity_repl::codec::StacksTransaction;
use txtx_addon_kit::types::commands::CommandExecutionResult;
use txtx_addon_kit::types::frontend::{
    ActionItemRequest, ActionItemRequestType, ActionItemRequestUpdate, ActionItemStatus, Actions,
    BlockEvent, ProvideSignedTransactionRequest,
};
use txtx_addon_kit::types::types::RunbookSupervisionContext;
use txtx_addon_kit::types::wallets::{
    return_synchronous_actions, return_synchronous_result, CheckSignabilityOk,
    SigningCommandsState, WalletActionErr, WalletActionsFutureResult, WalletActivateFutureResult,
    WalletImplementation, WalletInstance, WalletSignFutureResult, WalletSpecification,
};
use txtx_addon_kit::types::{
    commands::CommandSpecification,
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{ConstructDid, ValueStore};
use txtx_addon_kit::{channel, AddonDefaults};

use crate::constants::{
    ACTION_ITEM_CHECK_ADDRESS, ACTION_ITEM_PROVIDE_PUBLIC_KEY,
    ACTION_ITEM_PROVIDE_SIGNED_TRANSACTION, CHECKED_ADDRESS, CHECKED_COST_PROVISION,
    CHECKED_PUBLIC_KEY, EXPECTED_ADDRESS, FETCHED_BALANCE, FETCHED_NONCE, NETWORK_ID, PUBLIC_KEYS,
    REQUESTED_STARTUP_DATA, RPC_API_URL, SIGNED_TRANSACTION_BYTES,
};

use super::get_addition_actions_for_address;

lazy_static! {
    pub static ref STACKS_CONNECT: WalletSpecification = {
        let mut wallet = define_wallet! {
            StacksConnect => {
                name: "Stacks Connect",
                matcher: "connect",
                documentation:txtx_addon_kit::indoc! {r#"The `connect` wallet will route the transaction signing process through [Stacks.js connect](https://www.hiro.so/stacks-js).
                This allows a Runbook operator to sign the transaction with the browser wallet of their choice."#},
                inputs: [
                    expected_address: {
                        documentation: "The Stacks address that is expected to connect to the Runbook execution. Omitting this field will allow any address to be used for this wallet.",
                        typing: Type::string(),
                        optional: false,
                        interpolable: true,
                        sensitive: true
                    }
                ],
                outputs: [
                    public_key: {
                        documentation: "The public key of the connected wallet.",
                        typing: Type::array(Type::buffer())
                    }
                ],
                example: txtx_addon_kit::indoc! {r#"
                wallet "alice" "stacks::connect" {
                    expected_address = "ST12886CEM87N4TP9CGV91VWJ8FXVX57R6AG1AXS4"
                }
                "#},
            }
        };
        wallet.requires_interaction = true;
        wallet
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

    // check_activability analyses the wallet constructs.
    // it will returns all the ActionItemRequests required for a given wallet, which includes:
    // - ProvidePublicKey:
    // - ReviewInput (StacksAddress): Most of the case, unknown the first time it's being executed unless expected_address is provided in the construct
    // - ReviewInput (StacksBalance):
    // - ReviewInput (Assosiated Costs):
    // If the all of the informations above are present in the wallet state, nothing is returned.
    #[cfg(not(feature = "wasm"))]
    fn check_activability(
        construct_did: &ConstructDid,
        instance_name: &str,
        _spec: &WalletSpecification,
        args: &ValueStore,
        mut signing_command_state: ValueStore,
        wallets: SigningCommandsState,
        _wallets_instances: &HashMap<ConstructDid, WalletInstance>,
        defaults: &AddonDefaults,
        _supervision_context: &RunbookSupervisionContext,
        is_balance_check_required: bool,
        is_public_key_required: bool,
    ) -> WalletActionsFutureResult {
        use crate::constants::RPC_API_AUTH_TOKEN;

        let checked_public_key = signing_command_state.get_expected_string(CHECKED_PUBLIC_KEY);
        let _requested_startup_data = signing_command_state
            .get_expected_bool(REQUESTED_STARTUP_DATA)
            .ok()
            .unwrap_or(false);
        let _checked_address = signing_command_state.get_expected_string(CHECKED_ADDRESS);
        let _checked_cost_provision =
            signing_command_state.get_expected_integer(CHECKED_COST_PROVISION);
        let _fetched_nonce = signing_command_state.get_expected_integer(FETCHED_NONCE);
        let _fetched_balance = signing_command_state.get_expected_integer(FETCHED_BALANCE);

        let expected_address = args.get_string("expected_address").map(|e| e.to_string());
        let do_request_address_check = expected_address.is_some();
        let do_request_public_key = is_public_key_required;
        // only request public key if we haven't already created that action

        let _is_nonce_required = true;
        let do_request_balance = is_balance_check_required;

        let instance_name = instance_name.to_string();
        let signing_construct_did = construct_did.clone();
        let rpc_api_url = match args.get_defaulting_string(RPC_API_URL, defaults) {
            Ok(value) => value,
            Err(diag) => return Err((wallets, signing_command_state, diag)),
        };
        let rpc_api_auth_token = args
            .get_defaulting_string(RPC_API_AUTH_TOKEN, defaults)
            .ok();

        let network_id = match args.get_defaulting_string(NETWORK_ID, defaults) {
            Ok(value) => value,
            Err(diag) => return Err((wallets, signing_command_state, diag)),
        };

        if let Ok(public_key_bytes) = args.get_expected_buffer_bytes("public_key") {
            let version = if network_id.eq("mainnet") {
                clarity_repl::clarity::address::C32_ADDRESS_VERSION_MAINNET_SINGLESIG
            } else {
                clarity_repl::clarity::address::C32_ADDRESS_VERSION_TESTNET_SINGLESIG
            };

            let public_key = Secp256k1PublicKey::from_slice(&public_key_bytes).unwrap();

            let stx_address = StacksAddress::from_public_keys(
                version,
                &AddressHashMode::SerializeP2PKH,
                1,
                &vec![public_key],
            )
            .unwrap()
            .to_string();

            let mut actions: Actions = Actions::none();
            let mut success = true;
            let mut status_update = ActionItemStatus::Success(Some(stx_address.to_string()));
            if let Ok(expected_stx_address) = args.get_expected_string(EXPECTED_ADDRESS) {
                if !expected_stx_address.eq(&stx_address) {
                    status_update = ActionItemStatus::Error(diagnosed_error!(
                        "Wallet '{}': expected {} got {}",
                        instance_name,
                        expected_stx_address,
                        stx_address
                    ));
                    success = false;
                } else {
                    let update = ActionItemRequestUpdate::from_context(
                        &signing_construct_did,
                        ACTION_ITEM_CHECK_ADDRESS,
                    )
                    .set_status(status_update.clone());
                    actions.push_action_item_update(update);
                }
            }
            if success {
                signing_command_state.insert(
                    CHECKED_PUBLIC_KEY,
                    Value::string(txtx_addon_kit::hex::encode(public_key_bytes)),
                );
            }
            let update = ActionItemRequestUpdate::from_context(
                &signing_construct_did,
                ACTION_ITEM_PROVIDE_PUBLIC_KEY,
            )
            .set_status(status_update);
            actions.push_action_item_update(update);

            return return_synchronous_actions(Ok((wallets, signing_command_state, actions)));
        } else if checked_public_key.is_ok() {
            return return_synchronous_actions(Ok((
                wallets,
                signing_command_state,
                Actions::none(),
            )));
        }

        let future = async move {
            let mut actions = Actions::none();
            let res = get_addition_actions_for_address(
                &expected_address,
                &signing_construct_did,
                &instance_name,
                &network_id,
                &rpc_api_url,
                &rpc_api_auth_token,
                do_request_public_key,
                do_request_balance,
                do_request_address_check,
            )
            .await;
            signing_command_state.insert(&REQUESTED_STARTUP_DATA, Value::bool(true));

            let action_items = match res {
                Ok(action_items) => action_items,
                Err(diag) => return Err((wallets, signing_command_state, diag)),
            };
            if !action_items.is_empty() {
                actions.push_group(
                    "Review and check the following wallet related action items",
                    action_items,
                );
            }
            Ok((wallets, signing_command_state, actions))
        };
        Ok(Box::pin(future))
    }

    fn activate(
        _construct_id: &ConstructDid,
        _spec: &WalletSpecification,
        args: &ValueStore,
        mut signing_command_state: ValueStore,
        wallets: SigningCommandsState,
        _wallets_instances: &HashMap<ConstructDid, WalletInstance>,
        defaults: &AddonDefaults,
        _progress_tx: &channel::Sender<BlockEvent>,
    ) -> WalletActivateFutureResult {
        let result = CommandExecutionResult::new();
        let public_key = match signing_command_state.get_expected_value(CHECKED_PUBLIC_KEY) {
            Ok(value) => value,
            Err(diag) => {
                return Err((wallets, signing_command_state, diag));
            }
        };
        let network_id = match args.get_defaulting_string(NETWORK_ID, defaults) {
            Ok(value) => value,
            Err(diag) => return Err((wallets, signing_command_state, diag)),
        };
        signing_command_state.insert(PUBLIC_KEYS, Value::array(vec![public_key.clone()]));

        let version = match network_id.as_str() {
            "mainnet" => AddressHashMode::SerializeP2PKH.to_version_mainnet(),
            _ => AddressHashMode::SerializeP2PKH.to_version_testnet(),
        };

        signing_command_state.insert("hash_flag", Value::integer(version.into()));
        signing_command_state.insert("multi_sig", Value::bool(false));

        return_synchronous_result(Ok((wallets, signing_command_state, result)))
    }

    fn check_signability(
        construct_did: &ConstructDid,
        title: &str,
        description: &Option<String>,
        payload: &Value,
        _spec: &WalletSpecification,
        args: &ValueStore,
        signing_command_state: ValueStore,
        wallets: SigningCommandsState,
        _wallets_instances: &HashMap<ConstructDid, WalletInstance>,
        defaults: &AddonDefaults,
        _supervision_context: &RunbookSupervisionContext,
    ) -> Result<CheckSignabilityOk, WalletActionErr> {
        if let Some(_) = signing_command_state
            .get_scoped_value(&construct_did.to_string(), SIGNED_TRANSACTION_BYTES)
        {
            return Ok((wallets, signing_command_state, Actions::none()));
        }

        let network_id = match args.get_defaulting_string(NETWORK_ID, defaults) {
            Ok(value) => value,
            Err(diag) => return Err((wallets, signing_command_state, diag)),
        };
        let (status, payload) = if let Some(()) = payload.as_null() {
            (ActionItemStatus::Blocked, Value::string("N/A".to_string()))
        } else {
            let _ =
                StacksTransaction::consensus_deserialize(&mut &payload.expect_buffer_bytes()[..]);
            (ActionItemStatus::Todo, payload.clone())
        };

        let request = ActionItemRequest::new(
            &Some(construct_did.clone()),
            title,
            description.clone(),
            status,
            ActionItemRequestType::ProvideSignedTransaction(ProvideSignedTransactionRequest {
                check_expectation_action_uuid: Some(construct_did.clone()),
                signer_uuid: ConstructDid(signing_command_state.uuid.clone()),
                payload: payload.clone(),
                namespace: "stacks".to_string(),
                network_id,
            }),
            ACTION_ITEM_PROVIDE_SIGNED_TRANSACTION,
        );
        Ok((
            wallets,
            signing_command_state,
            Actions::append_item(
                request,
                Some("Review and sign the transactions from the list below"),
                Some("Transaction Signing"),
            ),
        ))
    }

    fn sign(
        construct_did: &ConstructDid,
        _title: &str,
        _payload: &Value,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        signing_command_state: ValueStore,
        wallets: SigningCommandsState,
        _wallets_instances: &HashMap<ConstructDid, WalletInstance>,
        _defaults: &AddonDefaults,
    ) -> WalletSignFutureResult {
        let mut result = CommandExecutionResult::new();
        let key = construct_did.to_string();
        let signed_transaction = signing_command_state
            .get_expected_value(&key)
            // .map_err(|e| (wallets, e))?;
            .unwrap();
        result
            .outputs
            .insert(SIGNED_TRANSACTION_BYTES.into(), signed_transaction.clone());

        return_synchronous_result(Ok((wallets, signing_command_state, result)))
    }
}