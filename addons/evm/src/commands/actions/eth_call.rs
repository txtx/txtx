use alloy::dyn_abi::DynSolValue;
use txtx_addon_kit::types::commands::{
    CommandExecutionFutureResult, CommandExecutionResult, CommandImplementation,
    PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::RunbookSupervisionContext;
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::types::{
    commands::CommandSpecification,
    diagnostics::Diagnostic,
    types::{Type, Value},
};

use crate::constants::RPC_API_URL;
use crate::rpc::EvmRpc;
use crate::typing::EVM_ADDRESS;

lazy_static! {
    pub static ref ETH_CALL: PreCommandSpecification = define_command! {
      EthCall => {
          name: "Eth Call",
          matcher: "eth_call",
          documentation: "The `evm::eth_call` command simulates an Ethereum transaction using the `eth_call` RPC endpoint.",
          implements_signing_capability: false,
          implements_background_task_capability: false,
          inputs: [
            description: {
                documentation: "A description of the call.",
                typing: Type::string(),
                optional: true,
                tainting: false,
                internal: false
            },
            rpc_api_url: {
                documentation: "The URL of the EVM API used to send the RPC request.",
                typing: Type::string(),
                optional: true,
                tainting: false,
                internal: false
            },
            contract_address: {
                documentation: "The address of the contract being called.",
                typing: Type::addon(EVM_ADDRESS),
                optional: false,
                tainting: true,
                internal: false
            },
            contract_abi: {
                documentation: "The contract ABI, optionally used to check input arguments before sending the transaction to the chain.",
                typing: Type::addon(EVM_ADDRESS),
                optional: true,
                tainting: true,
                internal: false
            },
            signer: {
                documentation: "The address that will be used as the sender of this contract call.",
                typing: Type::string(),
                optional: false,
                tainting: true,
                internal: false
            },
            function_name: {
                documentation: "The contract function to call.",
                typing: Type::string(),
                optional: true,
                tainting: true,
                internal: false
            },
            function_args: {
                documentation: "The contract function arguments.",
                typing: Type::array(Type::buffer()),
                optional: true,
                tainting: true,
                internal: false
            },
            amount: {
                documentation: "The amount, in Wei, to send in the transaction.",
                typing: Type::integer(),
                optional: true,
                tainting: true,
                internal: false
            },
            type: {
                documentation: "The transaction type. Options are 'Legacy', 'EIP2930', 'EIP1559', 'EIP4844'. The default is 'EIP1559'. This value will be retrieved from the network if omitted.",
                typing: Type::string(),
                optional: true,
                tainting: false,
                internal: false
            },
            max_fee_per_gas: {
                documentation: "Sets the max fee per gas of an EIP1559 transaction. This value will be retrieved from the network if omitted.",
                typing: Type::integer(),
                optional: true,
                tainting: false,
                internal: false
            },
            max_priority_fee_per_gas: {
                documentation: "Sets the max priority fee per gas of an EIP1559 transaction. This value will be retrieved from the network if omitted.",
                typing: Type::integer(),
                optional: true,
                tainting: false,
                internal: false
            },
            chain_id: {
                documentation: "The chain id.",
                typing: Type::string(),
                optional: true,
                tainting: true,
                internal: false
            },
            nonce: {
                documentation: "The account nonce of the sender. This value will be retrieved from the network if omitted.",
                typing: Type::integer(),
                optional: true,
                tainting: false,
                internal: false
            },
            gas_limit: {
                documentation: "Sets the maximum amount of gas that should be used to execute this transaction. This value will be retrieved from the network if omitted.",
                typing: Type::integer(),
                optional: true,
                tainting: false,
                internal: false
            },
            gas_price: {
                documentation: "Sets the gas price for Legacy transactions. This value will be retrieved from the network if omitted.",
                typing: Type::integer(),
                optional: true,
                tainting: false,
                internal: false
            }
          ],
          outputs: [
              result: {
                  documentation: "The contract call result.",
                  typing: Type::string()
              }
          ],
          example: txtx_addon_kit::indoc! {r#"
            action "call_some_contract" "evm::eth_call" {
                contract_address = input.contract_address
                function_name = "myFunction"
                function_args = [evm::bytes("0x1234")]
                signer = signer.operator.address
            }
      "#},
      }
    };
}

pub struct EthCall;
impl CommandImplementation for EthCall {
    fn check_instantiability(
        _ctx: &CommandSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn check_executability(
        _construct_id: &ConstructDid,
        _instance_name: &str,
        _spec: &CommandSpecification,
        _values: &ValueStore,
        _supervision_context: &RunbookSupervisionContext,
        _auth_context: &txtx_addon_kit::types::AuthorizationContext,
    ) -> Result<Actions, Diagnostic> {
        Ok(Actions::none()) // todo
    }

    fn run_execution(
        _construct_id: &ConstructDid,
        spec: &CommandSpecification,
        values: &ValueStore,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
    ) -> CommandExecutionFutureResult {
        let spec = spec.clone();
        let values = values.clone();

        let future = async move {
            let mut result = CommandExecutionResult::new();
            let call_result = build_eth_call(&spec, &values).await?;
            result.outputs.insert("result".into(), call_result);

            Ok(result)
        };

        Ok(Box::pin(future))
    }
}

#[cfg(not(feature = "wasm"))]
async fn build_eth_call(
    _spec: &CommandSpecification,
    values: &ValueStore,
) -> Result<Value, Diagnostic> {
    use alloy::json_abi::JsonAbi;

    use crate::{
        codec::{
            build_unsigned_transaction, value_to_abi_function_args, value_to_sol_value,
            CommonTransactionFields, TransactionType,
        },
        commands::actions::{
            call_contract::{
                encode_contract_call_inputs_from_abi_str, encode_contract_call_inputs_from_selector,
            },
            get_common_tx_params_from_args,
        },
        constants::{
            CHAIN_ID, CONTRACT_ABI, CONTRACT_ADDRESS, CONTRACT_FUNCTION_ARGS,
            CONTRACT_FUNCTION_NAME, SIGNER, TRANSACTION_TYPE,
        },
    };

    // let network_id = args.get_defaulting_string(NETWORK_ID, defaults)?;
    let rpc_api_url = values.get_expected_string(RPC_API_URL)?;
    let chain_id = values.get_expected_uint(CHAIN_ID)?;

    let contract_address: &Value = values.get_expected_value(CONTRACT_ADDRESS)?;
    let from = values.get_expected_value(SIGNER)?;
    let contract_abi = values.get_string(CONTRACT_ABI);
    let function_name = values.get_string(CONTRACT_FUNCTION_NAME);
    let function_args = values.get_value(CONTRACT_FUNCTION_ARGS);

    let (amount, gas_limit, nonce) =
        get_common_tx_params_from_args(values).map_err(|e| diagnosed_error!("{e}"))?;
    let tx_type = TransactionType::from_some_value(values.get_string(TRANSACTION_TYPE))?;

    let rpc = EvmRpc::new(&rpc_api_url).map_err(|e| diagnosed_error!("{e}"))?;

    let input = if let Some(function_name) = function_name {
        let function_args = if let Some(abi_str) = contract_abi {
            values
                .get_value(CONTRACT_FUNCTION_ARGS)
                .map(|v| {
                    let abi: JsonAbi = serde_json::from_str(&abi_str)
                        .map_err(|e| diagnosed_error!("invalid contract abi: {}", e))?;
                    value_to_abi_function_args(&function_name, &v, &abi)
                })
                .unwrap_or(Ok(vec![]))?
        } else {
            values
                .get_value(CONTRACT_FUNCTION_ARGS)
                .map(|v| {
                    v.expect_array()
                        .iter()
                        .map(|v| value_to_sol_value(&v).map_err(|e| diagnosed_error!("{}", e)))
                        .collect::<Result<Vec<DynSolValue>, Diagnostic>>()
                })
                .unwrap_or(Ok(vec![]))?
        };

        if let Some(abi_str) = contract_abi {
            encode_contract_call_inputs_from_abi_str(abi_str, function_name, &function_args)
                .map_err(|e| diagnosed_error!("{e}"))?
        } else {
            encode_contract_call_inputs_from_selector(function_name, &function_args)
                .map_err(|e| diagnosed_error!("{e}"))?
        }
    } else {
        // todo(hack): assume yul contract if no function name
        function_args.and_then(|a| Some(a.expect_buffer_bytes())).unwrap_or(vec![])
    };

    let common = CommonTransactionFields {
        to: Some(contract_address.clone()),
        from: from.clone(),
        nonce,
        chain_id,
        amount,
        gas_limit,
        tx_type,
        input: Some(input),
        deploy_code: None,
    };
    let (_, _, call_result) = build_unsigned_transaction(rpc.clone(), values, common)
        .await
        .map_err(|e| diagnosed_error!("{e}"))?;

    Ok(Value::string(call_result))
}
