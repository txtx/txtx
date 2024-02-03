#[macro_use]
extern crate serde_derive;

use hcl::{expr::Expression, structure::Block};
pub use hcl_edit as hcl;
use helpers::{fs::FileLocation, hcl::VisitorError};
use std::fmt::Debug;

pub mod helpers;
pub mod types;

pub trait Extension: Debug {
    fn get_name(self: &Self) -> String;
    fn get_construct_from_block_and_name(
        self: &Self,
        name: &String,
        block: &Block,
        location: &FileLocation,
    ) -> Result<Option<Box<dyn ExtensionConstruct>>, VisitorError>;
    fn supports_construct(self: &Self, name: &String) -> bool;
    fn index_node(self: &Self);
}

pub trait ExtensionConstruct: Debug + Sync + Send {
    fn get_name(self: &Self) -> &str;
    fn from_block(block: &Block, location: &FileLocation) -> Result<Box<Self>, VisitorError>
    where
        Self: Sized;
    fn collect_dependencies(self: &Self) -> Vec<Expression>;
}
