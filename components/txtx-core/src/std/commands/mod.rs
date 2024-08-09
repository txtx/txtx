pub mod actions;

use kit::types::frontend::{Actions, BlockEvent, DisplayOutputRequest, ReviewInputRequest};
use kit::types::types::RunbookSupervisionContext;
use kit::types::ValueStore;
use txtx_addon_kit::types::commands::return_synchronous_result;
use txtx_addon_kit::types::frontend::{
    ActionItemRequestType, ActionItemStatus, ProvideInputRequest,
};
use txtx_addon_kit::{
    define_command,
    types::{
        commands::{
            CommandExecutionFutureResult, CommandExecutionResult, CommandImplementation,
            CommandSpecification, PreCommandSpecification,
        },
        diagnostics::Diagnostic,
        frontend::ActionItemRequest,
        types::Type,
        ConstructDid,
    },
    AddonDefaults,
};

use crate::constants::ACTION_ITEM_CHECK_OUTPUT;

pub fn new_module_specification() -> CommandSpecification {
    let command = define_command! {
        Module => {
            name: "Module",
            matcher: "module",
            documentation: "Read Construct attribute",
            implements_signing_capability: false,
            implements_background_task_capability: false,
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
        _construct_id: &ConstructDid,
        _instance_name: &str,
        _spec: &CommandSpecification,
        _args: &ValueStore,
        _defaults: &AddonDefaults,
        _supervision_context: &RunbookSupervisionContext,
    ) -> Result<Actions, Diagnostic> {
        unimplemented!()
    }

    fn run_execution(
        _construct_id: &ConstructDid,
        _spec: &CommandSpecification,
        _args: &ValueStore,
        _defaults: &AddonDefaults,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
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
            implements_signing_capability: false,
            implements_background_task_capability: false,
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
        construct_did: &ConstructDid,
        instance_name: &str,
        spec: &CommandSpecification,
        args: &ValueStore,
        _defaults: &AddonDefaults,
        supervision_context: &RunbookSupervisionContext,
    ) -> Result<Actions, Diagnostic> {
        let title = instance_name;
        let description = args
            .get_string("description")
            .and_then(|d| Some(d.to_string()));

        if !supervision_context.review_input_values
            && !supervision_context.review_input_default_values
        {
            let executable =
                args.get_value("value").is_some() || args.get_value("default").is_some();
            match executable {
                true => return Ok(Actions::none()),
                false => {
                    return Err(diagnosed_error!(
                        "input {}: attribute 'default' or 'value' must be present",
                        instance_name
                    ))
                }
            }
        }

        if let Some(value) = args.get_value("value") {
            for input_spec in spec.inputs.iter() {
                if input_spec.name == "value" && input_spec.check_performed {
                    return Ok(Actions::none());
                }
            }
            if supervision_context.review_input_values {
                return Ok(Actions::new_sub_group_of_items(vec![
                    ActionItemRequest::new(
                        &Some(construct_did.clone()),
                        &title,
                        description,
                        ActionItemStatus::Todo,
                        ActionItemRequestType::ReviewInput(ReviewInputRequest {
                            input_name: "value".to_string(),
                            value: value.clone(),
                        }),
                        "check_input",
                    ),
                ]));
            } else {
                return Ok(Actions::none());
            }
        }

        let (default_value, typing) = match args.get_value("default") {
            Some(default_value) => {
                for input_spec in spec.inputs.iter() {
                    if input_spec.name == "default" && input_spec.check_performed {
                        return Ok(Actions::none());
                    }
                }
                (Some(default_value.clone()), default_value.get_type())
            }
            None => {
                let typing = args.get_expected_value("type")?;
                (
                    None,
                    Type::try_from(typing.as_string().unwrap_or("string").to_string()).map_err(
                        |e| {
                            diagnosed_error!(
                                "input {}: attribute 'type' has invalid value: {}",
                                instance_name,
                                e
                            )
                        },
                    )?,
                )
            }
        };

        let action = ActionItemRequest::new(
            &Some(construct_did.clone()),
            &title,
            description,
            ActionItemStatus::Todo,
            ActionItemRequestType::ProvideInput(ProvideInputRequest {
                default_value: default_value,
                input_name: "default".into(),
                typing,
            }),
            "provide_input",
        );

        return Ok(Actions::append_item(
            action,
            Some("Review and check the inputs from the list below"),
            Some("Inputs Review"),
        ));
    }

    fn run_execution(
        _construct_id: &ConstructDid,
        _spec: &CommandSpecification,
        args: &ValueStore,
        _defaults: &AddonDefaults,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
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
            implements_signing_capability: false,
            implements_background_task_capability: false,
            inputs: [
                value: {
                    documentation: "Value of the output",
                    typing: Type::string(),
                    optional: true,
                    interpolable: true
                },
                description: {
                    documentation: "Description of the output",
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
        construct_did: &ConstructDid,
        instance_name: &str,
        _spec: &CommandSpecification,
        args: &ValueStore,
        _defaults: &AddonDefaults,
        _supervision_context: &RunbookSupervisionContext,
    ) -> Result<Actions, Diagnostic> {
        let value = args.get_expected_value("value")?;
        let actions = Actions::new_sub_group_of_items(vec![ActionItemRequest::new(
            &Some(construct_did.clone()),
            instance_name,
            None,
            ActionItemStatus::Todo,
            ActionItemRequestType::DisplayOutput(DisplayOutputRequest {
                name: instance_name.into(),
                description: None,
                value: value.clone(),
            }),
            ACTION_ITEM_CHECK_OUTPUT,
        )]);
        Ok(actions)
    }

    fn run_execution(
        _construct_id: &ConstructDid,
        _spec: &CommandSpecification,
        args: &ValueStore,
        _defaults: &AddonDefaults,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
    ) -> CommandExecutionFutureResult {
        let value = args.get_expected_value("value")?;
        let mut result = CommandExecutionResult::new();
        result.outputs.insert("value".to_string(), value.clone());
        return_synchronous_result(Ok(result))
    }
}

pub fn new_runtime_setting() -> CommandSpecification {
    let command: PreCommandSpecification = define_command! {
        Runtime => {
            name: "Runtime",
            matcher: "runtime",
            documentation: "Construct designed to import an addon",
            implements_signing_capability: false,
            implements_background_task_capability: false,
            inputs: [
                defaults: {
                    documentation: "Value of the input",
                    typing: Type::object(vec![]),
                    optional: true,
                    interpolable: true
                }
            ],
            outputs: [
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

pub struct Runtime;

impl CommandImplementation for Runtime {
    fn check_instantiability(
        _ctx: &CommandSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn check_executability(
        _construct_did: &ConstructDid,
        _instance_name: &str,
        _spec: &CommandSpecification,
        _args: &ValueStore,
        _defaults: &AddonDefaults,
        _supervision_context: &RunbookSupervisionContext,
    ) -> Result<Actions, Diagnostic> {
        unimplemented!()
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
