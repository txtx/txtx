use crate::types::{ImportConstruct, ModuleConstruct, OutputConstruct, VariableConstruct};
use std::ops::Range;
use txtx_addon_kit::{
    hcl::{expr::Expression, structure::Block},
    types::ConstructUuid,
};

use self::addon::AddonConstruct;

pub mod addon;
pub mod import;
pub mod module;
pub mod output;
pub mod variable;

#[derive(Debug)]
pub enum PreConstructData {
    Variable(Block),
    Module(Block),
    Output(Block),
    Import(Block),
    Addon(Block),
    Root,
}

impl PreConstructData {
    pub fn as_import(&self) -> Option<&Block> {
        match self {
            PreConstructData::Import(data) => Some(&data),
            _ => None,
        }
    }

    pub fn as_variable(&self) -> Option<&Block> {
        match self {
            PreConstructData::Variable(data) => Some(&data),
            _ => None,
        }
    }

    pub fn as_output(&self) -> Option<&Block> {
        match self {
            PreConstructData::Output(data) => Some(&data),
            _ => None,
        }
    }

    pub fn as_module(&self) -> Option<&Block> {
        match self {
            PreConstructData::Module(data) => Some(&data),
            _ => None,
        }
    }

    pub fn as_addon(&self) -> Option<&Block> {
        match self {
            PreConstructData::Addon(data) => Some(&data),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct PreConstruct {
    pub uuid: ConstructUuid,
    pub name: String,
    pub data: PreConstructData,
    pub span: Range<usize>,
}

#[derive(Debug)]
pub enum ConstructData {
    Variable(VariableConstruct),
    Output(OutputConstruct),
    Module(ModuleConstruct),
    Import(ImportConstruct),
    Ext(AddonConstruct),
}

impl ConstructData {
    pub fn get_construct_uri(&self) -> &str {
        match self {
            ConstructData::Variable(data) => data.name.as_str(),
            ConstructData::Output(data) => data.name.as_str(),
            ConstructData::Module(data) => data.id.as_str(),
            ConstructData::Import(data) => data.name.as_str(),
            ConstructData::Ext(data) => data.name.as_str(),
        }
    }

    pub fn collect_dependencies(&self) -> Vec<Expression> {
        let deps = match self {
            ConstructData::Variable(data) => data.collect_dependencies(),
            ConstructData::Output(data) => data.collect_dependencies(),
            ConstructData::Module(data) => data.collect_dependencies(),
            ConstructData::Import(data) => data.collect_dependencies(),
            ConstructData::Ext(data) => data.collect_dependencies(),
        };
        deps
    }

    pub fn eval_inputs(&self) {
        let deps = match self {
            ConstructData::Variable(data) => data.eval_inputs(),
            ConstructData::Output(data) => data.eval_inputs(),
            ConstructData::Module(data) => data.eval_inputs(),
            ConstructData::Import(data) => data.eval_inputs(),
            ConstructData::Ext(data) => data.eval_inputs(),
        };
    }

    pub fn as_import(&self) -> Option<&ImportConstruct> {
        match self {
            ConstructData::Import(data) => Some(&data),
            _ => None,
        }
    }

    pub fn as_variable(&self) -> Option<&VariableConstruct> {
        match self {
            ConstructData::Variable(data) => Some(&data),
            _ => None,
        }
    }

    pub fn as_output(&self) -> Option<&OutputConstruct> {
        match self {
            ConstructData::Output(data) => Some(&data),
            _ => None,
        }
    }

    pub fn as_module(&self) -> Option<&ModuleConstruct> {
        match self {
            ConstructData::Module(data) => Some(&data),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct Construct {
    pub uuid: ConstructUuid,
    pub file_uri: String,
    pub name: String,
    pub data: ConstructData,
    pub span: Range<usize>,
}
