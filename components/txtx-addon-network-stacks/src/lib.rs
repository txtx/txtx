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
mod typing;

use rust_fsm::StateMachine;
use txtx_addon_kit::{
    hcl::{expr::Expression, structure::Block},
    helpers::{fs::FileLocation, hcl::VisitorError},
    types::{
        commands::{
            CommandExecutionResult, CommandId, CommandInstance, CommandInstanceStateMachine,
            CommandSpecification,
        },
        diagnostics::Diagnostic,
        functions::FunctionSpecification,
        ConstructUuid, PackageUuid,
    },
    Addon, AddonConstruct, AddonContext,
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

    fn get_actions(&self) -> Vec<CommandSpecification> {
        commands::actions::STACKS_ACTIONS.clone()
    }

    fn get_prompts(&self) -> Vec<CommandSpecification> {
        commands::prompts::STACKS_PROMPTS.clone()
    }

    fn create_context(&self) -> Box<dyn AddonContext> {
        let mut functions = HashMap::new();
        let mut commands = HashMap::new();

        for function in self.get_functions().into_iter() {
            functions.insert(function.name.clone(), function);
        }

        for command in self.get_actions().into_iter() {
            commands.insert(CommandId::Action(command.matcher.clone()), command);
        }

        for command in self.get_prompts().into_iter() {
            commands.insert(CommandId::Prompt(command.matcher.clone()), command);
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
    pub commands: HashMap<CommandId, CommandSpecification>,
}

impl AddonContext for StacksNetworkAddonContext {
    fn create_command_instance(
        self: &Self,
        command_id: &CommandId,
        command_name: &str,
        block: &Block,
        package_uuid: &PackageUuid,
    ) -> Result<CommandInstance, Diagnostic> {
        let Some(command_spec) = self.commands.get(command_id) else {
            todo!("return diagnostic: unknown command: {:?}", command_id)
        };
        let command_instance = CommandInstance {
            specification: command_spec.clone(),
            state: Arc::new(Mutex::new(
                StateMachine::<CommandInstanceStateMachine>::new(),
            )),
            name: command_name.to_string(),
            block: block.clone(),
            package_uuid: package_uuid.clone(),
        };
        Ok(command_instance)
    }

    fn resolve_construct_dependencies(
        self: &Self,
        _construct_uuid: &ConstructUuid,
    ) -> Vec<ConstructUuid> {
        vec![]
    }
}

#[derive(Debug)]
pub enum StacksNetworkConstructs {
    ContractCall,
    ContractDeploy,
    Transaction,
    Network,
}

impl AddonConstruct for StacksNetworkConstructs {
    //
    fn get_type(self: &Self) -> &str {
        unimplemented!()
    }

    ///
    fn get_name(self: &Self) -> &str {
        unimplemented!()
    }

    ///
    fn get_construct_uuid(self: &Self) -> &ConstructUuid {
        unimplemented!()
    }

    ///
    fn from_block(_block: &Block, _location: &FileLocation) -> Result<Box<Self>, VisitorError>
    where
        Self: Sized,
    {
        unimplemented!()
    }

    ///
    fn collect_dependencies(self: &Self) -> Vec<Expression> {
        unimplemented!()
    }

    fn eval(self: &Self, _dependencies: HashMap<&ConstructUuid, &CommandExecutionResult>) {}
}
