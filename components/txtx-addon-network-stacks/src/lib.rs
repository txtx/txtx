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
            CommandExecutionResult, CommandInstance, CommandInstanceStateMachine,
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

    fn get_commands(&self) -> Vec<CommandSpecification> {
        commands::STACKS_COMMANDS.clone()
    }

    fn create_context(&self) -> Box<dyn AddonContext> {
        let mut functions = HashMap::new();
        let available_functions = functions::STACKS_FUNCTIONS.clone();
        for function in available_functions.into_iter() {
            functions.insert(function.name.clone(), function);
        }
        let mut commands = HashMap::new();
        let available_commands = commands::STACKS_COMMANDS.clone();
        for command in available_commands.into_iter() {
            commands.insert(command.matcher.clone(), command);
        }
        Box::new(StacksNetworkAddonContext {
            constructs: HashMap::new(),
            functions,
            commands,
        })
    }
}

#[derive(Debug)]
pub struct StacksNetworkAddonContext {
    pub constructs: HashMap<ConstructUuid, StacksNetworkConstructs>,
    pub functions: HashMap<String, FunctionSpecification>,
    pub commands: HashMap<String, CommandSpecification>,
}

impl AddonContext for StacksNetworkAddonContext {
    fn get_construct(
        self: &Self,
        construct_uuid: &ConstructUuid,
    ) -> Option<Box<&dyn AddonConstruct>> {
        let Some(construct) = self.constructs.get(construct_uuid) else {
            return None;
        };
        let boxed_construct: Box<&dyn AddonConstruct> = Box::new(construct);
        return Some(boxed_construct);
    }

    fn create_command_instance(
        self: &Self,
        command_type: &str,
        command_name: &str,
        block: &Block,
        package_uuid: &PackageUuid,
    ) -> Result<CommandInstance, Diagnostic> {
        let Some(command_spec) = self.commands.get(command_type) else {
            todo!("return diagnostic: unknown command: {command_type}")
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
