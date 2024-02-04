#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate txtx_addon_kit;

mod functions;

use std::collections::HashMap;

use txtx_addon_kit::{
    hcl::{expr::Expression, structure::Block},
    helpers::{fs::FileLocation, hcl::VisitorError},
    types::{diagnostics::Diagnostic, functions::NativeFunction, ConstructUuid},
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

    fn get_native_functions(&self) -> Vec<NativeFunction> {
        functions::STACKS_NATIVE_FUNCTIONS.clone()
    }

    fn get_constructs_types(&self) -> Vec<String> {
        vec![
            "transaction".to_string(),
            "call_contract".to_string(),
            "deploy_contract".to_string(),
        ]
    }

    fn create_context(&self) -> Box<dyn AddonContext> {
        Box::new(StacksNetworkAddonContext {
            constructs: HashMap::new(),
        })
    }
}

#[derive(Debug)]
pub struct StacksNetworkAddonContext {
    pub constructs: HashMap<ConstructUuid, StacksNetworkConstructs>,
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

    fn index_pre_construct(
        self: &Self,
        _construct_name: &String,
        _block: &Block,
        _location: &FileLocation,
    ) -> Result<ConstructUuid, Diagnostic> {
        Ok(ConstructUuid::new())
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
    fn from_block(block: &Block, location: &FileLocation) -> Result<Box<Self>, VisitorError>
    where
        Self: Sized,
    {
        unimplemented!()
    }

    ///
    fn collect_dependencies(self: &Self) -> Vec<Expression> {
        unimplemented!()
    }
}
