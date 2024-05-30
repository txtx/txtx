use clarity::address::AddressHashMode;
use clarity::types::chainstate::StacksAddress;
use clarity::util::secp256k1::Secp256k1PublicKey;
use txtx_addon_kit::types::commands::{
    return_synchronous_result, CommandExecutionContext, CommandExecutionFutureResult,
    CommandExecutionResult,
};
use txtx_addon_kit::types::frontend::{
    ActionItemRequest, ActionItemRequestType, ActionItemStatus, ProvidePublicKeyRequest,
    ProvideSignedTransactionRequest,
};
use txtx_addon_kit::types::wallets::{
    WalletImplementation, WalletSpecification, WalletUsabilityFutureResult,
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
    FETCHED_BALANCE, FETCHED_NONCE, NETWORK_ID, RPC_API_URL,
};
use crate::rpc::StacksRpc;

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

    // check_usability analyses the wallet constructs.
    // it will returns all the ActionItemRequests required for a given wallet, which includes:
    // - ProvidePublicKey:
    // - ReviewInput (StacksAddress): Most of the case, unknown the first time it's being executed unless expected_address is provided in the construct
    // - ReviewInput (StacksBalance):
    // - ReviewInput (Assosiated Costs):
    // If the all of the informations above are present in the wallet state, nothing is returned.
    fn check_usability(
        _uuid: &ConstructUuid,
        _instance_name: &str,
        spec: &WalletSpecification,
        args: &ValueStore,
        state: &mut ValueStore,
        defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> WalletUsabilityFutureResult {
        let checked_public_key = state.get_expected_string(CHECKED_PUBLIC_KEY);
        let checked_address = state.get_expected_string(CHECKED_ADDRESS);
        let checked_cost_provision = state.get_expected_uint(CHECKED_COST_PROVISION);
        let fetched_nonce = state.get_expected_uint(FETCHED_NONCE);
        let fetched_balance = state.get_expected_uint(FETCHED_BALANCE);

        let expected_address = args.get_string("expected_address").map(|e| e.to_string());
        let is_address_check_required = expected_address.is_some();
        let is_balance_required = true;
        let is_nonce_required = true;

        let instance_name = _instance_name.to_string();
        let uuid = _uuid.clone();
        let rpc_api_url = args.retrieve_value_using_defaults(RPC_API_URL, defaults)?;
        let network_id = args.retrieve_value_using_defaults(NETWORK_ID, defaults)?;

        // let address_to_check = match (state.get("checked_expected_address"), args.get("expected_address")) {
        //     (Some(checked_address), Some(address_to_check)) => {
        //         check_expected_addressed_required = !checked_address.eq(address_to_check);
        //         Some(address_to_check.expect_string().to_string())
        //     }
        //     (_, Some(address_to_check)) => Some(address_to_check.expect_string().to_string()),
        //     None =>{
        //         None
        //     }
        // } ;

        // // Early return - public key was provided and expected addresss not specified
        // let public_key = match args.get("public_key") {
        //     Some(public_key) => {
        //         state.insert("public_key".into(), public_key.clone());
        //         Some(public_key.expect_string().to_string())
        //     }
        //     None =>{
        //         None
        //     }
        // } ;

        let future = async move {
            let stacks_rpc = StacksRpc::new(&rpc_api_url);
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
            }

            let mut action_item_requests = vec![];

            action_item_requests.push(ActionItemRequest::new(
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

            if let Some(expected_address) = expected_address {
                action_item_requests.push(ActionItemRequest::new(
                    &Uuid::new_v4(),
                    &Some(uuid.value()),
                    0,
                    "Check wallet signature provided",
                    &expected_address.to_string(),
                    ActionItemStatus::Todo,
                    ActionItemRequestType::ReviewInput,
                ))
            }
            Ok(action_item_requests)
        };
        Ok(Box::pin(future))
    }

    fn check_executability(
        uuid: &ConstructUuid,
        instance_name: &str,
        spec: &WalletSpecification,
        args: &ValueStore,
        _defaults: &AddonDefaults,
        execution_context: &CommandExecutionContext,
    ) -> Result<Vec<ActionItemRequest>, Diagnostic> {
        // Early return - public key was provided and expected address checked
        let check_expected_address = args.get_string("expected_address");
        if check_expected_address.is_some() {
            for input_spec in spec.inputs.iter() {
                if input_spec.name == "expected_address" && input_spec.check_performed {
                    return Ok(vec![]);
                }
            }
        }

        // Early return - public key was provided and expected addresss not specified
        if args.get_string("public_key").is_some() {
            for input_spec in spec.inputs.iter() {
                // todo: verify public_key/expected address match?
                if input_spec.name == "public_key" && input_spec.check_performed {
                    return Ok(vec![]);
                }
            }
        }

        let mut action_item_requests = vec![];
        action_item_requests.push(ActionItemRequest::new(
            &uuid.value(),
            &Some(uuid.value()),
            0,
            &format!("Connect wallet {instance_name}"),
            "".into(),
            ActionItemStatus::Todo,
            ActionItemRequestType::ProvidePublicKey(ProvidePublicKeyRequest {
                check_expectation_action_uuid: Some(uuid.value()), //todo: this is the wrong uuid
                message: "The Times 03/Jan/2009 Chancellor on brink of second bailout for banks."
                    .to_string(),
                namespace: "stacks".to_string(),
                network_id: "testnet".to_string(),
            }),
        ));

        if let Some(expected_address) = check_expected_address {
            action_item_requests.push(ActionItemRequest::new(
                &Uuid::new_v4(),
                &Some(uuid.value()),
                0,
                "Check wallet signature provided",
                &expected_address.to_string(),
                ActionItemStatus::Todo,
                ActionItemRequestType::ReviewInput,
            ))
        }

        return Ok(action_item_requests);
    }

    fn execute(
        _uuid: &ConstructUuid,
        spec: &WalletSpecification,
        args: &ValueStore,
        state: &mut ValueStore,
        defaults: &AddonDefaults,
        _progress_tx: &channel::Sender<(ConstructUuid, Diagnostic)>,
    ) -> CommandExecutionFutureResult {
        let mut result = CommandExecutionResult::new();
        // args.get_expected_string("public_key")

        // if let Some(public_key) = args.get("public_key") {
        //     state.insert(
        //         "public_keys",
        //         Value::array(vec![public_key.clone()]),
        //     );
        // } else {
        //     unreachable!("responsibility of 'check_executability'")
        // }
        // let network_id = args
        //     .get("network_id")
        //     .and_then(|a| Some(a.expect_string()))
        //     .or(defaults.keys.get("network_id").map(|x| x.as_str()))
        //     .ok_or(Diagnostic::error_from_string(format!(
        //         "command '{}': attribute 'network_id' is missing",
        //         spec.matcher
        //     )))
        //     .unwrap_or("testnet")
        //     .to_string();

        // let version = match network_id.as_str() {
        //     "mainnet" => AddressHashMode::SerializeP2PKH.to_version_mainnet(),
        //     _ => AddressHashMode::SerializeP2PKH.to_version_testnet(),
        // };

        // state.insert("hash_flag".to_string(), Value::uint(version.into()));
        // state.insert("multi_sig".to_string(), Value::bool(false));
        // state.insert("network_id".to_string(), Value::string(network_id));
        return_synchronous_result(Ok(result))
    }

    fn check_sign_executability(
        caller_uuid: &ConstructUuid,
        title: &str,
        payload: &Value,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        _defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> ActionItemRequest {
        ActionItemRequest::new(
            &Uuid::new_v4(),
            &Some(caller_uuid.value()),
            0,
            title,
            "", //payload,
            ActionItemStatus::Todo,
            ActionItemRequestType::ProvideSignedTransaction(ProvideSignedTransactionRequest {
                check_expectation_action_uuid: Some(caller_uuid.value()), // todo: this is the wrong uuid
                payload: payload.clone(),
                namespace: "stacks".to_string(),
                network_id: "".to_string(),
            }),
        )
    }

    fn check_public_key_expectations(
        _uuid: &ConstructUuid,
        instance_name: &str,
        public_key_bytes: &Vec<u8>,
        spec: &WalletSpecification,
        args: &ValueStore,
        defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> Result<Option<String>, Diagnostic> {
        let public_key = Secp256k1PublicKey::from_slice(&public_key_bytes).unwrap();

        let network_id = args.retrieve_value_using_defaults(NETWORK_ID, defaults)?;
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

    fn sign(
        _caller_uuid: &ConstructUuid,
        _title: &str,
        _payload: &Value,
        _spec: &WalletSpecification,
        _args: &ValueStore,
        _defaults: &AddonDefaults,
        _progress_tx: &channel::Sender<(ConstructUuid, Diagnostic)>,
    ) -> CommandExecutionFutureResult {
        let result = CommandExecutionResult::new();
        return_synchronous_result(Ok(result))
    }
}

pub async fn check_stacks_balance(api_url: &str) -> Result<u128, Diagnostic> {
    todo!("check_stacks_balance")

    // let client =  txtx_addon_kit::reqwest::Client::new();
    // let res = client
    //     .post(format!("{}/v2/transactions", api_url))
    //     .header("Content-Type", "application/json")
    //     .body(transaction_bytes.bytes)
    //     .send()
    //     .await
    //     .map_err(|e| {
    //         Diagnostic::error_from_string(format!(
    //             "Failed to broadcast stacks transaction: {e}"
    //         ))
    //     })?;

    // let status = res.status();
    // if !status.is_success() {
    //     let transaction: PostTransactionResponseError = res.json().await.map_err(|e| {
    //         println!("{:?}", e.to_string());
    //         Diagnostic::error_from_string(format!(
    //             "Failed to parse broadcasted Stacks transaction result: {e}"
    //         ))
    //     })?;
    //     return Err(Diagnostic::error_from_string(format!(
    //         "{:?}",
    //         transaction.reason
    //     )));
    // }
    // let mut txid = res.text().await.map_err(|e| {
    //     println!("{:?}", e.to_string());
    //     Diagnostic::error_from_string(format!(
    //         "Failed to parse broadcasted Stacks transaction result: {e}"
    //     ))
    // })?;

    // // Strip extra double quotes
    // txid = txid[1..65].to_string();
}
