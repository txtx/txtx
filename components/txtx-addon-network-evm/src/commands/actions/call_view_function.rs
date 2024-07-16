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
                documentation: "The path to the contract ABI in the local filesystem, or a URL to download it.",
                typing: Type::addon(ETH_ADDRESS.clone()),
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
              signed_transaction_bytes: {
                  documentation: "The signed transaction bytes.",
                  typing: Type::string()
              },
              network_id: {
                  documentation: "Network id of the signed transaction.",
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
            println!("call result: {:?}", call_result);
            result
                .outputs
                .insert("value".into(), Value::Array(Box::new(call_result)));

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
) -> Result<Vec<Value>, Diagnostic> {
    use crate::{
        codec::{get_contract_abi, sol_value_to_value, value_to_sol_value},
        commands::actions::get_expected_address,
        constants::{
            CONTRACT_ABI, CONTRACT_ADDRESS, CONTRACT_FUNCTION_ARGS, CONTRACT_FUNCTION_NAME,
        },
    };

    let network_id = args.get_defaulting_string(NETWORK_ID, defaults)?;
    let rpc_api_url = args.get_defaulting_string(RPC_API_URL, &defaults)?;

    let contract_address = args.get_expected_value(CONTRACT_ADDRESS)?;
    let contract_abi_loc = args.get_expected_string(CONTRACT_ABI)?;
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

    let rpc = EVMRpc::new(&rpc_api_url)
        .map_err(|e| diagnosed_error!("command 'evm::call_view_function': {}", e))?;

    let abi = get_contract_abi(contract_abi_loc)
        .await
        .map_err(|e| diagnosed_error!("command 'evm::call_view_function': {}", e))?;
    let contract_address = get_expected_address(&contract_address).map_err(|e| {
        diagnosed_error!("command 'evm::call_view_function': failed to parse to address: {e}")
    })?;

    let call_result = rpc
        .call(abi, &contract_address, function_name, &function_args)
        .await
        .map_err(|e| diagnosed_error!("command 'evm::call_view_function': {}", e))?
        .iter()
        .map(|s| sol_value_to_value(s))
        .collect::<Result<Vec<Value>, String>>()
        .map_err(|e| diagnosed_error!("command 'evm::call_view_function': {}", e))?;

    Ok(call_result)
}
