use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::PathBuf;

use txtx_core::kit::helpers::fs::{get_txtx_files_paths, FileLocation};
use txtx_core::kit::types::RunbookId;
use txtx_core::types::ProtocolManifest;
use txtx_core::types::RunbookMetadata;
use txtx_core::types::RunbookState;
use txtx_core::types::{Runbook, RunbookSources};

pub mod generator;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProtocolManifestFile {
    pub name: String,
    pub id: String,
    pub runbooks: Vec<RunbookMetadataFile>,
    pub environments: BTreeMap<String, BTreeMap<String, String>>,
}

impl ProtocolManifestFile {
    pub fn new(name: String) -> Self {
        let id = normalize_user_input(&name);
        ProtocolManifestFile {
            name,
            id,
            runbooks: vec![],
            environments: BTreeMap::new(),
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
pub struct RunbookMetadataFile {
    pub location: String,
    pub name: String,
    pub description: Option<String>,
    pub id: String,
    pub state: Option<RunbookStateFile>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunbookStateFile {
    pub file: Option<String>,
}

pub fn read_manifest_at_path(manifest_file_path: &str) -> Result<ProtocolManifest, String> {
    let location = FileLocation::from_path_string(manifest_file_path)?;
    let manifest_bytes = location.read_content()?;
    let manifest_file = serde_yml::from_slice::<ProtocolManifestFile>(&manifest_bytes)
        .map_err(|e| format!("unable to parse manifest: {}", e))?;
    let manifest = ProtocolManifest {
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
                            let root_path = PathBuf::from(manifest_file_path);
                            let mut location = FileLocation::from_path(root_path);
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
        location: Some(location),
    };
    Ok(manifest)
}

pub fn read_runbooks_from_manifest(
    manifest: &ProtocolManifest,
    runbooks_filter_in: Option<&Vec<String>>,
) -> Result<HashMap<String, (Runbook, RunbookSources, String, Option<RunbookState>)>, String> {
    let mut runbooks = HashMap::new();

    let root_path = manifest
        .location
        .as_ref()
        .expect("unable to get location")
        .get_parent_location()?;

    for runbook_metadata in manifest.runbooks.iter() {
        if let Some(runbooks_filter_in) = runbooks_filter_in {
            if !runbooks_filter_in.contains(&runbook_metadata.id) {
                continue;
            }
        }
        let mut package_location = root_path.clone();
        package_location.append_path(&runbook_metadata.location)?;
        let (_, runbook, sources) =
            read_runbook_from_location(&package_location, &runbook_metadata.description)?;

        runbooks.insert(
            runbook_metadata.id.to_string(),
            (
                runbook,
                sources,
                runbook_metadata.name.to_string(),
                runbook_metadata.state.clone(),
            ),
        );
    }
    Ok(runbooks)
}

pub fn read_runbook_from_location(
    location: &FileLocation,
    description: &Option<String>,
) -> Result<(String, Runbook, RunbookSources), String> {
    let runbook_name = location.get_file_name().unwrap_or(location.to_string());
    let mut runbook_sources = RunbookSources::new();
    let package_location = location.clone();
    match std::fs::read_dir(package_location.to_string()) {
        Ok(_) => {
            let files = get_txtx_files_paths(&package_location.to_string())
                .map_err(|e| format!("unable to read directory: {}", e))?;
            for file_path in files.into_iter() {
                let location = FileLocation::from_path(file_path);
                let file_content = location.read_content_as_utf8()?;
                runbook_sources.add_source(runbook_name.to_string(), location, file_content);
            }
        }
        Err(_) => {
            let file_content = package_location.read_content_as_utf8()?;
            runbook_sources.add_source(runbook_name.to_string(), package_location, file_content);
        }
    }

    let runbook_id = RunbookId {
        org: None,
        workspace: None,
        name: runbook_name.to_string(),
    };
    Ok((
        runbook_name,
        Runbook::new(runbook_id, description.clone()),
        runbook_sources,
    ))
}
