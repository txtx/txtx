use std::collections::HashMap;

use txtx_addon_kit::{
    define_command,
    types::{
        commands::{
            CommandExecutionResult, CommandImplementation, CommandInputsEvaluationResult,
            CommandSpecification,
        },
        diagnostics::Diagnostic,
        types::{PrimitiveValue, Type, Value},
    },
};

pub fn new_module_specification() -> CommandSpecification {
    let mut command = define_command! {
        Module => {
            name: "Module",
            matcher: "module",
            documentation: "Read Construct attribute",
            inputs: [],
            outputs: [],
        }
    };
    command.accepts_arbitrary_inputs = true;
    command.create_output_for_each_input = true;
    command
}

pub struct Module;
impl CommandImplementation for Module {
    fn check(_ctx: &CommandSpecification, _args: Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _ctx: &CommandSpecification,
        _args: &HashMap<String, Value>,
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

pub fn new_variable_specification() -> CommandSpecification {
    let command: CommandSpecification = define_command! {
        Variable => {
            name: "Variable",
            matcher: "variable",
            documentation: "Construct designed to store a variable",
            inputs: [
                description: {
                    documentation: "Description of the variable",
                    typing: Type::string(),
                    optional: true,
                    interpolable: true
                },
                value: {
                    documentation: "Value of the variable",
                    typing: Type::string(),
                    optional: true,
                    interpolable: true
                },
                default: {
                    documentation: "Default value of the variable, if value is omitted",
                    typing: Type::string(),
                    optional: true,
                    interpolable: true
                },
                type: {
                    documentation: "The type of the variable output. Can be inferred from `value` or `default` if provided.",
                    typing: Type::string(),
                    optional: true,
                    interpolable: true
                }
            ],
            outputs: [
                value: {
                    documentation: "Value of the variable",
                    typing: Type::string()
                }
            ],
        }
    };
    command
}

pub struct Variable;
impl CommandImplementation for Variable {
    fn check(_ctx: &CommandSpecification, _args: Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _ctx: &CommandSpecification,
        args: &HashMap<String, Value>,
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
        input_name: String,
        value: String,
    ) {
        let (input_key, value) = match input_name.as_str() {
            "value" => {
                let value_input = ctx
                    .inputs
                    .iter()
                    .find(|i| i.name == "value")
                    .expect("Variable specification must have value input");
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
                (value_input, value)
            }
            "default" => {
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
                (default_input, value)
            }
            "description" => {
                let description_input = ctx
                    .inputs
                    .iter()
                    .find(|i| i.name == "description")
                    .expect("Variable specification must have description input");

                let expected_type = description_input.typing.clone();
                let value = if value.is_empty() {
                    None
                } else {
                    Some(Value::from_string(value, expected_type, None))
                };
                (description_input, value)
            }
            _ => unimplemented!("cannot parse serialized output for input {input_name}"),
        };
        match value {
            Some(value) => current_input_evaluation_result
                .inputs
                .insert(input_key.clone(), value),
            None => current_input_evaluation_result.inputs.remove(&input_key),
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
                description: {
                    documentation: "Description of the output",
                    typing: Type::string(),
                    optional: true,
                    interpolable: true
                },
                value: {
                    documentation: "Value of the variable",
                    typing: Type::string(),
                    optional: true,
                    interpolable: true
                }
            ],
            outputs: [
                value: {
                    documentation: "Value of the variable",
                    typing: Type::string()
                }
            ],
        }
    };
    command
}

pub struct Output;
impl CommandImplementation for Output {
    fn check(_ctx: &CommandSpecification, _args: Vec<Type>) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn run(
        _ctx: &CommandSpecification,
        args: &HashMap<String, Value>,
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
