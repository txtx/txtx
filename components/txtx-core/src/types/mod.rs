mod construct;
mod package;

pub use super::runbook::{
    Runbook, RunbookExecutionContext, RunbookGraphContext, RunbookSnapshotContext, RunbookSources,
};
pub use construct::PreConstructData;
use kit::helpers::fs::FileLocation;
use kit::serde::{Deserialize, Serialize};
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
    File(String),
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
