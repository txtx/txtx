use std::collections::HashMap;
use std::future;

use clarity::address::AddressHashMode;
use clarity::types::chainstate::StacksAddress;
use clarity::util::secp256k1::Secp256k1PublicKey;
use txtx_addon_kit::types::commands::{CommandExecutionContext, CommandExecutionResult};
use txtx_addon_kit::types::frontend::{
    ActionItemRequest, ActionItemRequestType, ActionItemStatus, Actions, BlockEvent,
    ProvidePublicKeyRequest, ProvideSignedTransactionRequest, ReviewInputRequest,
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
    CHECKED_ADDRESS, CHECKED_COST_PROVISION, CHECKED_PUBLIC_KEY, DEFAULT_MESSAGE, EXPECTED_ADDRESS,
    FETCHED_BALANCE, FETCHED_NONCE, NETWORK_ID, PUBLIC_KEYS, RPC_API_URL, SIGNED_TRANSACTION_BYTES,
};
use crate::rpc::StacksRpc;
use crate::typing::CLARITY_BUFFER;

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
        _instance_name: &str,
        _spec: &WalletSpecification,
        args: &ValueStore,
        mut wallet_state: ValueStore,
        mut wallets: WalletsState,
        _wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> WalletActivabilityFutureResult {
        let _checked_public_key = wallet_state.get_expected_string(CHECKED_PUBLIC_KEY);
        let _checked_address = wallet_state.get_expected_string(CHECKED_ADDRESS);
        let _checked_cost_provision = wallet_state.get_expected_uint(CHECKED_COST_PROVISION);
        let _fetched_nonce = wallet_state.get_expected_uint(FETCHED_NONCE);
        let _fetched_balance = wallet_state.get_expected_uint(FETCHED_BALANCE);

        let expected_address = args.get_string("expected_address").map(|e| e.to_string());
        let _is_address_check_required = expected_address.is_some();
        let _is_nonce_required = true;
        let is_balance_check_required = true;

        // WIP
        let instance_name = _instance_name.to_string();
        let uuid = uuid.clone();
        let rpc_api_url = args.get_defaulting_string(RPC_API_URL, defaults).unwrap();
        // .map_err(|e| (wallets, e))?;
        let network_id = args.get_defaulting_string(NETWORK_ID, defaults).unwrap();
        // .map_err(|e| (wallets, e))?;

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
            actions.push_status_udpate_construct_uuid(
                &uuid,
                ActionItemStatus::Success(Some(stx_address.into())),
            );
            wallets.push_wallet_state(wallet_state);
            return Ok(Box::pin(future::ready(Ok((wallets, actions)))));
        }

        let future = async move {
            let stacks_rpc = StacksRpc::new(&rpc_api_url);
            let mut action_items = vec![];

            action_items.push(ActionItemRequest::new(
                &uuid.value(),
                &Some(uuid.value()),
                0,
                &format!("Connect wallet {instance_name}"),
                "".into(),
                ActionItemStatus::Todo,
                ActionItemRequestType::ProvidePublicKey(ProvidePublicKeyRequest {
                    check_expectation_action_uuid: Some(uuid.value()),
                    message: DEFAULT_MESSAGE.to_string(),
                    network_id,
                    namespace: "stacks".to_string(),
                }),
            ));

            if let Some(ref expected_address) = expected_address {
                action_items.push(ActionItemRequest::new(
                    &Uuid::new_v4(),
                    &Some(uuid.value()),
                    0,
                    "Check consistency with expected_address",
                    &expected_address.to_string(),
                    ActionItemStatus::Todo,
                    ActionItemRequestType::ReviewInput(ReviewInputRequest {
                        input_name: "".into(), // todo
                    }),
                ))
            }

            if is_balance_check_required {
                let mut check_balance = ActionItemRequest::new(
                    &Uuid::new_v4(),
                    &Some(uuid.value()),
                    0,
                    "Check wallet balance (STX)",
                    "",
                    ActionItemStatus::Todo,
                    ActionItemRequestType::ReviewInput(ReviewInputRequest {
                        input_name: "".into(), // todo
                    }),
                );
                if let Some(ref expected_address) = expected_address {
                    let balance = stacks_rpc
                        .get_balance(&expected_address)
                        .await
                        // .map_err(|e| {
                        //     (
                        //         wallets,
                        //         diagnosed_error!(
                        //             "unable to retrieve balance {}: {}",
                        //             expected_address,
                        //             e.to_string()
                        //         ),
                        //     )
                        // })?;
                        .unwrap();

                    check_balance.description = balance.balance.clone();
                }
                action_items.push(check_balance);
            }
            wallets.push_wallet_state(wallet_state);
            Ok((wallets, Actions::new_sub_group_of_items(action_items)))
        };
        Ok(Box::pin(future))
    }

    fn activate(
        uuid: &ConstructUuid,
        _spec: &WalletSpecification,
        args: &ValueStore,
        mut wallet_state: ValueStore,
        mut wallets: WalletsState,
        _wallets_instances: &HashMap<ConstructUuid, WalletInstance>,
        defaults: &AddonDefaults,
        _progress_tx: &channel::Sender<BlockEvent>,
    ) -> WalletActivateFutureResult {
        // WIP
        let result = CommandExecutionResult::new();
        let public_key = wallet_state.get_expected_value(CHECKED_PUBLIC_KEY).unwrap();
        // .map_err(|e| (wallets, e))?;
        let network_id = args.get_defaulting_string(NETWORK_ID, defaults).unwrap();
        // .map_err(|e| (wallets, e))?;

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
        let Ok(signed_transaction_bytes) = args.get_expected_value(SIGNED_TRANSACTION_BYTES) else {
            let network_id = args
                .get_defaulting_string(NETWORK_ID, defaults)
                // .map_err(|e| (wallets, e))?;
                .unwrap();

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
            return Ok((wallets, Actions::new_sub_group_of_items(vec![request])));
        };

        // signed_transaction_bytes
        wallet_state.insert(&uuid.value().to_string(), signed_transaction_bytes.clone());
        wallets.push_wallet_state(wallet_state);
        Ok((wallets, Actions::none()))
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
