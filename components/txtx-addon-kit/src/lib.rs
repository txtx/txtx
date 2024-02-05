#[macro_use]
extern crate serde_derive;

pub use uuid;

use hcl::{expr::Expression, structure::Block};
pub use hcl_edit as hcl;
use helpers::{fs::FileLocation, hcl::VisitorError};
use std::{collections::HashMap, fmt::Debug};
use types::{
    commands::{CommandExecutionResult, CommandSpecification},
    diagnostics::Diagnostic,
    functions::FunctionSpecification,
    ConstructUuid,
};

pub mod helpers;
pub mod macros;
pub mod types;

///
pub trait Addon: Debug {
    ///
    fn get_namespace(self: &Self) -> &str;
    ///
    fn get_functions(self: &Self) -> Vec<FunctionSpecification>;
    ///
    fn get_commands(self: &Self) -> Vec<CommandSpecification>;
    ///
    fn create_context(self: &Self) -> Box<dyn AddonContext>;
}

///
pub trait AddonContext: Debug + Sync + Send {
    ///
    fn get_construct(
        self: &Self,
        construct_uuid: &ConstructUuid,
    ) -> Option<Box<&dyn AddonConstruct>>;
    ///
    fn index_pre_construct(
        self: &Self,
        construct_name: &String,
        block: &Block,
        location: &FileLocation,
    ) -> Result<ConstructUuid, Diagnostic>;
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
