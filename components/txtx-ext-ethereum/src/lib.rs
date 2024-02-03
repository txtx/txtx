use constructs::address::Address;
use txtx_ext_kit::{
    hcl::structure::Block,
    helpers::{fs::FileLocation, hcl::VisitorError},
    Extension, ExtensionConstruct,
};

mod constructs;
use crate::constructs::transaction::Transaction;

pub struct EthereumExtension {
    name: Option<String>,
}

impl EthereumExtension {
    pub fn new() -> EthereumExtension {
        EthereumExtension { name: None }
    }
}
impl Extension for EthereumExtension {
    fn get_name(self: &Self) -> String {
        format!("{}", self.name.clone().unwrap_or("ethereum".to_string()))
    }

    fn get_construct_from_block_and_name(
        self: &Self,
        name: &String,
        block: &Block,
        location: &FileLocation,
    ) -> Result<Option<Box<dyn ExtensionConstruct>>, VisitorError> {
        match name.as_str() {
            "transaction" => {
                let construct = Transaction::from_block(block, location)?;
                Ok(Some(construct))
            }
            "address" => {
                let construct = Address::from_block(block, location)?;
                Ok(Some(construct))
            }
            _ => Ok(None),
        }
    }

    fn supports_construct(self: &Self, name: &String) -> bool {
        match name.as_str() {
            "transaction" => true,
            "address" => true,
            _ => false,
        }
    }

    fn index_node(self: &Self) {
        todo!()
    }
}

impl std::fmt::Debug for EthereumExtension {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.get_name())
    }
}
