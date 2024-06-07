mod construct;
mod package;
mod runbook;

pub use construct::PreConstructData;
pub use package::Package;
pub use runbook::{Runbook, SourceTree};
use std::collections::{BTreeMap, HashMap};
pub use txtx_addon_kit::types::commands::CommandInstance;
use txtx_addon_kit::types::PackageUuid;

use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::types::functions::FunctionSpecification;
use txtx_addon_kit::types::types::Value;
pub use txtx_addon_kit::types::ConstructUuid;

use crate::AddonsContext;

pub struct RuntimeContext {
    pub functions: HashMap<String, FunctionSpecification>,
    pub addons_ctx: AddonsContext,
    pub selected_env: Option<String>,
    pub environments: BTreeMap<String, BTreeMap<String, String>>,
}

impl RuntimeContext {
    pub fn new(
        addons_ctx: AddonsContext,
        environments: BTreeMap<String, BTreeMap<String, String>>,
    ) -> RuntimeContext {
        let mut functions = HashMap::new();

        for (_, addon) in addons_ctx.addons.iter() {
            for function in addon.get_functions().iter() {
                functions.insert(function.name.clone(), function.clone());
            }
        }
        let selected_env = environments
            .iter()
            .next()
            .and_then(|(k, _)| Some(k.clone()));
        RuntimeContext {
            functions,
            addons_ctx,
            selected_env,
            environments,
        }
    }

    pub fn execute_function(
        &self,
        package_uuid: PackageUuid,
        namespace_opt: Option<String>,
        name: &str,
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        let function = match namespace_opt {
            Some(namespace) => match self
                .addons_ctx
                .contexts
                .get(&(package_uuid, namespace.clone()))
            {
                Some(addon) => match addon.functions.get(name) {
                    Some(function) => function,
                    None => {
                        return Err(diagnosed_error!(
                            "could not find function {name} in addon {}",
                            namespace
                        ))
                    }
                },
                None => return Err(diagnosed_error!("could not find namespace {}", namespace)),
            },
            None => match self.functions.get(name) {
                Some(function) => function,
                None => {
                    return Err(diagnosed_error!("could not find function {name}"));
                }
            },
        };
        (function.runner)(function, args)
    }

    pub fn set_active_environment(&mut self, new_env: String) {
        match self.environments.get(&new_env) {
            Some(_) => self.selected_env = Some(new_env),
            None => {}
        }
    }

    pub fn get_active_environment_variables(&self) -> BTreeMap<String, String> {
        let Some(ref active_env) = self.selected_env else {
            return BTreeMap::new();
        };
        match self.environments.get(active_env) {
            Some(variables) => variables.clone(),
            None => BTreeMap::new(),
        }
    }
}
