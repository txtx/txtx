mod construct;
mod package;

pub use super::runbook::{
    Runbook, RunbookExecutionContext, RunbookGraphContext, RunbookSnapshotContext,
    RunbookSources,
};
pub use construct::PreConstructData;
use kit::helpers::fs::FileLocation;
use kit::serde::{Deserialize, Serialize};
pub use package::Package;
use std::collections::{BTreeMap, HashMap};
pub use txtx_addon_kit::types::commands::CommandInstance;
use txtx_addon_kit::types::PackageDid;

use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::types::functions::FunctionSpecification;
use txtx_addon_kit::types::types::Value;
pub use txtx_addon_kit::types::ConstructDid;

use crate::AddonsContext;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProtocolManifest {
    pub name: String,
    pub id: String,
    pub runbooks: Vec<RunbookMetadata>,
    pub environments: BTreeMap<String, BTreeMap<String, String>>,
    #[serde(skip_serializing, skip_deserializing)]
    pub location: Option<FileLocation>,
}

impl ProtocolManifest {
    pub fn new(name: String) -> Self {
        let id = normalize_user_input(&name);
        ProtocolManifest {
            name,
            id,
            runbooks: vec![],
            environments: BTreeMap::new(),
            location: None,
        }
    }
}

fn normalize_user_input(input: &str) -> String {
    let normalized = input.to_lowercase().replace(" ", "-");
    // only allow alphanumeric
    let slug = normalized
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect::<String>();
    slug
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunbookMetadata {
    pub location: String,
    pub name: String,
    pub description: Option<String>,
    pub id: String,
    pub stateful: bool,
}

impl RunbookMetadata {
    pub fn new(action: &str, name: &str, description: Option<String>) -> Self {
        let id = normalize_user_input(name);
        let location = format!("runbooks/{}/{}.tx", action, id);
        RunbookMetadata {
            location,
            name: name.to_string(),
            description,
            id,
            stateful: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnvironmentMetadata {
    location: String,
    name: String,
    description: Option<String>,
}

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

        for (_, (addon, scoped)) in addons_ctx.addons.iter() {
            if *scoped {
                continue;
            }
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
        package_did: PackageDid,
        namespace_opt: Option<String>,
        name: &str,
        args: &Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        let function = match namespace_opt {
            Some(namespace) => match self
                .addons_ctx
                .contexts
                .get(&(package_did, namespace.clone()))
            {
                Some(addon) => match addon.functions.get(name) {
                    Some(function) => function,
                    None => {
                        return Err(diagnosed_error!(
                            "could not find function {name} in namespace {}",
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

    pub fn set_active_environment(&mut self, selected_env: String) -> Result<(), String> {
        match self.environments.get(&selected_env) {
            Some(_) => self.selected_env = Some(selected_env),
            None => {
                return Err(format!("environment with key {} unknown", selected_env));
            }
        }
        Ok(())
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
