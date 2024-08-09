use clarity::util::sleep_ms;
use clarity::vm::types::QualifiedContractIdentifier;
use clarity::vm::Value;
use txtx_addon_kit::types::commands::{CommandExecutionFutureResult, PreCommandSpecification};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::types::RunbookSupervisionContext;
use txtx_addon_kit::types::{
    commands::{CommandExecutionResult, CommandImplementation, CommandSpecification},
    diagnostics::Diagnostic,
    types::Type,
};
use txtx_addon_kit::types::{ConstructDid, ValueStore};
use txtx_addon_kit::AddonDefaults;

use crate::constants::{DEFAULT_DEVNET_BACKOFF, DEFAULT_MAINNET_BACKOFF, NETWORK_ID, RPC_API_URL};
use crate::rpc::StacksRpc;
use crate::stacks_helpers::{clarity_value_to_value, parse_clarity_value};
use crate::typing::{STACKS_CV_GENERIC, STACKS_CV_PRINCIPAL};

lazy_static! {
    pub static ref CALL_READONLY_FN: PreCommandSpecification = define_command! {
        CallReadonlyStacksFunction => {
            name: "Call Clarity Read only function",
            matcher: "call_readonly_fn",
            documentation: "The `call_readonly_fn` action queries a public readonly function.",
            implements_signing_capability: false,
            implements_background_task_capability: false,
            inputs: [
                contract_id: {
                    documentation: "The address and identifier of the contract to invoke.",
                    typing: Type::addon(STACKS_CV_PRINCIPAL),
                    optional: false,
                    interpolable: true
                },
                function_name: {
                    documentation: "The contract method to invoke.",
                    typing: Type::string(),
                    optional: false,
                    interpolable: true
                },
                function_args: {
                    documentation: "The function arguments for the contract call.",
                    typing: Type::array(Type::addon(STACKS_CV_GENERIC)),
                    optional: true,
                    interpolable: true
                },
                rpc_api_url: {
                    documentation: "The URL of the Stacks API to broadcast to.",
                    typing: Type::string(),
                    optional: true,
                    interpolable: true
                },
                sender: {
                    documentation: "The simulated tx-sender to use.",
                    typing: Type::string(),
                    optional: true,
                    interpolable: true
                },
                block_height: {
                    documentation: "Coming soon.",
                    typing: Type::integer(),
                    optional: true,
                    interpolable: true
                }
            ],
            outputs: [
                value: {
                    documentation: "The result of the function execution.",
                    typing: Type::string()
                }
            ],
            example: txtx_addon_kit::indoc! {r#"
        "#},
        }
    };
}

pub struct CallReadonlyStacksFunction;
impl CommandImplementation for CallReadonlyStacksFunction {
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
        let args = args.clone();
        let contract_id_arg = args.get_expected_string("contract_id")?;
        let contract_id = QualifiedContractIdentifier::parse(contract_id_arg)
            .map_err(|e| diagnosed_error!("unable to parse contract_id: {}", e.to_string()))?;

        let function_name = args.get_expected_string("function_name")?.to_string();

        let function_args_values = args.get_expected_array("function_args")?.clone();
        let mut function_args = vec![];
        for arg_value in function_args_values.iter() {
            let Some(data) = arg_value.as_addon_data() else {
                return Err(diagnosed_error!(
                    "function '{}': expected array, got {:?}",
                    spec.matcher,
                    arg_value
                ));
            };
            let arg = parse_clarity_value(&data.bytes, &data.id)?;
            function_args.push(arg);
        }

        let rpc_api_url = args.get_defaulting_string(RPC_API_URL, defaults)?;
        let network_id = match args.get_defaulting_string(NETWORK_ID, &defaults) {
            Ok(value) => value,
            Err(diag) => return Err(diag),
        };

        let sender = args
            .get_expected_string("sender")
            .map(|s| s.to_string())
            .unwrap_or(contract_id.issuer.to_address());

        #[cfg(not(feature = "wasm"))]
        let future = async move {
            let mut result = CommandExecutionResult::new();

            let backoff_ms = if network_id.eq("devnet") {
                DEFAULT_DEVNET_BACKOFF
            } else {
                DEFAULT_MAINNET_BACKOFF
            };

            let client = StacksRpc::new(&rpc_api_url);
            let mut retry_count = 4;
            let call_result = loop {
                // if block_height provided, retrieve and provide block hash in the subsequent request

                match client
                    .call_readonly_fn_fn(
                        &contract_id.issuer.to_address(),
                        &contract_id.name.to_string(),
                        &function_name,
                        function_args.clone(),
                        &sender,
                    )
                    .await
                {
                    Ok(res) => break res,
                    Err(e) => {
                        retry_count -= 1;
                        sleep_ms(backoff_ms);
                        if retry_count > 0 {
                            continue;
                        }

                        return Err(Diagnostic::error_from_string(format!(
                            "Failed to call readonly function: {e}"
                        )));
                    }
                }
            };

            if let Value::Response(ref response) = call_result {
                if !response.committed {
                    let args = function_args
                        .iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    return Err(diagnosed_error!(
                        "Contract-call {}::{}({}) failed with error {}.",
                        contract_id,
                        function_name,
                        args,
                        response.data.to_string()
                    ));
                }
            }

            let value = clarity_value_to_value(call_result)?;
            result.outputs.insert("value".into(), value);

            Ok(result)
        };
        #[cfg(feature = "wasm")]
        panic!("async commands are not enabled for wasm");
        #[cfg(not(feature = "wasm"))]
        Ok(Box::pin(future))
    }
}
