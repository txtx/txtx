#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate lazy_static;

#[macro_use]
mod macros;

pub use hex;
pub use indoc::indoc;
use rust_fsm::StateMachine;
pub use uuid;

use hcl::structure::Block;
pub use hcl_edit as hcl;
use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, Mutex},
};
use types::{
    commands::{
        CommandId, CommandInstance, CommandInstanceOrParts, CommandInstanceStateMachine,
        CommandInstanceType, PreCommandSpecification,
    },
    diagnostics::Diagnostic,
    functions::FunctionSpecification,
    ConstructUuid, PackageUuid,
};

pub use reqwest;
pub use serde;

pub mod helpers;
pub mod types;

lazy_static! {
    pub static ref DEFAULT_ADDON_DOCUMENTATION_TEMPLATE: String =
        include_str!("doc/default_addon_template.mdx").to_string();
}

///
pub trait Addon: Debug + Sync + Send {
    ///
    fn get_name(self: &Self) -> &str;
    ///
    fn get_description(self: &Self) -> &str;
    ///
    fn get_namespace(self: &Self) -> &str;
    ///
    fn get_functions(self: &Self) -> Vec<FunctionSpecification>;
    ///
    fn get_actions(&self) -> Vec<PreCommandSpecification>;
    ///
    fn get_prompts(&self) -> Vec<PreCommandSpecification>;
    ///
    fn build_function_lookup(self: &Self) -> HashMap<String, FunctionSpecification> {
        let mut functions = HashMap::new();
        for function in self.get_functions().into_iter() {
            functions.insert(function.name.clone(), function);
        }
        functions
    }
    ///
    fn build_command_lookup(self: &Self) -> HashMap<CommandId, PreCommandSpecification> {
        let mut commands = HashMap::new();

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
        commands
    }

    fn create_context(&self) -> AddonContext {
        AddonContext {
            functions: self.build_function_lookup(),
            commands: self.build_command_lookup(),
            defaults: AddonDefaults::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AddonDefaults {
    pub keys: HashMap<String, String>,
}

impl AddonDefaults {
    pub fn new() -> AddonDefaults {
        AddonDefaults {
            keys: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AddonContext {
    pub functions: HashMap<String, FunctionSpecification>,
    pub commands: HashMap<CommandId, PreCommandSpecification>,
    pub defaults: AddonDefaults,
}

impl AddonContext {
    pub fn create_command_instance(
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
                    namespace: namespace.to_string(),
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

    pub fn resolve_construct_dependencies(
        self: &Self,
        _construct_uuid: &ConstructUuid,
    ) -> Vec<ConstructUuid> {
        vec![]
    }
}
