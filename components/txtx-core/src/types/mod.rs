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
    pub environments: BTreeMap<String, HashMap<String, String>>,
}

impl RuntimeContext {
    pub fn new(
        addons_ctx: AddonsContext,
        environments_opt: Option<BTreeMap<String, HashMap<String, String>>>,
    ) -> RuntimeContext {
        let mut functions = HashMap::new();

        for (_, addon) in addons_ctx.addons.iter() {
            for function in addon.get_functions().iter() {
                functions.insert(function.name.clone(), function.clone());
            }
        }
        let environments = environments_opt.unwrap_or(BTreeMap::new());
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

    pub fn get_active_environment_variables(&self) -> HashMap<String, String> {
        let Some(ref active_env) = self.selected_env else {
            return HashMap::new();
        };
        match self.environments.get(active_env) {
            Some(variables) => variables.clone(),
            None => HashMap::new(),
        }
    }
}
