pub mod actions;

use kit::types::ValueStore;
use std::collections::HashMap;
use txtx_addon_kit::types::commands::{return_synchronous_result, CommandExecutionContext};
use txtx_addon_kit::types::frontend::{
    ActionItemRequestType, ActionItemStatus, ProvideInputRequest,
};
use txtx_addon_kit::types::wallets::WalletInstance;
use txtx_addon_kit::uuid::Uuid;
use txtx_addon_kit::{
    define_command,
    types::{
        commands::{
            CommandExecutionFutureResult, CommandExecutionResult, CommandImplementation,
            CommandSpecification, PreCommandSpecification,
        },
        diagnostics::Diagnostic,
        frontend::ActionItemRequest,
        types::{Type, Value},
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
    fn check_instantiability(
        _ctx: &CommandSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn check_executability(
        _uuid: &ConstructUuid,
        _instance_name: &str,
        _spec: &CommandSpecification,
        _args: &ValueStore,
        _defaults: &AddonDefaults,
        _wallet_instances: &HashMap<ConstructUuid, WalletInstance>,
        _execution_context: &CommandExecutionContext,
    ) -> Result<Vec<ActionItemRequest>, Diagnostic> {
        unimplemented!()
    }

    fn execute(
        _uuid: &ConstructUuid,
        _spec: &CommandSpecification,
        _args: &ValueStore,
        _defaults: &AddonDefaults,
        _wallet_instances: &HashMap<ConstructUuid, WalletInstance>,
        _progress_tx: &txtx_addon_kit::channel::Sender<(ConstructUuid, Diagnostic)>,
    ) -> CommandExecutionFutureResult {
        let result = CommandExecutionResult::new();
        return_synchronous_result(Ok(result))
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
    fn check_instantiability(
        _ctx: &CommandSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn check_executability(
        uuid: &ConstructUuid,
        instance_name: &str,
        spec: &CommandSpecification,
        args: &ValueStore,
        _defaults: &AddonDefaults,
        _wallet_instances: &HashMap<ConstructUuid, WalletInstance>,
        execution_context: &CommandExecutionContext,
    ) -> Result<Vec<ActionItemRequest>, Diagnostic> {
        if let Some(value) = args.get_value("value") {
            for input_spec in spec.inputs.iter() {
                if input_spec.name == "value" && input_spec.check_performed {
                    return Ok(vec![]);
                }
            }
            if execution_context.review_input_values {
                return Ok(vec![ActionItemRequest::new(
                    &Uuid::new_v4(),
                    &Some(uuid.value()),
                    0,
                    &instance_name,
                    &value.to_string(),
                    ActionItemStatus::Todo,
                    ActionItemRequestType::ReviewInput,
                )]);
            } else {
                return Ok(vec![]);
            }
        }

        let (default_value, typing) = match args.get_value("default") {
            Some(default_value) => {
                for input_spec in spec.inputs.iter() {
                    if input_spec.name == "value" && input_spec.check_performed {
                        return Ok(vec![]);
                    }
                }
                (
                    Some(default_value.to_string()),
                    default_value.expect_primitive().get_type(),
                )
            }
            None => {
                let typing = args.get_expected_value("type")?;
                (None, serde_json::de::from_str(&typing.to_string()).unwrap())
            }
        };

        return Ok(vec![ActionItemRequest::new(
            &Uuid::new_v4(),
            &Some(uuid.value()),
            0,
            &instance_name,
            &default_value.unwrap_or("".into()),
            ActionItemStatus::Todo,
            ActionItemRequestType::ProvideInput(ProvideInputRequest {
                input_name: instance_name.to_string(),
                typing: typing,
            }),
        )]);
    }

    fn execute(
        _uuid: &ConstructUuid,
        _spec: &CommandSpecification,
        args: &ValueStore,
        _defaults: &AddonDefaults,
        _wallet_instances: &HashMap<ConstructUuid, WalletInstance>,
        _progress_tx: &txtx_addon_kit::channel::Sender<(ConstructUuid, Diagnostic)>,
    ) -> CommandExecutionFutureResult {
        let mut result = CommandExecutionResult::new();
        if let Some(value) = args.get_value("value") {
            result.outputs.insert("value".to_string(), value.clone());
        } else if let Some(default) = args.get_value("default") {
            result.outputs.insert("value".to_string(), default.clone());
        }
        return_synchronous_result(Ok(result))
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
    fn check_instantiability(
        _ctx: &CommandSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn check_executability(
        _uuid: &ConstructUuid,
        _instance_name: &str,
        _spec: &CommandSpecification,
        _args: &ValueStore,
        _defaults: &AddonDefaults,
        _wallet_instances: &HashMap<ConstructUuid, WalletInstance>,
        _execution_context: &CommandExecutionContext,
    ) -> Result<Vec<ActionItemRequest>, Diagnostic> {
        Ok(vec![])
    }

    fn execute(
        _uuid: &ConstructUuid,
        _spec: &CommandSpecification,
        args: &ValueStore,
        _defaults: &AddonDefaults,
        _wallet_instances: &HashMap<ConstructUuid, WalletInstance>,
        _progress_tx: &txtx_addon_kit::channel::Sender<(ConstructUuid, Diagnostic)>,
    ) -> CommandExecutionFutureResult {
        let value = args.get_expected_value("value")?;
        let mut result = CommandExecutionResult::new();
        result.outputs.insert("value".to_string(), value.clone());
        return_synchronous_result(Ok(result))
    }
}
