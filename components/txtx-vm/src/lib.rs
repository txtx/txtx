pub mod errors;
pub mod types;
pub mod visitor;

use kit::hcl::structure::Block;
use kit::helpers::fs::FileLocation;
use kit::helpers::hcl::VisitorError;
use kit::ExtensionConstruct;
pub use txtx_ext_kit as kit;

use std::collections::HashMap;

use txtx_ext_kit::Extension;
use types::Manual;
use visitor::run_node_indexer;
use visitor::run_node_processor;

pub fn simulate_manual(
    manual: &mut Manual,
    ext_manager: &mut ExtensionManager,
) -> Result<(), String> {
    let _ = run_node_indexer(manual, ext_manager)?;
    let _ = run_node_processor(ext_manager, manual)?;
    manual
        .errors
        .iter()
        .enumerate()
        .for_each(|(i, e)| println!("Error {}: {:?}", i + 1, e));
    Ok(())
}

pub struct ExtensionManager {
    registered_extensions: HashMap<String, Box<dyn Extension>>,
}

impl ExtensionManager {
    pub fn new() -> Self {
        Self {
            registered_extensions: HashMap::new(),
        }
    }

    pub fn register(&mut self, extension: Box<dyn Extension>) {
        let extension_id = extension.get_name();
        self.registered_extensions.insert(extension_id, extension);
    }

    pub fn from_block(
        &self,
        extension_id: String,
        construct_name: String,
        block: &Block,
        location: &FileLocation,
    ) -> Result<Option<Box<dyn ExtensionConstruct>>, VisitorError> {
        let ext = self.registered_extensions.get(&extension_id).unwrap();
        ext.get_construct_from_block_and_name(&construct_name, block, location)
    }
}
