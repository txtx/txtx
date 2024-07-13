use std::collections::BTreeMap;
use std::collections::HashMap;

use txtx_addon_network_stacks::StacksNetworkAddon;
use txtx_core::kit::helpers::fs::{get_txtx_files_paths, FileLocation};
use txtx_core::kit::types::RunbookId;
use txtx_core::std::StdAddon;
use txtx_core::types::ProtocolManifest;
use txtx_core::types::RunbookMetadata;
use txtx_core::types::RuntimeContext;
use txtx_core::types::{Runbook, RunbookSources};
use txtx_core::AddonsContext;

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
    pub stateful: Option<bool>,
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
                stateful: e.stateful.unwrap_or(false),
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
) -> Result<HashMap<String, (Runbook, RunbookSources, RuntimeContext, String)>, String> {
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
        let (_, runbook, sources, runtime_context) = read_runbook_from_location(
            &package_location,
            &runbook_metadata.description,
            &manifest.environments,
        )?;

        runbooks.insert(
            runbook_metadata.id.to_string(),
            (
                runbook,
                sources,
                runtime_context,
                runbook_metadata.name.to_string(),
            ),
        );
    }
    Ok(runbooks)
}

pub fn read_runbook_from_location(
    location: &FileLocation,
    description: &Option<String>,
    environments: &BTreeMap<String, BTreeMap<String, String>>,
) -> Result<(String, Runbook, RunbookSources, RuntimeContext), String> {
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

    let mut addons_ctx = AddonsContext::new();
    addons_ctx.register(Box::new(StdAddon::new()), false);
    addons_ctx.register(Box::new(StacksNetworkAddon::new()), true);
    let runtime_context = RuntimeContext::new(addons_ctx, environments.clone());
    let runbook_id = RunbookId {
        org: None,
        project: None,
        name: runbook_name.to_string(),
    };
    Ok((
        runbook_name,
        Runbook::new(runbook_id, description.clone()),
        runbook_sources,
        runtime_context,
    ))
}
