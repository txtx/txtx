use crate::runbook::RunbookInputsMap;
use kit::helpers::fs::{FileAccessor, FileLocation};
use kit::indexmap::IndexMap;
use kit::serde::{Deserialize, Serialize};
use kit::types::types::Value;

pub mod file;

pub use file::WorkspaceManifestFile;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceManifest {
    pub name: String,
    pub id: String,
    pub runbooks: Vec<RunbookMetadata>,
    pub environments: IndexMap<String, IndexMap<String, String>>,
    #[serde(skip_serializing, skip_deserializing)]
    pub location: Option<FileLocation>,
}

impl WorkspaceManifest {
    pub fn new(name: String) -> Self {
        let id = normalize_user_input(&name);
        WorkspaceManifest {
            name,
            id,
            runbooks: vec![],
            environments: IndexMap::new(),
            location: None,
        }
    }

    pub async fn from_file_accessor(
        location: &FileLocation,
        file_accessor: &dyn FileAccessor,
    ) -> Result<WorkspaceManifest, String> {
        let content = file_accessor.read_file(location.to_string()).await?;

        let manifest_file: WorkspaceManifestFile = match serde_yml::from_slice(content.as_bytes()) {
            Ok(s) => s,
            Err(e) => {
                return Err(format!("txtx.yml file malformatted {:?}", e));
            }
        };
        WorkspaceManifest::from_manifest_file(manifest_file, location)
    }

    pub fn from_location(location: &FileLocation) -> Result<WorkspaceManifest, String> {
        let manifest_file_content = location.read_content()?;
        let manifest_file: WorkspaceManifestFile =
            match serde_yml::from_slice(&manifest_file_content[..]) {
                Ok(s) => s,
                Err(e) => {
                    return Err(format!("txtx.yml file malformatted {:?}", e));
                }
            };

        WorkspaceManifest::from_manifest_file(manifest_file, location)
    }

    pub fn get_runbook_metadata_from_location(
        &self,
        runbook_location: &str,
    ) -> Option<&RunbookMetadata> {
        for r in self.runbooks.iter() {
            if r.location.eq(runbook_location) {
                return Some(r);
            }
        }
        None
    }

    pub fn from_manifest_file(
        manifest_file: WorkspaceManifestFile,
        manifest_location: &FileLocation,
    ) -> Result<WorkspaceManifest, String> {
        let manifest = WorkspaceManifest {
            name: manifest_file.name,
            id: manifest_file.id,
            runbooks: manifest_file
                .runbooks
                .iter()
                .map(|e| RunbookMetadata {
                    location: e.location.clone(),
                    name: e.name.clone(),
                    description: e.description.clone(),
                    id: e.id.clone(),
                    state: e
                        .state
                        .as_ref()
                        .map(|s| {
                            s.file.clone().map(|f| {
                                let mut location = manifest_location.clone();
                                location = location
                                    .get_parent_location()
                                    .expect("unable to create state destination path");
                                location
                                    .append_path(&f)
                                    .expect("unable to create state destination path");
                                RunbookState::File(location)
                            })
                        })
                        .unwrap_or(None),
                })
                .collect::<Vec<_>>(),
            environments: manifest_file.environments.clone(),
            location: Some(manifest_location.clone()),
        };
        Ok(manifest)
    }

    pub fn get_runbook_inputs(
        &self,
        selector: &Option<String>,
        cli_inputs: &Vec<String>,
        buffer_stdin: Option<String>,
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
        inputs_map.override_values_with_cli_inputs(cli_inputs, buffer_stdin)?;
        Ok(inputs_map)
    }
}

fn normalize_user_input(input: &str) -> String {
    let normalized = input.to_lowercase().replace(" ", "-");
    // only allow alphanumeric
    let slug = normalized.chars().filter(|c| c.is_alphanumeric() || *c == '-').collect::<String>();
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
        RunbookMetadata { location, name: name.to_string(), description, id, state: None }
    }
}
