pub mod actions;

use std::collections::HashMap;
use txtx_addon_kit::{
    define_command,
    types::{
        commands::{
            CommandExecutionResult, CommandImplementation, CommandInstance, CommandSpecification,
            PreCommandSpecification,
        },
        diagnostics::Diagnostic,
        frontend::{ActionItem, ActionItemStatus, ActionItemType, ProvideInputContext},
        types::{PrimitiveType, PrimitiveValue, Type, Value},
        ConstructUuid,
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
        let result = CommandExecutionResult::new();
        Ok(result)
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

    fn get_action(
        _ctx: &CommandSpecification,
        args: &HashMap<String, Value>,
        _defaults: &AddonDefaults,
        uuid: &ConstructUuid,
        index: u16,
        instance: &CommandInstance,
    ) -> Option<ActionItem> {
        if let Some(value) = args.get("value") {
            return Some(ActionItem::new(
                &uuid.value(),
                index,
                &instance.name,
                &value.to_string(),
                ActionItemStatus::Todo,
                ActionItemType::ReviewInput,
            ));
        } else if let Some(default) = args.get("default") {
            return Some(ActionItem::new(
                &uuid.value(),
                index,
                &instance.name,
                &default.to_string(),
                ActionItemStatus::Todo,
                ActionItemType::ProvideInput,
            ));
        }
        None
    }

    fn run(
        _ctx: &CommandSpecification,
        args: &HashMap<String, Value>,
        _defaults: &AddonDefaults,
    ) -> Result<CommandExecutionResult, Diagnostic> {
        let mut result = CommandExecutionResult::new();
        if let Some(value) = args.get("value") {
            result.outputs.insert("value".to_string(), value.clone());
        } else if let Some(default) = args.get("default") {
            result.outputs.insert("value".to_string(), default.clone());
        }
        Ok(result)
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

    fn get_action(
        _ctx: &CommandSpecification,
        args: &HashMap<String, Value>,
        _defaults: &AddonDefaults,
        uuid: &ConstructUuid,
        index: u16,
        instance: &CommandInstance,
    ) -> Option<ActionItem> {
        if let Some(value) = args.get("value") {
            return Some(ActionItem {
                uuid: uuid.value().clone(),
                index,
                title: instance.name.clone(),
                description: value.expect_string().to_string(),
                action_status: ActionItemStatus::Todo,
                action_type: ActionItemType::ReviewInput,
            });
        }
        None
    }

    fn run(
        _ctx: &CommandSpecification,
        args: &HashMap<String, Value>,
        _defaults: &AddonDefaults,
    ) -> Result<CommandExecutionResult, Diagnostic> {
        let value = args.get("value").unwrap().clone(); // todo(lgalabru): get default, etc.
        let mut result = CommandExecutionResult::new();
        result.outputs.insert("value".to_string(), value);
        Ok(result)
    }
}
