#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate txtx_addon_kit;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

mod commands;
mod functions;
mod stacks_helpers;
mod typing;

use rust_fsm::StateMachine;
use txtx_addon_kit::{
    hcl::structure::Block,
    types::{
        commands::{
            CommandId, CommandInstance, CommandInstanceOrParts,
            CommandInstanceStateMachine, CommandInstanceType, PreCommandSpecification,
        },
        diagnostics::Diagnostic,
        functions::FunctionSpecification,
        ConstructUuid, PackageUuid,
    },
    Addon, AddonContext,
};

#[derive(Debug)]
pub struct StacksNetworkAddon;

impl StacksNetworkAddon {
    pub fn new() -> Self {
        Self {}
    }
}

impl Addon for StacksNetworkAddon {
    fn get_namespace(&self) -> &str {
        "stacks"
    }

    fn get_functions(&self) -> Vec<FunctionSpecification> {
        functions::STACKS_FUNCTIONS.clone()
    }

    fn get_actions(&self) -> Vec<PreCommandSpecification> {
        commands::actions::STACKS_ACTIONS.clone()
    }

    fn get_prompts(&self) -> Vec<PreCommandSpecification> {
        commands::prompts::STACKS_PROMPTS.clone()
    }

    fn create_context(&self) -> Box<dyn AddonContext> {
        let mut functions = HashMap::new();
        let mut commands = HashMap::new();

        for function in self.get_functions().into_iter() {
            functions.insert(function.name.clone(), function);
        }

        for command in self.get_actions().into_iter() {
            let matcher = match &command {
                PreCommandSpecification::Atomic(command) => command.matcher.clone(),
                PreCommandSpecification::Composite(command) => command.matcher.clone(),
            };
            commands.insert(CommandId::Action(matcher), command);
        }

        for command in self.get_prompts().into_iter() {
            let matcher = match &command {
                PreCommandSpecification::Atomic(command) => command.matcher.clone(),
                PreCommandSpecification::Composite(command) => command.matcher.clone(),
            };
            commands.insert(CommandId::Prompt(matcher), command);
        }

        Box::new(StacksNetworkAddonContext {
            functions,
            commands,
        })
    }
}

#[derive(Debug)]
pub struct StacksNetworkAddonContext {
    pub functions: HashMap<String, FunctionSpecification>,
    pub commands: HashMap<CommandId, PreCommandSpecification>,
}

impl AddonContext for StacksNetworkAddonContext {
    fn create_command_instance(
        self: &Self,
        command_id: &CommandId,
        namespace: &str,
        command_name: &str,
        block: &Block,
        package_uuid: &PackageUuid,
    ) -> Result<CommandInstanceOrParts, Diagnostic> {
        let Some(pre_command_spec) = self.commands.get(command_id) else {
            todo!("return diagnostic: unknown command: {:?}", command_id)
        };
        let typing = match command_id {
            CommandId::Action(_) => CommandInstanceType::Action,
            CommandId::Prompt(_) => CommandInstanceType::Prompt,
        };
        match pre_command_spec {
            PreCommandSpecification::Atomic(command_spec) => {
                let command_instance = CommandInstance {
                    specification: command_spec.clone(),
                    state: Arc::new(Mutex::new(
                        StateMachine::<CommandInstanceStateMachine>::new(),
                    )),
                    name: command_name.to_string(),
                    block: block.clone(),
                    package_uuid: package_uuid.clone(),
                    typing,
                    namespace: Some(namespace.to_string()),
                };
                Ok(CommandInstanceOrParts::Instance(command_instance))
            }
            PreCommandSpecification::Composite(composite_spec) => {
                let bodies = (composite_spec.router)(
                    &block.body.to_string(),
                    &command_name.to_string(),
                    &composite_spec.parts,
                )?;
                Ok(CommandInstanceOrParts::Parts(bodies))
            }
        }
    }

    fn resolve_construct_dependencies(
        self: &Self,
        _construct_uuid: &ConstructUuid,
    ) -> Vec<ConstructUuid> {
        vec![]
    }
}
