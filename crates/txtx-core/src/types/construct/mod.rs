use crate::types::ConstructType;
use txtx_addon_kit::types::embedded_runbooks::EmbeddedRunbookInstance;
use txtx_addon_kit::{
    hcl::structure::Block,
    types::{commands::CommandInstance, signers::SignerInstance},
};

#[derive(Debug)]
pub enum PreConstructData {
    Variable(Block),
    Module(Block),
    Output(Block),
    Import(Block),
    Action(CommandInstance),
    Signer(SignerInstance),
    Addon(Block),
    EmbeddedRunbook(EmbeddedRunbookInstance),
    Root,
}

impl PreConstructData {
    /// Get the construct type as an enum.
    ///
    /// This provides type-safe access to the construct type.
    pub fn construct_type_enum(&self) -> ConstructType {
        match &self {
            PreConstructData::Import(_) => ConstructType::Import,
            PreConstructData::Variable(_) => ConstructType::Variable,
            PreConstructData::Output(_) => ConstructType::Output,
            PreConstructData::Module(_) => ConstructType::Module,
            PreConstructData::Action(_) => ConstructType::Action,
            PreConstructData::Signer(_) => ConstructType::Signer,
            PreConstructData::Addon(_) => ConstructType::Addon,
            PreConstructData::EmbeddedRunbook(_) => ConstructType::Runbook,
            PreConstructData::Root => unreachable!(),
        }
    }

    /// Get the construct type as a string.
    ///
    /// For new code, prefer `construct_type_enum()` for type safety.
    pub fn construct_type(&self) -> &str {
        use crate::types::ConstructType;

        match &self {
            PreConstructData::Import(_) => ConstructType::IMPORT,
            PreConstructData::Variable(_) => ConstructType::VARIABLE,
            PreConstructData::Output(_) => ConstructType::OUTPUT,
            PreConstructData::Module(_) => ConstructType::MODULE,
            PreConstructData::Action(_) => ConstructType::ACTION,
            PreConstructData::Signer(_) => ConstructType::SIGNER,
            PreConstructData::Addon(_) => ConstructType::ADDON,
            PreConstructData::EmbeddedRunbook(_) => ConstructType::RUNBOOK,
            PreConstructData::Root => unreachable!(),
        }
    }

    pub fn as_import(&self) -> Option<&Block> {
        match self {
            PreConstructData::Import(data) => Some(&data),
            _ => None,
        }
    }

    pub fn as_input(&self) -> Option<&Block> {
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

    pub fn as_action(&self) -> Option<&CommandInstance> {
        match self {
            PreConstructData::Action(data) => Some(&data),
            _ => None,
        }
    }
}
