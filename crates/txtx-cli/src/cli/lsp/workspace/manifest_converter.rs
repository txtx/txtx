//! Converter between LSP Manifest and txtx-core WorkspaceManifest types
//!
//! This module provides conversion utilities to bridge the simplified LSP
//! manifest representation with the full WorkspaceManifest used by doctor.

use super::manifests::Manifest as LspManifest;
use txtx_addon_kit::indexmap::IndexMap;
use txtx_core::manifest::{RunbookMetadata, WorkspaceManifest};

/// Convert an LSP Manifest to a WorkspaceManifest for doctor validation
pub fn lsp_manifest_to_workspace_manifest(lsp_manifest: &LspManifest) -> WorkspaceManifest {
    // Convert runbooks
    let runbooks = lsp_manifest
        .runbooks
        .iter()
        .map(|runbook_ref| RunbookMetadata {
            name: runbook_ref.name.clone(),
            location: runbook_ref.location.clone(),
            description: None,
            state: None,
        })
        .collect();

    // Convert environments - need to convert HashMap to IndexMap
    let mut environments = IndexMap::new();
    for (env_name, env_vars) in &lsp_manifest.environments {
        let mut vars = IndexMap::new();
        for (key, value) in env_vars {
            vars.insert(key.clone(), value.clone());
        }
        environments.insert(env_name.clone(), vars);
    }

    WorkspaceManifest {
        name: "workspace".to_string(), // Default name since LSP doesn't track this
        id: "workspace".to_string(),   // Default ID
        runbooks,
        environments,
        location: None, // LSP doesn't track file location in the same way
    }
}

/// Convert a minimal manifest for validation when only environments are needed
#[allow(dead_code)]
pub fn create_minimal_workspace_manifest(
    environments: &std::collections::HashMap<String, std::collections::HashMap<String, String>>,
) -> WorkspaceManifest {
    let mut env_map = IndexMap::new();
    for (env_name, env_vars) in environments {
        let mut vars = IndexMap::new();
        for (key, value) in env_vars {
            vars.insert(key.clone(), value.clone());
        }
        env_map.insert(env_name.clone(), vars);
    }

    WorkspaceManifest {
        name: "workspace".to_string(),
        id: "workspace".to_string(),
        runbooks: vec![],
        environments: env_map,
        location: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::Url;
    use std::collections::HashMap;

    #[test]
    fn test_lsp_to_workspace_manifest_conversion() {
        // Create a sample LSP manifest
        let mut environments = HashMap::new();
        let mut global_env = HashMap::new();
        global_env.insert("API_KEY".to_string(), "test_key".to_string());
        environments.insert("global".to_string(), global_env);

        let lsp_manifest = LspManifest {
            uri: Url::parse("file:///test/txtx.yml").unwrap(),
            runbooks: vec![],
            environments,
        };

        // Convert to WorkspaceManifest
        let workspace_manifest = lsp_manifest_to_workspace_manifest(&lsp_manifest);

        // Verify conversion
        assert_eq!(workspace_manifest.name, "workspace");
        assert_eq!(workspace_manifest.environments.len(), 1);
        assert_eq!(
            workspace_manifest.environments.get("global").unwrap().get("API_KEY").unwrap(),
            "test_key"
        );
    }
}
