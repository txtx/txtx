use std::collections::BTreeMap;
use std::collections::HashMap;

use serde_json::Value;
use txtx_vm::kit::helpers::fs::{get_txtx_files_paths, FileLocation};
use txtx_vm::types::{Manual, SourceTree};

pub mod generator;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProtocolManifest {
    manuals: BTreeMap<String, String>,
    txvars: Option<BTreeMap<String, Txvar>>,
    #[serde(skip_serializing, skip_deserializing)]
    location: Option<FileLocation>,
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
        .map_err(|e| format!("unable to parse manifest: {}", e.to_string()))?;
    manifest.location = Some(location);
    Ok(manifest)
}

pub fn read_manuals_from_manifest(
    manifest: &ProtocolManifest,
    manuals_filter_in: Option<&Vec<String>>,
) -> Result<HashMap<String, Manual>, String> {
    let mut manuals = HashMap::new();

    let root_path = manifest
        .location
        .as_ref()
        .expect("unable to get location")
        .get_parent_location()?;

    for (manual_name, manual_root_package_relative_path) in manifest.manuals.iter() {
        if let Some(manuals_filter_in) = manuals_filter_in {
            if !manuals_filter_in.contains(manual_name) {
                continue;
            }
        }

        let mut source_tree = SourceTree::new();
        let mut package_location = root_path.clone();
        package_location.append_path(manual_root_package_relative_path)?;
        match std::fs::read_dir(package_location.to_string()) {
            Ok(_) => {
                let files = get_txtx_files_paths(&package_location.to_string())
                    .map_err(|e| format!("unable to read directory: {}", e.to_string()))?;
                for file_path in files.into_iter() {
                    let location = FileLocation::from_path(file_path);
                    let file_content = location.read_content_as_utf8()?;
                    source_tree.add_source(manual_name.to_string(), location, file_content);
                }
            }
            Err(_) => {
                let file_content = package_location.read_content_as_utf8()?;
                source_tree.add_source(manual_name.to_string(), package_location, file_content);
            }
        }
        manuals.insert(manual_name.to_string(), Manual::new(Some(source_tree)));
    }
    Ok(manuals)
}
