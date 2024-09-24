use clarity::util::sleep_ms;
use clarity::vm::types::QualifiedContractIdentifier;
use clarity::vm::Value;
use txtx_addon_kit::types::commands::{CommandExecutionFutureResult, PreCommandSpecification};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::RunbookSupervisionContext;
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::types::{
    commands::{CommandExecutionResult, CommandImplementation, CommandSpecification},
    diagnostics::Diagnostic,
    types::Type,
};

use crate::codec::cv::{cv_to_value, decode_cv_bytes};
use crate::constants::{
    DEFAULT_DEVNET_BACKOFF, DEFAULT_MAINNET_BACKOFF, NETWORK_ID, RPC_API_AUTH_TOKEN, RPC_API_URL,
};
use crate::rpc::StacksRpc;
use crate::typing::{STACKS_CV_GENERIC, STACKS_CV_PRINCIPAL};

lazy_static! {
    pub static ref CALL_READONLY_FN: PreCommandSpecification = define_command! {
        CallReadonlyStacksFunction => {
            name: "Call a Clarity Read Only Function",
            matcher: "call_readonly_fn",
            documentation: "The `stacks::call_readonly_fn` action queries a public readonly function.",
            implements_signing_capability: false,
            implements_background_task_capability: false,
            inputs: [
                contract_id: {
                    documentation: "The address and identifier of the contract to invoke.",
                    typing: Type::addon(STACKS_CV_PRINCIPAL),
                    optional: false,
                    tainting: true,
                    internal: false
                },
                function_name: {
                    documentation: "The contract method to invoke.",
                    typing: Type::string(),
                    optional: false,
                    tainting: true,
                    internal: false
                },
                function_args: {
                    documentation: "The function arguments for the contract call.",
                    typing: Type::array(Type::addon(STACKS_CV_GENERIC)),
                    optional: true,
                    tainting: true,
                    internal: false
                },
                network_id: {
                    documentation: indoc!{r#"The network id. Can be `"mainnet"`, `"testnet"` or `"devnet"`."#},
                    typing: Type::string(),
                    optional: false,
                    tainting: true,
                    internal: false
                },
                rpc_api_url: {
                    documentation: "The URL to use when making API requests.",
                    typing: Type::string(),
                    optional: false,
                    tainting: true,
                    internal: false
                },
                rpc_api_auth_token: {
                    documentation: "The HTTP authentication token to include in the headers when making API requests.",
                    typing: Type::string(),
                    optional: true,
                    tainting: true,
                    internal: false
                },
                sender: {
                    documentation: "The simulated tx-sender address to use.",
                    typing: Type::string(),
                    optional: true,
                    tainting: true,
                    internal: false
                },
                block_height: {
                    documentation: "Coming soon.",
                    typing: Type::integer(),
                    optional: true,
                    tainting: true,
                    internal: false
                }
            ],
            outputs: [
                value: {
                    documentation: "The result of the function execution.",
                    typing: Type::string()
                }
            ],
            example: txtx_addon_kit::indoc! {r#"
            action "get_name_price" "stacks::call_readonly_fn" {
                description = "Get price for bns name"
                contract_id = "ST000000000000000000002AMW42H.bns"
                function_name = "get-name-price"
                function_args = [
                    stacks::cv_buff(encode_hex("btc")), // namespace
                    stacks::cv_buff(encode_hex("test")) // name
                ]
                sender = "ST2JHG361ZXG51QTKY2NQCVBPPRRE2KZB1HR05NNC"
            }
            output "name_price" {
                value = action.get_name_price
            }
            // > name_price: 100
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
        _values: &ValueStore,
        _supervision_context: &RunbookSupervisionContext,
    ) -> Result<Actions, Diagnostic> {
        Ok(Actions::none()) // todo
    }

    #[cfg(not(feature = "wasm"))]
    fn run_execution(
        _construct_id: &ConstructDid,
        spec: &CommandSpecification,
        values: &ValueStore,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
    ) -> CommandExecutionFutureResult {
        let args = values.clone();
        let contract_id_arg = args.get_expected_string("contract_id")?;
        let contract_id = QualifiedContractIdentifier::parse(contract_id_arg)
            .map_err(|e| diagnosed_error!("unable to parse contract_id: {}", e.to_string()))?;

        let function_name = args.get_expected_string("function_name")?.to_string();

        let empty_array = vec![];
        let function_args_values = args.get_expected_array("function_args").unwrap_or(&empty_array);
        let mut function_args = vec![];
        for arg_value in function_args_values.iter() {
            let Some(data) = arg_value.as_addon_data() else {
                return Err(diagnosed_error!(
                    "function '{}': expected array, got {:?}",
                    spec.matcher,
                    arg_value
                ));
            };
            let arg = decode_cv_bytes(&data.bytes).map_err(|e| diagnosed_error!("{e}"))?;
            function_args.push(arg);
        }

        let rpc_api_url = args.get_expected_string(RPC_API_URL)?.to_owned();
        let rpc_api_auth_token =
            args.get_string(RPC_API_AUTH_TOKEN).and_then(|t| Some(t.to_owned()));
        let network_id = args.get_expected_string(NETWORK_ID)?.to_owned();

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

            let client = StacksRpc::new(&rpc_api_url, &rpc_api_auth_token);
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
                    let args =
                        function_args.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", ");
                    return Err(diagnosed_error!(
                        "Contract-call {}::{}({}) failed with error {}.",
                        contract_id,
                        function_name,
                        args,
                        response.data.to_string()
                    ));
                }
            }

            let value = cv_to_value(call_result)?;
            result.outputs.insert("value".into(), value);

            Ok(result)
        };
        Ok(Box::pin(future))
    }
}
