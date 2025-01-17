use kit::{
    helpers::fs::{get_txtx_files_paths, FileLocation},
    indexmap::IndexMap,
    types::RunbookId,
};
use serde::{Deserialize, Serialize};

use crate::runbook::{Runbook, RunbookSources};

use super::{RunbookState, WorkspaceManifest};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceManifestFile {
    pub name: String,
    pub id: String,
    pub runbooks: Vec<RunbookMetadataFile>,
    pub environments: IndexMap<String, IndexMap<String, String>>,
}

impl WorkspaceManifestFile {
    pub fn new(name: String) -> Self {
        let id = normalize_user_input(&name);
        WorkspaceManifestFile { name, id, runbooks: vec![], environments: IndexMap::new() }
    }
}

fn normalize_user_input(input: &str) -> String {
    let normalized = input.to_lowercase().replace(" ", "-");
    // only allow alphanumeric
    let slug = normalized.chars().filter(|c| c.is_alphanumeric() || *c == '-').collect::<String>();
    slug
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunbookMetadataFile {
    pub location: String,
    pub name: String,
    pub description: Option<String>,
    pub state: Option<RunbookStateFile>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunbookStateFile {
    pub location: Option<String>,
}

pub fn read_runbooks_from_manifest(
    manifest: &WorkspaceManifest,
    environment_selector: &Option<String>,
    runbooks_filter_in: Option<&Vec<String>>,
) -> Result<IndexMap<String, (Runbook, RunbookSources, String, Option<RunbookState>)>, String> {
    let mut runbooks = IndexMap::new();

    let root_path =
        manifest.location.as_ref().expect("unable to get location").get_parent_location()?;

    for runbook_metadata in manifest.runbooks.iter() {
        if let Some(runbooks_filter_in) = runbooks_filter_in {
            if !runbooks_filter_in.contains(&runbook_metadata.name) {
                continue;
            }
        }
        let mut package_location = root_path.clone();
        package_location.append_path(&runbook_metadata.location)?;
        let (_, runbook, sources) = read_runbook_from_location(
            &package_location,
            &runbook_metadata.description,
            environment_selector,
            Some(&runbook_metadata.name),
        )?;

        runbooks.insert(
            runbook_metadata.name.to_string(),
            (runbook, sources, runbook_metadata.name.to_string(), runbook_metadata.state.clone()),
        );
    }
    Ok(runbooks)
}

pub fn read_runbook_from_location(
    location: &FileLocation,
    description: &Option<String>,
    environment_selector: &Option<String>,
    runbook_id: Option<&str>,
) -> Result<(String, Runbook, RunbookSources), String> {
    let runbook_name = runbook_id
        .and_then(|id| Some(id.to_string()))
        .unwrap_or(location.get_file_name().unwrap_or(location.to_string()));
    let mut runbook_sources = RunbookSources::new();
    let package_location = location.clone();
    match std::fs::read_dir(package_location.to_string()) {
        Ok(_) => {
            let files = get_txtx_files_paths(&package_location.to_string(), environment_selector)
                .map_err(|e| format!("unable to read directory: {}", e))?;
            for file_path in files.into_iter() {
                let location = FileLocation::from_path(file_path);
                let file_content = location.read_content_as_utf8()?;
                runbook_sources.add_source(runbook_name.clone(), location, file_content);
            }
        }
        Err(_) => {
            let file_content = package_location.read_content_as_utf8()?;
            runbook_sources.add_source(runbook_name.clone(), package_location, file_content);
        }
    }

    let runbook_id = RunbookId::new(None, None, &runbook_name);
    Ok((runbook_name, Runbook::new(runbook_id, description.clone()), runbook_sources))
}
