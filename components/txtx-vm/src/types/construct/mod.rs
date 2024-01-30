use crate::types::{ImportConstruct, ModuleConstruct, OutputConstruct, VariableConstruct};
use std::ops::Range;
use txtx_ext_kit::hcl::{expr::Expression, structure::Block};
use uuid::Uuid;

use self::ext::ExtConstruct;

pub mod import;
pub mod module;
pub mod output;
pub mod variable;
pub mod ext;

#[derive(Debug)]
pub enum PreConstructData {
    Variable(Block),
    Module(Block),
    Output(Block),
    Import(Block),
    Ext(Block),
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

    pub fn as_ext(&self) -> Option<&Block> {
        match self {
            PreConstructData::Ext(data) => Some(&data),
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
    Ext(ExtConstruct),
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ConstructUuid {
    Local(Uuid),
}

impl ConstructUuid {
    pub fn new() -> Self {
        Self::Local(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: &Uuid) -> Self {
        Self::Local(uuid.clone())
    }

    pub fn value(&self) -> Uuid {
        match &self {
            Self::Local(v) => v.clone(),
        }
    }
}
