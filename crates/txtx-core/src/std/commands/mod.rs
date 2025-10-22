pub mod actions;

use kit::constants::DocumentationKey;
use kit::types::AuthorizationContext;
use txtx_addon_kit::types::commands::return_synchronous_result;
use txtx_addon_kit::types::frontend::{ActionItemRequestType, ProvideInputRequest};
use txtx_addon_kit::types::frontend::{
    Actions, BlockEvent, DisplayOutputRequest, ReviewInputRequest,
};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::RunbookSupervisionContext;
use txtx_addon_kit::{
    define_command,
    types::{
        commands::{
            CommandExecutionFutureResult, CommandExecutionResult, CommandImplementation,
            CommandSpecification, PreCommandSpecification,
        },
        diagnostics::Diagnostic,
        types::Type,
        ConstructDid,
    },
};

use txtx_addon_kit::constants::ActionItemKey;

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
        _values: &ValueStore,
        _supervision_context: &RunbookSupervisionContext,
        _auth_context: &AuthorizationContext,
    ) -> Result<Actions, Diagnostic> {
        unimplemented!()
    }

    fn run_execution(
        _construct_id: &ConstructDid,
        _spec: &CommandSpecification,
        _values: &ValueStore,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
        _auth_ctx: &txtx_addon_kit::types::AuthorizationContext,
    ) -> CommandExecutionFutureResult {
        let result = CommandExecutionResult::new();
        return_synchronous_result(Ok(result))
    }
}

pub fn new_variable_specification() -> CommandSpecification {
    let command: PreCommandSpecification = define_command! {
        Variable => {
            name: "Variable",
            matcher: "variable",
            documentation: "A construct designed to store a variable.",
            implements_signing_capability: false,
            implements_background_task_capability: false,
            inputs: [
                value: {
                    documentation: "The value of the variable.",
                    typing: Type::string(),
                    optional: false,
                    tainting: true,
                    internal: false
                },
                editable: {
                    documentation: "Determines if the variable value is editable in the supervisor UI.",
                    typing: Type::string(),
                    optional: true,
                    tainting: false,
                    internal: false
                },
                description: {
                    documentation: "A description of the variable.",
                    typing: Type::string(),
                    optional: true,
                    tainting: false,
                    internal: false
                },
                type: {
                    documentation: "The type of the variable. This can usually be inferred from the `value` field.",
                    typing: Type::string(),
                    optional: true,
                    tainting: true,
                    internal: false
                }
            ],
            outputs: [
                value: {
                    documentation: "Value of the variable.",
                    typing: Type::string()
                }
            ],
            example: "",
        }
    };
    match command {
        PreCommandSpecification::Atomic(command) => command,
        PreCommandSpecification::Composite(_) => {
            panic!("variable should not be composite command specification")
        }
    }
}

pub struct Variable;

impl CommandImplementation for Variable {
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
        values: &ValueStore,
        supervision_context: &RunbookSupervisionContext,
        auth_context: &AuthorizationContext,
    ) -> Result<Actions, Diagnostic> {
        let Some(value) = values.get_value("value") else {
            return Err(diagnosed_error!(
                "variable {}: attribute 'value' must be present",
                instance_name
            ));
        };
        if !supervision_context.review_input_values
            && !supervision_context.review_input_default_values
        {
            return Ok(Actions::none());
        }
        for input_spec in spec.inputs.iter() {
            if input_spec.name == "value" && input_spec.check_performed {
                return Ok(Actions::none());
            }
        }

        let title = instance_name;
        let description = values.get_string(DocumentationKey::Description).and_then(|d| Some(d.to_string()));
        let markdown = values.get_markdown(&auth_context)?;

        let is_editable = values.get_bool("editable").unwrap_or(false);
        let action = if is_editable {
            ActionItemRequestType::ProvideInput(ProvideInputRequest {
                default_value: Some(value.to_owned()),
                input_name: "value".into(),
                typing: value.get_type(),
            })
            .to_request(title, ActionItemKey::ProvideInput)
            .with_some_description(description)
            .with_construct_did(construct_did)
            .with_some_markdown(markdown)
        } else {
            if supervision_context.review_input_values {
                ReviewInputRequest::new("value", &value)
                    .to_action_type()
                    .to_request(title, ActionItemKey::CheckInput)
                    .with_some_description(description)
                    .with_construct_did(construct_did)
                    .with_some_markdown(markdown)
            } else {
                return Ok(Actions::none());
            }
        };
        return Ok(Actions::append_item(
            action,
            Some("Review and check the variables from the list below"),
            Some("Variables Review"),
        ));
    }

    fn run_execution(
        _construct_id: &ConstructDid,
        _spec: &CommandSpecification,
        values: &ValueStore,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
        _auth_ctx: &txtx_addon_kit::types::AuthorizationContext,
    ) -> CommandExecutionFutureResult {
        let mut result = CommandExecutionResult::new();
        let value = values.get_expected_value("value")?;
        result.outputs.insert("value".to_string(), value.clone());
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
                    tainting: true,
                    internal: false
                },
                description: {
                    documentation: "Description of the output",
                    typing: Type::string(),
                    optional: true,
                    tainting: false,
                    internal: false
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
        _supervision_context: &RunbookSupervisionContext,
        auth_context: &AuthorizationContext,
    ) -> Result<Actions, Diagnostic> {
        let value = args.get_expected_value("value")?;
        let description = args.get_string(DocumentationKey::Description).and_then(|d| Some(d.to_string()));
        let markdown = args.get_markdown(&auth_context)?;
        let actions = Actions::new_sub_group_of_items(
            None,
            vec![ActionItemRequestType::DisplayOutput(DisplayOutputRequest {
                name: instance_name.into(),
                description: None,
                value: value.clone(),
            })
            .to_request(instance_name, ActionItemKey::CheckOutput)
            .with_construct_did(construct_did)
            .with_some_description(description)
            .with_some_markdown(markdown)],
        );
        Ok(actions)
    }

    fn run_execution(
        _construct_id: &ConstructDid,
        _spec: &CommandSpecification,
        args: &ValueStore,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
        _auth_ctx: &txtx_addon_kit::types::AuthorizationContext,
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
                    typing: Type::arbitrary_object(),
                    optional: true,
                    tainting: true,
                    internal: false
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
        _values: &ValueStore,
        _supervision_context: &RunbookSupervisionContext,
        _auth_context: &AuthorizationContext,
    ) -> Result<Actions, Diagnostic> {
        unimplemented!()
    }

    fn run_execution(
        _construct_id: &ConstructDid,
        _spec: &CommandSpecification,
        _values: &ValueStore,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
        _auth_ctx: &txtx_addon_kit::types::AuthorizationContext,
    ) -> CommandExecutionFutureResult {
        unimplemented!()
    }
}
