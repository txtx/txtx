use std::collections::HashMap;

use txtx_addon_kit::{
    define_command,
    types::{
        commands::{CommandExecutionResult, CommandImplementation, CommandSpecification},
        typing::{Typing, Value},
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
    fn check(_ctx: &CommandSpecification, _args: Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(_ctx: &CommandSpecification, _args: &HashMap<String, Value>) -> CommandExecutionResult {
        let result = CommandExecutionResult::new();
        result
    }
}

pub fn new_variable_specification() -> CommandSpecification {
    let command = define_command! {
        Variable => {
            name: "Variable",
            matcher: "variable",
            documentation: "Construct designed to store a variable",
            inputs: [
                description: {
                    documentation: "Description of the variable",
                    typing: Typing::String,
                    optional: true,
                    interpolable: true
                },
                value: {
                    documentation: "Value of the variable",
                    typing: Typing::String,
                    optional: true,
                    interpolable: true
                },
                default: {
                    documentation: "Default value of the variable, if value is omitted",
                    typing: Typing::String,
                    optional: true,
                    interpolable: true
                }
            ],
            outputs: [
                value: {
                    documentation: "Value of the variable",
                    typing: Typing::String
                }
            ],
        }
    };
    command
}

pub struct Variable;
impl CommandImplementation for Variable {
    fn check(_ctx: &CommandSpecification, _args: Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(_ctx: &CommandSpecification, args: &HashMap<String, Value>) -> CommandExecutionResult {
        let value = args.get("value").unwrap().clone(); // todo(lgalabru): get default, etc.
        let mut result = CommandExecutionResult::new();
        result.outputs.insert("value".to_string(), value);
        result
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
                    typing: Typing::String,
                    optional: true,
                    interpolable: true
                },
                value: {
                    documentation: "Value of the variable",
                    typing: Typing::String,
                    optional: true,
                    interpolable: true
                }
            ],
            outputs: [
                value: {
                    documentation: "Value of the variable",
                    typing: Typing::String
                }
            ],
        }
    };
    command
}

pub struct Output;
impl CommandImplementation for Output {
    fn check(_ctx: &CommandSpecification, _args: Vec<Typing>) -> Typing {
        unimplemented!()
    }

    fn run(_ctx: &CommandSpecification, args: &HashMap<String, Value>) -> CommandExecutionResult {
        let value = args.get("value").unwrap().clone(); // todo(lgalabru): get default, etc.
        let mut result = CommandExecutionResult::new();
        result.outputs.insert("value".to_string(), value);
        result
    }
}
