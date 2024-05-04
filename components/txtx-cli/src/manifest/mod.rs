use std::collections::BTreeMap;
use std::collections::HashMap;

use serde_json::Value;
use txtx_core::kit::helpers::fs::{get_txtx_files_paths, FileLocation};
use txtx_core::types::{Runbook, SourceTree};

pub mod generator;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProtocolManifest {
    pub name: String,
    runbooks: Vec<RunbookMetadata>,
    pub environments: Option<BTreeMap<String, HashMap<String, String>>>,
    #[serde(skip_serializing, skip_deserializing)]
    location: Option<FileLocation>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunbookMetadata {
    location: String,
    name: String,
    description: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnvironmentMetadata {
    location: String,
    name: String,
    description: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Txvar {
    description: Option<String>,
    value: Value,
}

pub fn read_manifest_at_path(manifest_file_path: &str) -> Result<ProtocolManifest, String> {
    let location = FileLocation::from_path_string(manifest_file_path)?;
    let manifest_bytes = location.read_content()?;
    let mut manifest = serde_json::from_slice::<ProtocolManifest>(&manifest_bytes)
        .map_err(|e| format!("unable to parse manifest: {}", e))?;
    manifest.location = Some(location);
    Ok(manifest)
}

pub fn read_runbooks_from_manifest(
    manifest: &ProtocolManifest,
    runbooks_filter_in: Option<&Vec<String>>,
) -> Result<HashMap<String, Runbook>, String> {
    let mut runbooks = HashMap::new();

    let root_path = manifest
        .location
        .as_ref()
        .expect("unable to get location")
        .get_parent_location()?;

    for RunbookMetadata {
        name: runbook_name,
        location: runbook_root_package_relative_path,
        description,
    } in manifest.runbooks.iter()
    {
        if let Some(runbooks_filter_in) = runbooks_filter_in {
            if !runbooks_filter_in.contains(runbook_name) {
                continue;
            }
        }

        let mut source_tree = SourceTree::new();
        let mut package_location = root_path.clone();
        package_location.append_path(runbook_root_package_relative_path)?;
        match std::fs::read_dir(package_location.to_string()) {
            Ok(_) => {
                let files = get_txtx_files_paths(&package_location.to_string())
                    .map_err(|e| format!("unable to read directory: {}", e))?;
                for file_path in files.into_iter() {
                    let location = FileLocation::from_path(file_path);
                    let file_content = location.read_content_as_utf8()?;
                    source_tree.add_source(runbook_name.to_string(), location, file_content);
                }
            }
            Err(_) => {
                let file_content = package_location.read_content_as_utf8()?;
                source_tree.add_source(runbook_name.to_string(), package_location, file_content);
            }
        }
        runbooks.insert(
            runbook_name.to_string(),
            Runbook::new(Some(source_tree), description.clone()),
        );
    }
    Ok(runbooks)
}
