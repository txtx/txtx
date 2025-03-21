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
    pub fn construct_type(&self) -> &str {
        match &self {
            PreConstructData::Import(_) => "import",
            PreConstructData::Variable(_) => "variable",
            PreConstructData::Output(_) => "output",
            PreConstructData::Module(_) => "module",
            PreConstructData::Action(_) => "action",
            PreConstructData::Signer(_) => "signer",
            PreConstructData::Addon(_) => "addon",
            PreConstructData::EmbeddedRunbook(_) => "runbook",
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
