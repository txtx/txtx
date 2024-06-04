use std::collections::HashMap;
use std::future;

use clarity::address::AddressHashMode;
use clarity::types::chainstate::StacksAddress;
use clarity::util::secp256k1::Secp256k1PublicKey;
use txtx_addon_kit::types::commands::{CommandExecutionContext, CommandExecutionResult};
use txtx_addon_kit::types::frontend::{
    ActionItemRequest, ActionItemRequestType, ActionItemStatus, Actions, BlockEvent,
    ProvideSignedTransactionRequest,
};
use txtx_addon_kit::types::wallets::{
    return_synchronous_result, WalletActivabilityFutureResult, WalletActivateFutureResult,
    WalletImplementation, WalletInstance, WalletSignFutureResult, WalletSpecification,
    WalletsState,
};
use txtx_addon_kit::types::{
    commands::CommandSpecification,
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{ConstructUuid, ValueStore};
use txtx_addon_kit::uuid::Uuid;
use txtx_addon_kit::{channel, AddonDefaults};

use crate::constants::{
    CHECKED_ADDRESS, CHECKED_COST_PROVISION, CHECKED_PUBLIC_KEY, EXPECTED_ADDRESS, FETCHED_BALANCE,
    FETCHED_NONCE, NETWORK_ID, PUBLIC_KEYS, RPC_API_URL, SIGNED_TRANSACTION_BYTES,
};
use crate::typing::CLARITY_BUFFER;

use super::get_addition_actions_for_address;

lazy_static! {
    pub static ref STACKS_CONNECT: WalletSpecification = {
        let mut wallet = define_wallet! {
            StacksConnect => {
                name: "Stacks Connect",
                matcher: "connect",
                documentation: "Coming soon",
                inputs: [
                    expected_address: {
                        documentation: "Coming soon",
                        typing: Type::string(),
                        optional: false,
                        interpolable: true
                    },
                    expected_public_key: {
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
                    }
                ],
                example: txtx_addon_kit::indoc! {r#"
                // Coming soon
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
    fn check_activability(
        uuid: &ConstructUuid,
        instance_name: &str,
        _spec: &WalletSpecification,
        args: &ValueStore,
        mut wallet_state: ValueStore,
        mut wallets: WalletsState,
        _wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
        is_balance_check_required: bool,
        is_public_key_required: bool,
    ) -> WalletActivabilityFutureResult {
        let _checked_public_key = wallet_state.get_expected_string(CHECKED_PUBLIC_KEY);
        let _checked_address = wallet_state.get_expected_string(CHECKED_ADDRESS);
        let _checked_cost_provision = wallet_state.get_expected_uint(CHECKED_COST_PROVISION);
        let _fetched_nonce = wallet_state.get_expected_uint(FETCHED_NONCE);
        let _fetched_balance = wallet_state.get_expected_uint(FETCHED_BALANCE);

        let expected_address = args.get_string("expected_address").map(|e| e.to_string());
        let _is_address_check_required = expected_address.is_some();
        let is_public_key_required = is_public_key_required || expected_address.is_none();
        let _is_nonce_required = true;

        let instance_name = instance_name.to_string();
        let uuid = uuid.clone();
        let rpc_api_url = match args.get_defaulting_string(RPC_API_URL, defaults) {
            Ok(value) => value,
            Err(diag) => return Err((wallets, diag)),
        };
        let network_id = match args.get_defaulting_string(NETWORK_ID, defaults) {
            Ok(value) => value,
            Err(diag) => return Err((wallets, diag)),
        };

        if let Ok(public_key_buffer) = args.get_expected_buffer("public_key", &CLARITY_BUFFER) {
            let version = if network_id.eq("mainnet") {
                clarity_repl::clarity::address::C32_ADDRESS_VERSION_MAINNET_SINGLESIG
            } else {
                clarity_repl::clarity::address::C32_ADDRESS_VERSION_TESTNET_SINGLESIG
            };

            let public_key = Secp256k1PublicKey::from_slice(&public_key_buffer.bytes).unwrap();

            let stx_address = StacksAddress::from_public_keys(
                version,
                &AddressHashMode::SerializeP2PKH,
                1,
                &vec![public_key],
            )
            .unwrap()
            .to_string();

            if let Ok(expected_stx_address) = args.get_expected_string(EXPECTED_ADDRESS) {
                if !expected_stx_address.eq(&stx_address) {
                    wallets.push_wallet_state(wallet_state);
                    return Err((
                        wallets,
                        diagnosed_error!(
                            "Wallet '{}': expected {} got {}",
                            instance_name,
                            expected_stx_address,
                            stx_address
                        ),
                    ));
                }
            }

            wallet_state.insert(
                CHECKED_PUBLIC_KEY,
                Value::string(txtx_addon_kit::hex::encode(public_key_buffer.bytes)),
            );
            let mut actions = Actions::none();
            actions.push_status_update_construct_uuid(
                &uuid,
                ActionItemStatus::Success(Some(stx_address.into())),
            );
            wallets.push_wallet_state(wallet_state);
            return Ok(Box::pin(future::ready(Ok((wallets, actions)))));
        }

        let future = async move {
            let mut actions = Actions::none();
            let res = get_addition_actions_for_address(
                &expected_address,
                &uuid,
                &instance_name,
                &network_id,
                &rpc_api_url,
                is_public_key_required,
                is_balance_check_required,
            )
            .await;
            wallets.push_wallet_state(wallet_state);

            let action_items = match res {
                Ok(action_items) => action_items,
                Err(diag) => return Err((wallets, diag)),
            };
            actions.push_sub_group(action_items);
            Ok((wallets, actions))
        };
        Ok(Box::pin(future))
    }

    fn activate(
        _uuid: &ConstructUuid,
        _spec: &WalletSpecification,
        args: &ValueStore,
        mut wallet_state: ValueStore,
        mut wallets: WalletsState,
        _wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        defaults: &AddonDefaults,
        _progress_tx: &channel::Sender<BlockEvent>,
    ) -> WalletActivateFutureResult {
        let result = CommandExecutionResult::new();
        let public_key = match wallet_state.get_expected_value(CHECKED_PUBLIC_KEY) {
            Ok(value) => value,
            Err(diag) => return Err((wallets, diag)),
        };
        let network_id = match args.get_defaulting_string(NETWORK_ID, defaults) {
            Ok(value) => value,
            Err(diag) => return Err((wallets, diag)),
        };
        wallet_state.insert(PUBLIC_KEYS, Value::array(vec![public_key.clone()]));

        let version = match network_id.as_str() {
            "mainnet" => AddressHashMode::SerializeP2PKH.to_version_mainnet(),
            _ => AddressHashMode::SerializeP2PKH.to_version_testnet(),
        };

        wallet_state.insert("hash_flag", Value::uint(version.into()));
        wallet_state.insert("multi_sig", Value::bool(false));

        wallets.push_wallet_state(wallet_state);
        return_synchronous_result(Ok((wallets, result)))
    }

    fn check_signability(
        uuid: &ConstructUuid,
        title: &str,
        payload: &Value,
        _spec: &WalletSpecification,
        args: &ValueStore,
        mut wallet_state: ValueStore,
        mut wallets: WalletsState,
        _wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> Result<(WalletsState, Actions), (WalletsState, Diagnostic)> {
        if let Ok(signed_transaction_bytes) = args.get_expected_value(SIGNED_TRANSACTION_BYTES) {
            // signed_transaction_bytes
            wallet_state.insert(&uuid.value().to_string(), signed_transaction_bytes.clone());
            wallets.push_wallet_state(wallet_state);
            return Ok((wallets, Actions::none()));
        }

        let network_id = match args.get_defaulting_string(NETWORK_ID, defaults) {
            Ok(value) => value,
            Err(diag) => return Err((wallets, diag)),
        };

        let request = ActionItemRequest::new(
            &Uuid::new_v4(),
            &Some(uuid.value()),
            0,
            title,
            "", //payload,
            ActionItemStatus::Todo,
            ActionItemRequestType::ProvideSignedTransaction(ProvideSignedTransactionRequest {
                check_expectation_action_uuid: Some(uuid.value()), // todo: this is the wrong uuid
                payload: payload.clone(),
                namespace: "stacks".to_string(),
                network_id,
            }),
        );
        wallets.push_wallet_state(wallet_state);
        Ok((wallets, Actions::new_sub_group_of_items(vec![request])))
    }

    fn sign(
        uuid: &ConstructUuid,
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
        let key = uuid.value().to_string();
        let signed_transaction = wallet_state
            .get_expected_value(&key)
            // .map_err(|e| (wallets, e))?;
            .unwrap();
        result
            .outputs
            .insert(SIGNED_TRANSACTION_BYTES.into(), signed_transaction.clone());

        wallets.push_wallet_state(wallet_state);
        return_synchronous_result(Ok((wallets, result)))
    }
}
