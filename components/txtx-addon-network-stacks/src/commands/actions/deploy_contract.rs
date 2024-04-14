use std::collections::HashMap;

use txtx_addon_kit::types::{
    commands::{
        CommandExecutionResult, CommandImplementation, CommandInputsEvaluationResult,
        CommandSpecification,
    },
    diagnostics::Diagnostic,
    types::{Type, Value},
};

lazy_static! {
    pub static ref DEPLOY_STACKS_CONTRACT: CommandSpecification = define_command! {
      StacksDeployContract => {
          name: "Stacks Contract Deployment",
          matcher: "deploy_contract",
          documentation: "Encode contract deployment payload",
          inputs: [
              clarity_value: {
                  documentation: "Any valid Clarity value",
                  typing: Type::bool(),
                  optional: true,
                  interpolable: true
              }
          ],
          outputs: [
              bytes: {
                  documentation: "Encoded contract call",
                  typing: Type::buffer()
              }
          ],
      }
    };
}
pub struct StacksDeployContract;
impl CommandImplementation for StacksDeployContract {
    fn check(_ctx: &CommandSpecification, _args: Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _ctx: &CommandSpecification,
        args: &HashMap<String, Value>,
    ) -> Result<CommandExecutionResult, Diagnostic> {
        let value = args.get("clarity_value").unwrap().clone(); // todo(lgalabru): get default, etc.
        let mut result = CommandExecutionResult::new();
        result.outputs.insert("bytes".to_string(), value);
        Ok(result)
    }

    fn update_input_evaluation_results_from_user_input(
        _ctx: &CommandSpecification,
        _current_input_evaluation_result: &mut CommandInputsEvaluationResult,
        _input_name: String,
        _value: String,
    ) {
        todo!()
    }
}
