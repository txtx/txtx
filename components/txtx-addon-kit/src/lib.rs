#[macro_use]
extern crate serde_derive;

pub use uuid;

use hcl::structure::Block;
pub use hcl_edit as hcl;
use std::fmt::Debug;
use types::{
    commands::{CommandId, CommandInstanceOrParts, PreCommandSpecification},
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
    fn get_actions(&self) -> Vec<PreCommandSpecification>;
    ///
    fn get_prompts(&self) -> Vec<PreCommandSpecification>;
    ///
    fn create_context(self: &Self) -> Box<dyn AddonContext>;
}

///
pub trait AddonContext: Debug + Sync + Send {
    ///
    fn create_command_instance(
        self: &Self,
        command_id: &CommandId,
        namespace: &str,
        command_name: &str,
        block: &Block,
        package_uuid: &PackageUuid,
    ) -> Result<CommandInstanceOrParts, Diagnostic>;
    ///
    fn resolve_construct_dependencies(
        self: &Self,
        construct_uuid: &ConstructUuid,
    ) -> Vec<ConstructUuid>;
}
