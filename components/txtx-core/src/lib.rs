#[macro_use]
extern crate lazy_static;

pub mod errors;
pub mod functions;
pub mod runtime;
pub mod types;
pub mod visitor;

use kit::hcl::structure::Block;
use kit::types::functions::FunctionDeclaration;
use kit::AddonContext;
pub use txtx_addon_kit as kit;
use types::PackageUuid;
use visitor::run_edge_indexer;

use std::collections::HashMap;

use txtx_addon_kit::Addon;
use types::Manual;
use visitor::run_constructs_indexer;
use visitor::run_constructs_processor;

pub fn simulate_manual(manual: &mut Manual, addons_ctx: &mut AddonsContext) -> Result<(), String> {
    let _ = run_constructs_indexer(manual, addons_ctx)?;
    let _ = run_constructs_processor(manual, addons_ctx)?;
    let edges = run_edge_indexer(manual, addons_ctx)?;
    runtime::run(manual, addons_ctx)?;
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

    pub fn consolidate_functions_to_register(&mut self) -> Vec<FunctionDeclaration> {
        let mut functions = vec![];
        for (_, addon) in self.addons.iter() {
            let native_functions = addon.get_native_functions();
            functions.append(&mut addon.get_native_functions());
        }
        functions
    }

    fn find_or_create_context(
        &mut self,
        namespace: &str,
        package_uuid: &PackageUuid,
    ) -> Result<&Box<dyn AddonContext>, String> {
        let key = (package_uuid.clone(), namespace.to_string());
        if self.contexts.get(&key).is_none() {
            let Some(addon) = self.addons.get(namespace) else {
                return Err(format!("addon '{}' unknown", namespace));
            };
            let ctx = addon.create_context();
            self.contexts.insert(key.clone(), ctx);
        }
        return Ok(self.contexts.get(&key).unwrap());
    }

    pub fn index_construct(
        &mut self,
        namespace: &str,
        package_uuid: &PackageUuid,
        block: &Block,
    ) -> Result<bool, String> {
        let ctx = self.find_or_create_context(namespace, package_uuid)?;
        // ctx.index_pre_construct(name, block, location);
        Ok(true)
    }
}
