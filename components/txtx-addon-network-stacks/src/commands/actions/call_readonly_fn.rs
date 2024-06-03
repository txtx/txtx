use clarity::util::sleep_ms;
use clarity::vm::types::QualifiedContractIdentifier;
use txtx_addon_kit::types::commands::{
    CommandExecutionContext, CommandExecutionFutureResult, PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent, ProgressBarStatus};
use txtx_addon_kit::types::types::Value;
use txtx_addon_kit::types::{
    commands::{CommandExecutionResult, CommandImplementation, CommandSpecification},
    diagnostics::Diagnostic,
    types::Type,
};
use txtx_addon_kit::types::{ConstructUuid, ValueStore};
use txtx_addon_kit::uuid::Uuid;
use txtx_addon_kit::AddonDefaults;

use crate::constants::RPC_API_URL;
use crate::rpc::StacksRpc;
use crate::typing::{CLARITY_PRINCIPAL, CLARITY_VALUE};

lazy_static! {
    pub static ref CALL_READONLY_FN: PreCommandSpecification = define_command! {
        BroadcastStacksTransaction => {
            name: "Call Clarity Read only function",
            matcher: "call_readonly_fn",
            documentation: "The `call_readonly_fn` action queries public functions.",
            requires_signing_capability: false,
            inputs: [
                contract_id: {
                    documentation: "Address and identifier of the contract to invoke",
                    typing: Type::addon(CLARITY_PRINCIPAL.clone()),
                    optional: false,
                    interpolable: true
                },
                function_name: {
                    documentation: "Method to invoke",
                    typing: Type::string(),
                    optional: false,
                    interpolable: true
                },
                function_args: {
                    documentation: "Args to provide",
                    typing: Type::array(Type::addon(CLARITY_VALUE.clone())),
                    optional: true,
                    interpolable: true
                },
                rpc_api_url: {
                    documentation: "The URL of the Stacks API to broadcast to.",
                    typing: Type::string(),
                    optional: false,
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

pub struct BroadcastStacksTransaction;
impl CommandImplementation for BroadcastStacksTransaction {
    fn check_instantiability(
        _ctx: &CommandSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn check_executability(
        _uuid: &ConstructUuid,
        _instance_name: &str,
        _spec: &CommandSpecification,
        _args: &ValueStore,
        _defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> Result<Actions, Diagnostic> {
        Ok(Actions::none()) // todo
    }

    fn run_execution(
        _uuid: &ConstructUuid,
        _spec: &CommandSpecification,
        args: &ValueStore,
        defaults: &AddonDefaults,
        progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
    ) -> CommandExecutionFutureResult {
        let args = args.clone();
        let contract_id_arg = args.get_expected_string("contract_id")?;
        let contract_id = QualifiedContractIdentifier::parse(contract_id_arg)
            .map_err(|e| diagnosed_error!("unable to parse contract_id: {}", e.to_string()))?;
        let function_name = args.get_expected_string("function_name")?.to_string();
        let function_args = args.get_expected_array("function_args")?.clone();
        let rpc_api_url = args.get_defaulting_string(RPC_API_URL, defaults)?;

        let progress_tx = progress_tx.clone();

        let future = async move {
            let mut progress_bar = ProgressBarStatus {
                uuid: Uuid::new_v4(),
                visible: true,
                status: "status".to_string(),
                message: "message".to_string(),
                diagnostic: None,
            };
            let _ = progress_tx.send(BlockEvent::ProgressBar(progress_bar.clone()));

            let mut result = CommandExecutionResult::new();

            let backoff_ms = 5000;

            let client = StacksRpc::new(&rpc_api_url);
            let mut retry_count = 4;
            let call_result = loop {
                match client
                    .call_readonly_fn_fn(
                        &contract_id.issuer.to_address(),
                        &contract_id.name.to_string(),
                        &function_name,
                        vec![],
                        &contract_id.issuer.to_address(),
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

            println!("{:?}", call_result);

            result
                .outputs
                .insert("value".into(), Value::string("todo".into()));

            progress_bar.visible = false;
            let _ = progress_tx.send(BlockEvent::ProgressBar(progress_bar));

            Ok(result)
        };
        Ok(Box::pin(future))
    }
}
