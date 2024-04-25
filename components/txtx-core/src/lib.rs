#[macro_use]
extern crate lazy_static;

pub mod errors;
pub mod eval;
pub mod std;
pub mod types;
pub mod visitor;

use ::std::collections::HashMap;
use ::std::sync::mpsc::Sender;
use ::std::sync::Arc;
use ::std::sync::RwLock;

use kit::hcl::structure::Block;
use kit::helpers::fs::FileLocation;
use kit::types::commands::CommandId;
use kit::types::commands::CommandInstanceOrParts;
use kit::types::commands::EvalEvent;
use kit::types::diagnostics::Diagnostic;
use kit::types::functions::FunctionSpecification;
use kit::types::PackageUuid;
use kit::AddonContext;
pub use txtx_addon_kit as kit;
use types::RuntimeContext;
use visitor::run_constructs_dependencies_indexing;

use eval::run_constructs_evaluation;
use txtx_addon_kit::Addon;
use types::Runbook;
use visitor::run_constructs_checks;
use visitor::run_constructs_indexing;

pub fn simulate_runbook(
    runbook: &Arc<RwLock<Runbook>>,
    runtime_context: &Arc<RwLock<RuntimeContext>>,
    eval_tx: Sender<EvalEvent>,
) -> Result<(), String> {
    match runtime_context.write() {
        Ok(mut runtime_context) => {
            let _ = run_constructs_indexing(runbook, &mut runtime_context.addons_ctx)?;
            let _ = run_constructs_checks(runbook, &mut runtime_context.addons_ctx)?;
        }
        Err(e) => unimplemented!("could not acquire lock: {e}"),
    }
    let _ = run_constructs_dependencies_indexing(runbook, runtime_context)?;
    let _ = run_constructs_evaluation(runbook, runtime_context, None, eval_tx).unwrap();
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

    pub fn consolidate_functions_to_register(&mut self) -> Vec<FunctionSpecification> {
        let mut functions = vec![];
        for (_, addon) in self.addons.iter() {
            let mut addon_functions = addon.get_functions();
            functions.append(&mut addon_functions);
        }
        functions
    }

    fn find_or_create_context(
        &mut self,
        namespace: &str,
        package_uuid: &PackageUuid,
    ) -> Result<&Box<dyn AddonContext>, Diagnostic> {
        let key = (package_uuid.clone(), namespace.to_string());
        if self.contexts.get(&key).is_none() {
            let Some(addon) = self.addons.get(namespace) else {
                unimplemented!();
            };
            let ctx = addon.create_context();
            self.contexts.insert(key.clone(), ctx);
        }
        return Ok(self.contexts.get(&key).unwrap());
    }

    pub fn create_action_instance(
        &mut self,
        namespaced_action: &str,
        command_name: &str,
        package_uuid: &PackageUuid,
        block: &Block,
        _location: &FileLocation,
    ) -> Result<CommandInstanceOrParts, Diagnostic> {
        let Some((namespace, command_id)) = namespaced_action.split_once("::") else {
            todo!("return diagnostic")
        };
        let ctx = self.find_or_create_context(namespace, package_uuid)?;
        let command_id = CommandId::Action(command_id.to_string());
        ctx.create_command_instance(&command_id, namespace, command_name, block, package_uuid)
    }

    pub fn create_prompt_instance(
        &mut self,
        namespaced_action: &str,
        command_name: &str,
        package_uuid: &PackageUuid,
        block: &Block,
        _location: &FileLocation,
    ) -> Result<CommandInstanceOrParts, Diagnostic> {
        let Some((namespace, command_id)) = namespaced_action.split_once("::") else {
            todo!("return diagnostic")
        };
        let ctx = self.find_or_create_context(namespace, package_uuid)?;
        let command_id = CommandId::Prompt(command_id.to_string());
        ctx.create_command_instance(&command_id, namespace, command_name, block, package_uuid)
    }
}
