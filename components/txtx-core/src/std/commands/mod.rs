pub mod actions;

use std::collections::HashMap;
use txtx_addon_kit::{
    define_command,
    types::{
        commands::{
            CommandExecutionResult, CommandImplementation, CommandInputsEvaluationResult,
            CommandSpecification, PreCommandSpecification,
        },
        diagnostics::Diagnostic,
        types::{PrimitiveValue, Type, Value},
        wallets::WalletSpecification,
    },
    AddonDefaults,
};

pub fn new_module_specification() -> CommandSpecification {
    let command = define_command! {
        Module => {
            name: "Module",
            matcher: "module",
            documentation: "Read Construct attribute",
            inputs: [],
            outputs: [],
            example: "",
        }
    };
    match command {
        PreCommandSpecification::Atomic(mut command) => {
            command.accepts_arbitrary_inputs = true;
            command.create_output_for_each_input = true;
            command
        }
        PreCommandSpecification::Composite(_) => panic!("module must not be composite"),
    }
}

pub struct Module;
impl CommandImplementation for Module {
    fn check(_ctx: &CommandSpecification, _args: Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _ctx: &CommandSpecification,
        _args: &HashMap<String, Value>,
        _defaults: &AddonDefaults,
        _wallets: &HashMap<String, WalletSpecification>,
    ) -> Result<CommandExecutionResult, Diagnostic> {
        let result = CommandExecutionResult::new();
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

pub fn new_input_specification() -> CommandSpecification {
    let command: PreCommandSpecification = define_command! {
        Input => {
            name: "Input",
            matcher: "input",
            documentation: "Construct designed to store an input",
            inputs: [
                value: {
                    documentation: "Value of the input",
                    typing: Type::string(),
                    optional: true,
                    interpolable: true
                },
                default: {
                    documentation: "Default value of the input, if value is omitted",
                    typing: Type::string(),
                    optional: true,
                    interpolable: true
                },
                description: {
                    documentation: "Description of the input",
                    typing: Type::string(),
                    optional: true,
                    interpolable: true
                },
                type: {
                    documentation: "The type of the input output. Can be inferred from `value` or `default` if provided.",
                    typing: Type::string(),
                    optional: true,
                    interpolable: true
                }
            ],
            outputs: [
                value: {
                    documentation: "Value of the input",
                    typing: Type::string()
                }
            ],
            example: "",
        }
    };
    match command {
        PreCommandSpecification::Atomic(command) => command,
        PreCommandSpecification::Composite(_) => {
            panic!("input should not be composite command specification")
        }
    }
}

pub struct Input;
impl CommandImplementation for Input {
    fn check(_ctx: &CommandSpecification, _args: Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _ctx: &CommandSpecification,
        args: &HashMap<String, Value>,
        _defaults: &AddonDefaults,
        _wallets: &HashMap<String, WalletSpecification>,
    ) -> Result<CommandExecutionResult, Diagnostic> {
        let mut result = CommandExecutionResult::new();
        if let Some(value) = args.get("value") {
            result.outputs.insert("value".to_string(), value.clone());
        } else if let Some(default) = args.get("default") {
            result.outputs.insert("value".to_string(), default.clone());
        }
        Ok(result)
    }

    fn update_input_evaluation_results_from_user_input(
        ctx: &CommandSpecification,
        current_input_evaluation_result: &mut CommandInputsEvaluationResult,
        _input_name: String,
        value: String,
    ) {
        let default_input = ctx
            .inputs
            .iter()
            .find(|i| i.name == "default")
            .expect("Variable specification must have default input");
        let value = if value.is_empty() {
            None
        } else {
            let type_casted_value = match current_input_evaluation_result
                .inputs
                .iter()
                .find(|(i, _)| i.name == "type")
            {
                Some((_, expected_type)) => match expected_type {
                    Err(e) => Err(e.clone()),
                    Ok(Value::Primitive(PrimitiveValue::String(expected_type))) => {
                        Value::from_string(value, Type::from(expected_type.clone()), None)
                    }
                    _ => Value::from_string(value, Type::default(), None),
                },
                None => unimplemented!("no type"), // todo
            };
            Some(type_casted_value)
        };

        match value {
            Some(value) => current_input_evaluation_result
                .inputs
                .insert(default_input.clone(), value),
            None => current_input_evaluation_result
                .inputs
                .remove(&default_input),
        };
    }
}

pub fn new_output_specification() -> CommandSpecification {
    let command = define_command! {
        Output => {
            name: "Output",
            matcher: "output",
            documentation: "Read Construct attribute",
            inputs: [
                value: {
                    documentation: "Value of the output",
                    typing: Type::string(),
                    optional: true,
                    interpolable: true
                }
            ],
            outputs: [
                value: {
                    documentation: "Value of the output",
                    typing: Type::string()
                }
            ],
            example: "",
        }
    };
    match command {
        PreCommandSpecification::Atomic(command) => command,
        PreCommandSpecification::Composite(_) => {
            panic!("output should not be composite command specification")
        }
    }
}

pub struct Output;
impl CommandImplementation for Output {
    fn check(_ctx: &CommandSpecification, _args: Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _ctx: &CommandSpecification,
        args: &HashMap<String, Value>,
        _defaults: &AddonDefaults,
        _wallets: &HashMap<String, WalletSpecification>,
    ) -> Result<CommandExecutionResult, Diagnostic> {
        let value = args.get("value").unwrap().clone(); // todo(lgalabru): get default, etc.
        let mut result = CommandExecutionResult::new();
        result.outputs.insert("value".to_string(), value);
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
