#[macro_use]
extern crate serde_derive;

pub use uuid;

use hcl::{expr::Expression, structure::Block};
pub use hcl_edit as hcl;
use helpers::{fs::FileLocation, hcl::VisitorError};
use std::{collections::HashMap, fmt::Debug};
use types::{
    commands::{CommandExecutionResult, CommandId, CommandInstance, CommandSpecification},
    diagnostics::Diagnostic,
    functions::FunctionSpecification,
    ConstructUuid, PackageUuid,
};

pub use reqwest;

pub mod helpers;
pub mod macros;
pub mod types;

///
pub trait Addon: Debug + Sync + Send {
    ///
    fn get_namespace(self: &Self) -> &str;
    ///
    fn get_functions(self: &Self) -> Vec<FunctionSpecification>;
    ///
    fn get_actions(&self) -> Vec<CommandSpecification>;
    ///
    fn get_prompts(&self) -> Vec<CommandSpecification>;
    ///
    fn create_context(self: &Self) -> Box<dyn AddonContext>;
}

///
pub trait AddonContext: Debug + Sync + Send {
    ///
    fn create_command_instance(
        self: &Self,
        command_id: &CommandId,
        command_name: &str,
        block: &Block,
        package_uuid: &PackageUuid,
    ) -> Result<CommandInstance, Diagnostic>;
    ///
    fn resolve_construct_dependencies(
        self: &Self,
        construct_uuid: &ConstructUuid,
    ) -> Vec<ConstructUuid>;
}

///
pub trait AddonConstruct: Debug + Sync + Send {
    ///
    fn get_type(self: &Self) -> &str;
    ///
    fn get_name(self: &Self) -> &str;
    ///
    fn get_construct_uuid(self: &Self) -> &ConstructUuid;
    ///
    fn from_block(block: &Block, location: &FileLocation) -> Result<Box<Self>, VisitorError>
    where
        Self: Sized;
    ///
    fn collect_dependencies(self: &Self) -> Vec<Expression>;
    ///
    fn eval(self: &Self, dependencies: HashMap<&ConstructUuid, &CommandExecutionResult>);
}
