use clarity::address::AddressHashMode;
use clarity::types::chainstate::StacksAddress;
use clarity::util::secp256k1::Secp256k1PublicKey;
use txtx_addon_kit::types::commands::{
    return_synchronous_result, CommandExecutionContext, CommandExecutionFutureResult,
    CommandExecutionResult,
};
use txtx_addon_kit::types::frontend::{
    ActionItemRequest, ActionItemRequestType, ActionItemStatus, ActionSubGroup,
    ProvidePublicKeyRequest, ProvideSignedTransactionRequest,
};
use txtx_addon_kit::types::wallets::{
    WalletActivabilityFutureResult, WalletImplementation, WalletSpecification,
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
    FETCHED_BALANCE, FETCHED_NONCE, NETWORK_ID, PUBLIC_KEYS, RPC_API_URL,
};
use crate::rpc::StacksRpc;
use crate::typing::CLARITY_BUFFER;

lazy_static! {
    pub static ref STACKS_CONNECT: WalletSpecification = define_wallet! {
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
        _uuid: &ConstructUuid,
        _instance_name: &str,
        _spec: &WalletSpecification,
        args: &ValueStore,
        state: &mut ValueStore,
        defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> WalletActivabilityFutureResult {
        let _checked_public_key = state.get_expected_string(CHECKED_PUBLIC_KEY);
        let _checked_address = state.get_expected_string(CHECKED_ADDRESS);
        let _checked_cost_provision = state.get_expected_uint(CHECKED_COST_PROVISION);
        let _fetched_nonce = state.get_expected_uint(FETCHED_NONCE);
        let _fetched_balance = state.get_expected_uint(FETCHED_BALANCE);

        let expected_address = args.get_string("expected_address").map(|e| e.to_string());
        let _is_address_check_required = expected_address.is_some();
        let _is_nonce_required = true;
        let is_balance_check_required = true;

        let instance_name = _instance_name.to_string();
        let uuid = _uuid.clone();
        let rpc_api_url = args.get_defaulting_string(RPC_API_URL, defaults)?;
        let network_id = args.get_defaulting_string(NETWORK_ID, defaults)?;

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
                    ActionItemRequestType::ReviewInput,
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
                    ActionItemRequestType::ReviewInput,
                );
                if let Some(ref expected_address) = expected_address {
                    let balance = stacks_rpc
                        .get_balance(&expected_address)
                        .await
                        .map_err(|e| {
                            diagnosed_error!(
                                "unable to retrieve balance {}: {}",
                                expected_address,
                                e.to_string()
                            )
                        })?;

                    check_balance.description = balance.balance.clone();
                }
                action_items.push(check_balance);
            }

            Ok(vec![ActionSubGroup {
                allow_batch_completion: false,
                action_items,
            }])
        };
        Ok(Box::pin(future))
    }

    fn activate(
        _uuid: &ConstructUuid,
        _spec: &WalletSpecification,
        args: &ValueStore,
        state: &mut ValueStore,
        defaults: &AddonDefaults,
        _progress_tx: &channel::Sender<(ConstructUuid, Diagnostic)>,
    ) -> CommandExecutionFutureResult {
        let result = CommandExecutionResult::new();
        let public_key = args.get_expected_value("public_key")?;
        let network_id = args.get_defaulting_string(NETWORK_ID, defaults)?;

        state.insert(PUBLIC_KEYS, Value::array(vec![public_key.clone()]));

        let version = match network_id.as_str() {
            "mainnet" => AddressHashMode::SerializeP2PKH.to_version_mainnet(),
            _ => AddressHashMode::SerializeP2PKH.to_version_testnet(),
        };

        state.insert("hash_flag", Value::uint(version.into()));
        state.insert("multi_sig", Value::bool(false));
        return_synchronous_result(Ok(result))
    }

    fn check_signability(
        caller_uuid: &ConstructUuid,
        title: &str,
        payload: &Value,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        state: &ValueStore,
        _defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> Result<Vec<ActionItemRequest>, Diagnostic> {
        let key = caller_uuid.value().to_string();
        let ations_items = match state.get_expected_buffer(&key, &CLARITY_BUFFER) {
            Ok(r) => vec![],
            Err(_) => {
                vec![ActionItemRequest::new(
                    &Uuid::new_v4(),
                    &Some(caller_uuid.value()),
                    0,
                    title,
                    "", //payload,
                    ActionItemStatus::Todo,
                    ActionItemRequestType::ProvideSignedTransaction(
                        ProvideSignedTransactionRequest {
                            check_expectation_action_uuid: Some(caller_uuid.value()), // todo: this is the wrong uuid
                            payload: payload.clone(),
                            namespace: "stacks".to_string(),
                            network_id: "".to_string(),
                        },
                    ),
                )]
            }
        };
        Ok(ations_items)
    }

    fn sign(
        caller_uuid: &ConstructUuid,
        _title: &str,
        _payload: &Value,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        state: &ValueStore,
        _defaults: &AddonDefaults,
    ) -> CommandExecutionFutureResult {
        let mut result = CommandExecutionResult::new();
        let key = caller_uuid.value().to_string();
        let signed_transaction = state.get_expected_value(&key)?;
        result.outputs.insert(
            "signed_transaction_bytes".into(),
            signed_transaction.clone(),
        );
        return_synchronous_result(Ok(result))
    }

    fn check_public_key_expectations(
        _uuid: &ConstructUuid,
        instance_name: &str,
        public_key_bytes: &Vec<u8>,
        _spec: &WalletSpecification,
        args: &ValueStore,
        defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> Result<Option<String>, Diagnostic> {
        let public_key = Secp256k1PublicKey::from_slice(&public_key_bytes).unwrap();

        let network_id = args.get_defaulting_string(NETWORK_ID, defaults)?;
        let version = if network_id.eq("mainnet") {
            clarity_repl::clarity::address::C32_ADDRESS_VERSION_MAINNET_SINGLESIG
        } else {
            clarity_repl::clarity::address::C32_ADDRESS_VERSION_TESTNET_SINGLESIG
        };

        let stx_address = StacksAddress::from_public_keys(
            version,
            &AddressHashMode::SerializeP2PKH,
            1,
            &vec![public_key],
        )
        .unwrap()
        .to_string();

        let Ok(check_expected_address) = args.get_expected_string(EXPECTED_ADDRESS) else {
            // No constraint on the address
            return Ok(Some(stx_address));
        };

        // Make sure the retrieve address is matching expectations
        if check_expected_address.eq(&stx_address) {
            return Ok(Some(stx_address));
        }

        return Err(diagnosed_error!(
            "Wallet '{}': expected {} got {}",
            instance_name,
            check_expected_address,
            stx_address
        ));
    }
}
