use txtx_addon_kit::{
    types::{
        commands::{
            CommandExecutionFutureResult, CommandImplementation, CommandSpecification,
            PreCommandSpecification,
        },
        diagnostics::Diagnostic,
        frontend::{Actions, BlockEvent},
        types::{RunbookSupervisionContext, Type},
        ConstructDid, ValueStore,
    },
    AddonDefaults,
};

lazy_static! {
    pub static ref DEPLOY_STACKS_CONTRACT: PreCommandSpecification = define_command! {
      StacksDeployContract => {
          name: "Stacks Contract Deployment",
          matcher: "deploy_contract",
          documentation: "Coming soon",
          implements_signing_capability: false,
          implements_background_task_capability: false,
          inputs: [
              clarity_value: {
                  documentation: "",
                  typing: Type::bool(),
                  optional: true,
                  interpolable: true
              }
          ],
          outputs: [
              bytes: {
                  documentation: "",
                  typing: Type::buffer()
              }
          ],
          example: txtx_addon_kit::indoc! {r#"
            // Coming soon
        "#},
      }
    };
}
pub struct StacksDeployContract;

impl CommandImplementation for StacksDeployContract {
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
        _spec: &CommandSpecification,
        _args: &ValueStore,
        _defaults: &AddonDefaults,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
    ) -> CommandExecutionFutureResult {
        unimplemented!()
    }
}
