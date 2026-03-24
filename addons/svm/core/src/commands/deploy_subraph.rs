use std::vec;

use txtx_addon_kit::types::commands::{
    CommandExecutionFutureResult, CommandExecutionResult, CommandImplementation,
    CommandSpecification, PreCommandSpecification,
};
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::types::frontend::{Actions, BlockEvent, LogDispatcher};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::{RunbookSupervisionContext, Type};
use txtx_addon_kit::types::ConstructDid;

lazy_static! {
    pub static ref DEPLOY_SUBGRAPH: PreCommandSpecification = {
        let command = define_command! {
            DeployProgram => {
                name: "Deploy SVM Program Subgraph",
                matcher: "deploy_subgraph",
                documentation: "`svm::deploy_subgraph` is deprecated. If you are using this in your runbook, it should be removed.",
                implements_signing_capability: false,
                implements_background_task_capability: false,
                inputs: [],
                outputs: [],
                example: "",
            }
        };
        command
    };
}

pub struct DeployProgram;
impl CommandImplementation for DeployProgram {
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
        Ok(Actions::none())
    }

    #[cfg(not(feature = "wasm"))]
    fn run_execution(
        construct_did: &ConstructDid,
        _spec: &CommandSpecification,
        _values: &ValueStore,
        progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
        _auth_ctx: &txtx_addon_kit::types::AuthorizationContext,
    ) -> CommandExecutionFutureResult {
        use txtx_addon_kit::types::commands::return_synchronous_ok;

        let logger =
            LogDispatcher::new(construct_did.as_uuid(), "svm::deploy_subgraph", &progress_tx);

        logger.warn(
            "Deprecated Action",
            "The svm::deploy_subgraph action is deprecated. Please remove from your runbooks.",
        );
        return_synchronous_ok(CommandExecutionResult::new())
    }
}
