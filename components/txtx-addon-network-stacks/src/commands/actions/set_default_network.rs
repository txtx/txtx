use std::collections::HashMap;
use txtx_addon_kit::types::commands::{CommandImplementation, PreCommandSpecification};
use txtx_addon_kit::types::{
    commands::{CommandExecutionResult, CommandInputsEvaluationResult, CommandSpecification},
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::AddonDefaults;

lazy_static! {
    pub static ref SET_DEFAULT_NETWORK: PreCommandSpecification = {
        let mut command = define_command! {
            SetStacksGlobals => {
            name: "Configure Stacks Network",
            matcher: "set_default_network",
            documentation: "Configure Stacks Network.",
            inputs: [
                network_id: {
                    documentation: "Network to use (mainnet, testnet).",
                    typing: Type::string(),
                    optional: false,
                    interpolable: true
                },
                stacks_api_url: {
                    documentation: "Stacks API RPC URL.",
                    typing: Type::string(),
                    optional: false,
                    interpolable: true
                }
            ],
            outputs: [],
            }
        };
        if let PreCommandSpecification::Atomic(ref mut spec) = command {
            spec.update_addon_defaults = true;
        }
        command
    };
}

pub struct SetStacksGlobals;
impl CommandImplementation for SetStacksGlobals {
    fn check(_ctx: &CommandSpecification, _args: Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _ctx: &CommandSpecification,
        args: &HashMap<String, Value>,
        _defaults: &AddonDefaults,
    ) -> Result<CommandExecutionResult, Diagnostic> {
        let mut result = CommandExecutionResult::new();

        let stacks_network = args.get("network_id").unwrap().expect_string();
        let stacks_api_url = args.get("stacks_api_url").unwrap().expect_string();

        result.outputs.insert(
            "network_id".to_string(),
            Value::string(stacks_network.into()),
        );

        result.outputs.insert(
            "stacks_api_url".to_string(),
            Value::string(stacks_api_url.into()),
        );
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
