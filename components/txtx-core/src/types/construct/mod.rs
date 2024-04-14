use txtx_addon_kit::{hcl::structure::Block, types::commands::CommandInstance};

#[derive(Debug)]
pub enum PreConstructData {
    Input(Block),
    Module(Block),
    Output(Block),
    Import(Block),
    Addon(CommandInstance),
    Root,
}

impl PreConstructData {
    pub fn as_import(&self) -> Option<&Block> {
        match self {
            PreConstructData::Import(data) => Some(&data),
            _ => None,
        }
    }

    pub fn as_input(&self) -> Option<&Block> {
        match self {
            PreConstructData::Input(data) => Some(&data),
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

    pub fn as_addon(&self) -> Option<&CommandInstance> {
        match self {
            PreConstructData::Addon(data) => Some(&data),
            _ => None,
        }
    }
}
