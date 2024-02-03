pub mod errors;
pub mod types;
pub mod visitor;

use kit::AddonContext;
pub use txtx_addon_kit as kit;
use types::PackageUuid;
// use visitor::run_edge_indexer;

use std::collections::HashMap;

use txtx_addon_kit::Addon;
use types::Manual;
use visitor::run_constructs_indexer;
use visitor::run_constructs_processor;

pub fn simulate_manual(manual: &mut Manual, addons_ctx: &mut AddonsContext) -> Result<(), String> {
    let _ = run_constructs_indexer(manual, addons_ctx)?;
    let _ = run_constructs_processor(manual, addons_ctx)?;
    // let edges = run_edge_indexer(manual)?;
    Ok(())
}

pub struct AddonsContext {
    addons: HashMap<String, Box<dyn Addon>>,
    contexts: HashMap<(PackageUuid, String), Box<dyn AddonContext>>,
}

impl AddonsContext {
    pub fn new() -> Self {
        Self {
            addons: HashMap::new(),
            contexts: HashMap::new(),
        }
    }

    pub fn register(&mut self, addon: Box<dyn Addon>) {
        self.addons.insert(addon.get_namespace().to_string(), addon);
    }

    pub fn get_functions_to_register(&mut self) -> Vec<String> {
        let mut functions = vec![];
        for (_, addon) in self.addons.iter() {
            functions.append(&mut addon.get_functions());
        }
        functions
    }

    pub fn instantiate_context(&mut self, namespace: &str, package_uuid: &PackageUuid) {
        let Some(addon) = self.addons.get(namespace) else {
            return;
        };
        let ctx = addon.create_context();
        self.contexts
            .insert((package_uuid.clone(), namespace.to_string()), ctx);
    }
}
