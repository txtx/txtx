use std::collections::HashMap;

use txtx_addon_kit::{
    types::{
        commands::{
            CommandExecutionContext, CommandExecutionFutureResult, CommandImplementation,
            CommandInstance, CommandSpecification, PreCommandSpecification,
        },
        diagnostics::Diagnostic,
        frontend::ActionItem,
        types::{Type, Value},
        ConstructUuid,
    },
    AddonDefaults,
};

lazy_static! {
    pub static ref DEPLOY_STACKS_CONTRACT: PreCommandSpecification = define_command! {
      StacksDeployContract => {
          name: "Stacks Contract Deployment",
          matcher: "deploy_contract",
          documentation: "Coming soon",
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
        _uuid: &ConstructUuid,
        _spec: &CommandSpecification,
        _args: &HashMap<String, Value>,
        _defaults: &AddonDefaults,
        _execution_context: &CommandExecutionContext,
    ) -> Result<(), ActionItem> {
        unimplemented!()
    }

    fn execute(
        _uuid: &ConstructUuid,
        _spec: &CommandSpecification,
        _args: &HashMap<String, Value>,
        _defaults: &AddonDefaults,
        _progress_tx: &txtx_addon_kit::channel::Sender<(ConstructUuid, Diagnostic)>,
    ) -> CommandExecutionFutureResult {
        unimplemented!()
    }
}
