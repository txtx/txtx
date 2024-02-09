use txtx_addon_kit::hcl::structure::Block;

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
