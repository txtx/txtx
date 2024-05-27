use std::collections::HashMap;

use txtx_addon_kit::{
    types::{
        commands::{
            CommandExecutionResult, CommandImplementation, CommandInstance, CommandSpecification,
            PreCommandSpecification,
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
    fn check(_ctx: &CommandSpecification, _args: Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }
    fn get_action(
        _ctx: &CommandSpecification,
        _args: &HashMap<String, Value>,
        _defaults: &AddonDefaults,
        _uuid: &ConstructUuid,
        _index: u16,
        _instance: &CommandInstance,
    ) -> Option<ActionItem> {
        todo!()
    }
    fn run(
        _ctx: &CommandSpecification,
        _args: &HashMap<String, Value>,
        _defaults: &AddonDefaults,
    ) -> Result<CommandExecutionResult, Diagnostic> {
        unimplemented!()
    }
}
