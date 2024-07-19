use alloy::dyn_abi::DynSolValue;
use txtx_addon_kit::types::commands::{
    CommandExecutionFutureResult, CommandExecutionResult, CommandImplementation,
    PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::types::RunbookSupervisionContext;
use txtx_addon_kit::types::{
    commands::CommandSpecification,
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{ConstructDid, ValueStore};
use txtx_addon_kit::AddonDefaults;

use crate::constants::{NETWORK_ID, RPC_API_URL};
use crate::rpc::EVMRpc;
use crate::typing::ETH_ADDRESS;

lazy_static! {
    pub static ref CALL_VIEW_FUNCTION: PreCommandSpecification = define_command! {
      CallEVMViewFunction => {
          name: "Call EVM Contract View Function",
          matcher: "call_view_function",
          documentation: "The `evm::call_view_function` calls a contract function that is marked as `view`.",
          implements_signing_capability: false,
          implements_background_task_capability: false,
          inputs: [
            description: {
                documentation: "A description of the call.",
                typing: Type::string(),
                optional: true,
                interpolable: true
            },
            contract_address: {
                documentation: "The address of the contract being called.",
                typing: Type::addon(ETH_ADDRESS.clone()),
                optional: false,
                interpolable: true
            },
            contract_abi: {
                documentation: "The contract ABI, optionally used to check input arguments before sending the transaction to the chain.",
                typing: Type::addon(ETH_ADDRESS.clone()),
                optional: true,
                interpolable: true
            },
            from: {
                documentation: "The address that will be used as the sender of this contract call.",
                typing: Type::string(),
                optional: false,
                interpolable: true
            },
            function_name: {
                documentation: "The contract function to call.",
                typing: Type::string(),
                optional: false,
                interpolable: true
            },
            function_args: {
                documentation: "The contract function argument.",
                typing: Type::array(Type::buffer()),
                optional: true,
                interpolable: true
            },
            amount: {
                documentation: "The amount, in Wei, to send in the call to the `view` function.",
                typing: Type::uint(),
                optional: true,
                interpolable: true
            },
            type: {
                documentation: "The transaction type. Options are 'Legacy', 'EIP2930', 'EIP1559', 'EIP4844'. The default is 'EIP1559'.",
                typing: Type::string(),
                optional: true,
                interpolable: true
            },
            max_fee_per_gas: {
                documentation: "Sets the max fee per gas of an EIP1559 transaction.",
                typing: Type::uint(),
                optional: true,
                interpolable: true
            },
            max_priority_fee_per_gas: {
                documentation: "Sets the max priority fee per gas of an EIP1559 transaction.",
                typing: Type::uint(),
                optional: true,
                interpolable: true
            },
            chain_id: {
                documentation: "The chain id.",
                typing: Type::string(),
                optional: true,
                interpolable: true
            },
            nonce: {
                documentation: "The account nonce of the sender. This value will be retrieved from the network if omitted.",
                typing: Type::uint(),
                optional: true,
                interpolable: true
            },
            gas_limit: {
                documentation: "Sets the maximum amount of gas that should be used to execute this transaction.",
                typing: Type::uint(),
                optional: true,
                interpolable: true
            },
            gas_price: {
                documentation: "Sets the gas price for Legacy transactions.",
                typing: Type::uint(),
                optional: true,
                interpolable: true
            },
            network_id: {
                documentation: "The network id.",
                typing: Type::string(),
                optional: true,
                interpolable: true
            },
            depends_on: {
              documentation: "References another command's outputs, preventing this command from executing until the referenced command is successful.",
              typing: Type::string(),
              optional: true,
              interpolable: true
            }
          ],
          outputs: [
              result: {
                  documentation: "The contract call result.",
                  typing: Type::string()
              }
          ],
          example: txtx_addon_kit::indoc! {r#"
          // Coming soon
      "#},
      }
    };
}

pub struct CallEVMViewFunction;
impl CommandImplementation for CallEVMViewFunction {
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
        _args: &ValueStore,
        _defaults: &AddonDefaults,
        _supervision_context: &RunbookSupervisionContext,
    ) -> Result<Actions, Diagnostic> {
        Ok(Actions::none()) // todo
    }

    fn run_execution(
        _construct_id: &ConstructDid,
        spec: &CommandSpecification,
        args: &ValueStore,
        defaults: &AddonDefaults,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
    ) -> CommandExecutionFutureResult {
        let spec = spec.clone();
        let args = args.clone();
        let defaults = defaults.clone();

        let future = async move {
            let mut result = CommandExecutionResult::new();
            let call_result = build_view_call(&spec, &args, &defaults).await?;
            result.outputs.insert("result".into(), call_result);

            Ok(result)
        };

        Ok(Box::pin(future))
    }
}

#[cfg(not(feature = "wasm"))]
async fn build_view_call(
    _spec: &CommandSpecification,
    args: &ValueStore,
    defaults: &AddonDefaults,
) -> Result<Value, Diagnostic> {
    use alloy::{contract::Interface, json_abi::JsonAbi};

    use crate::{
        codec::{
            build_unsigned_transaction, value_to_sol_value, CommonTransactionFields,
            TransactionType,
        },
        constants::{
            CHAIN_ID, CONTRACT_ABI, CONTRACT_ADDRESS, CONTRACT_FUNCTION_ARGS,
            CONTRACT_FUNCTION_NAME, GAS_LIMIT, NONCE, TRANSACTION_AMOUNT, TRANSACTION_FROM,
            TRANSACTION_TYPE,
        },
    };

    let network_id = args.get_defaulting_string(NETWORK_ID, defaults)?;
    let rpc_api_url = args.get_defaulting_string(RPC_API_URL, &defaults)?;
    let chain_id = args.get_defaulting_uint(CHAIN_ID, &defaults)?;

    let contract_address: &Value = args.get_expected_value(CONTRACT_ADDRESS)?;
    let from = args.get_expected_value(TRANSACTION_FROM)?;
    let contract_abi = args.get_string(CONTRACT_ABI);
    let function_name = args.get_expected_string(CONTRACT_FUNCTION_NAME)?;
    let function_args: Vec<DynSolValue> = args
        .get_value(CONTRACT_FUNCTION_ARGS)
        .map(|v| {
            v.expect_array()
                .iter()
                .map(|v| {
                    value_to_sol_value(&v)
                        .map_err(|e| diagnosed_error!("command 'evm::call_view_function': {}", e))
                })
                .collect::<Result<Vec<DynSolValue>, Diagnostic>>()
        })
        .unwrap_or(Ok(vec![]))?;

    let amount = args
        .get_value(TRANSACTION_AMOUNT)
        .map(|v| v.expect_uint())
        .unwrap_or(0);
    let gas_limit = args.get_value(GAS_LIMIT).map(|v| v.expect_uint());
    let nonce = args.get_value(NONCE).map(|v| v.expect_uint());
    let tx_type = TransactionType::from_some_value(args.get_string(TRANSACTION_TYPE))?;

    let rpc = EVMRpc::new(&rpc_api_url)
        .map_err(|e| diagnosed_error!("command 'evm::call_view_function': {}", e))?;

    let input = if let Some(abi_str) = contract_abi {
        let abi: JsonAbi = serde_json::from_str(&abi_str).map_err(|e| {
            diagnosed_error!("command 'call_view_function': invalid contract abi: {}", e)
        })?;

        let interface = Interface::new(abi);
        interface
            .encode_input(function_name, &function_args)
            .map_err(|e| {
                diagnosed_error!(
                    "command 'call_view_function': failed to encode contract inputs: {e}"
                )
            })?
    } else {
        function_args
            .iter()
            .flat_map(|v| v.abi_encode_params())
            .collect()
    };

    let common = CommonTransactionFields {
        to: Some(contract_address.clone()),
        from: from.clone(),
        nonce,
        chain_id,
        amount: amount,
        gas_limit,
        tx_type,
        input: Some(input),
        deploy_code: None,
    };
    let tx = build_unsigned_transaction(rpc.clone(), args, common)
        .await
        .map_err(|e| diagnosed_error!("command: 'evm::call_view_function': {e}"))?;

    let call_result = rpc
        .call(&tx)
        .await
        .map_err(|e| diagnosed_error!("command 'evm::call_view_function': {}", e))?;

    Ok(Value::string(call_result))
}
