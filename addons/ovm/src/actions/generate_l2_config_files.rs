use txtx_addon_kit::types::commands::{
    CommandExecutionFutureResult, CommandExecutionResult, CommandImplementation,
    PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::{commands::CommandSpecification, diagnostics::Diagnostic, types::Type};
use txtx_addon_kit::types::{types::RunbookSupervisionContext, ConstructDid};
use txtx_addon_kit::uuid::Uuid;

lazy_static! {
    pub static ref GENERATE_L2_CONFIG_FILES: PreCommandSpecification = define_command! {
        GenerateL2ConfigFiles => {
            name: "Coming Soon",
            matcher: "generate_l2_config_files",
            documentation: "The `ovm::generate_l2_config_files` action takes some L2 settings and deployment addresses of the L1 contracts and generates the `genesis.json` and `rollup.json` files needed to start a L2 OP node.",
            implements_signing_capability: false,
            implements_background_task_capability: true,
            inputs: [
                l1_rpc_api_url: {
                    documentation: "The URL of the L1 EVM API used to fetch data.",
                    typing: Type::string(),
                    optional: false,
                    tainting: false,
                    internal: false
                },
                deployment_config: {
                    documentation: "Coming soon.",
                    typing: Type::object(vec![]),
                    optional: false,
                    tainting: false,
                    internal: false
                },
                l1_deployment_addresses: {
                    documentation: "Coming soon.",
                    typing: Type::object(vec![]),
                    optional: false,
                    tainting: false,
                    internal: false
                }
            ],
            outputs: [
                genesis: {
                    documentation: "The genesis.json settings.",
                    typing: Type::object(vec![])
                },
                rollup: {
                    documentation: "The rollup.json settings.",
                    typing: Type::object(vec![])
                }
            ],
            example: txtx_addon_kit::indoc! {r#"
                // Coming soon
            "#},
        }
    };
}

pub struct GenerateL2ConfigFiles;
impl CommandImplementation for GenerateL2ConfigFiles {
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
        Ok(Actions::none())
    }

    #[cfg(not(feature = "wasm"))]
    fn run_execution(
        _construct_id: &ConstructDid,
        _spec: &CommandSpecification,
        _values: &ValueStore,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
    ) -> CommandExecutionFutureResult {
        let future = async move {
            let result = CommandExecutionResult::new();
            Ok(result)
        };

        Ok(Box::pin(future))
    }

    fn build_background_task(
        construct_did: &ConstructDid,
        spec: &CommandSpecification,
        inputs: &ValueStore,
        outputs: &ValueStore,
        progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
        background_tasks_uuid: &Uuid,
        supervision_context: &RunbookSupervisionContext,
    ) -> CommandExecutionFutureResult {
        let future = async move {
            let result = CommandExecutionResult::new();

            Ok(result)
        };
        Ok(Box::pin(future))
    }
}
