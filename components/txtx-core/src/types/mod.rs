mod construct;
mod package;

use crate::runbook::RunbookInputsMap;

pub use super::runbook::{
    Runbook, RunbookExecutionContext, RunbookGraphContext, RunbookSnapshotContext, RunbookSources,
};
pub use construct::PreConstructData;
use kit::helpers::fs::FileLocation;
use kit::serde::{Deserialize, Serialize};
use kit::types::types::Value;
pub use package::Package;
use std::collections::BTreeMap;
pub use txtx_addon_kit::types::commands::CommandInstance;
pub use txtx_addon_kit::types::ConstructDid;

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

    pub fn get_runbook_inputs(
        &self,
        selector: &Option<String>,
    ) -> Result<RunbookInputsMap, String> {
        if let Some(selector) = selector {
            if self.environments.get(selector).is_none() {
                return Err(format!("environment '{}' unknown from manifest", selector));
            }
        }

        let mut inputs_map = RunbookInputsMap::new();
        for (selector, inputs) in self.environments.iter() {
            let mut values = vec![];
            for (key, value) in inputs.iter() {
                values.push((key.to_string(), Value::parse_and_default_to_string(value)));
            }
            inputs_map.environments.push(selector.into());
            inputs_map.values.insert(Some(selector.to_string()), values);
        }
        inputs_map.values.insert(None, vec![]);
        inputs_map.current = inputs_map.environments.get(0).map(|v| v.to_string());
        Ok(inputs_map)
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
pub enum RunbookState {
    File(FileLocation),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunbookMetadata {
    pub location: String,
    pub name: String,
    pub description: Option<String>,
    pub id: String,
    pub state: Option<RunbookState>,
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
            state: None,
        }
    }
}
