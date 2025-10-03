//! Workspace-related handlers for environment management
//!
//! This module provides custom LSP handlers for workspace operations,
//! specifically for environment selection and management.

use super::SharedWorkspaceState;
use crate::cli::lsp::utils::file_scanner;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Serialize, Deserialize)]
pub struct SetEnvironmentParams {
    pub environment: String,
}

/// Handler for workspace-related requests
#[derive(Clone)]
pub struct WorkspaceHandler {
    workspace_state: SharedWorkspaceState,
    current_environment: std::sync::Arc<std::sync::RwLock<Option<String>>>,
}

impl WorkspaceHandler {
    pub fn new(workspace_state: SharedWorkspaceState) -> Self {
        Self {
            workspace_state,
            current_environment: std::sync::Arc::new(std::sync::RwLock::new(None))
        }
    }

    /// Get the workspace state
    pub fn workspace_state(&self) -> &SharedWorkspaceState {
        &self.workspace_state
    }

    /// Get the list of available environments in the workspace
    pub fn get_environments(&self) -> Vec<String> {
        eprintln!("[DEBUG] Getting available environments");

        let mut environments = HashSet::new();

        // Only collect environments from manifest - this is the source of truth
        // Filename-based extraction would include invalid environments not defined in manifest
        self.collect_environments_from_manifest(&mut environments);

        // Filter out 'global' - it's a special default environment that shouldn't be selectable
        let mut env_list: Vec<String> = environments.into_iter()
            .filter(|env| env != "global")
            .collect();
        env_list.sort();

        eprintln!("[DEBUG] Found environments: {:?}", env_list);
        env_list
    }

    /// Set the current environment for validation
    #[allow(dead_code)] // Will be used when async handlers are implemented
    pub fn set_environment(&self, environment: String) {
        eprintln!("[DEBUG] Setting environment to: {}", environment);
        *self.current_environment.write().unwrap() = Some(environment.clone());
        // Also update in the workspace state
        self.workspace_state.write().set_current_environment(Some(environment));
    }

    /// Get the current environment
    pub fn get_current_environment(&self) -> Option<String> {
        // Get from workspace state instead of local field
        self.workspace_state.read().get_current_environment()
    }

    /// Collect environments from manifest
    fn collect_environments_from_manifest(&self, environments: &mut HashSet<String>) {
        let workspace = self.workspace_state.read();

        // First try manifest in already-open documents
        if let Some(manifest) = workspace
            .documents()
            .iter()
            .find(|(uri, _)| {
                uri.path().ends_with("txtx.yml") || uri.path().ends_with("txtx.yaml")
            })
            .and_then(|(uri, _)| workspace.get_manifest_for_document(uri))
        {
            environments.extend(manifest.environments.keys().cloned());
            return;
        }

        // Search upward from any open document to find manifest
        for (uri, _) in workspace.documents() {
            let Ok(path) = uri.to_file_path() else { continue };
            let Some(root) = file_scanner::find_txtx_yml_root(&path) else { continue };

            // Try both txtx.yml and txtx.yaml
            let manifest = ["txtx.yml", "txtx.yaml"]
                .iter()
                .find_map(|name| {
                    let manifest_path = root.join(name);
                    manifest_path.exists().then(|| {
                        lsp_types::Url::from_file_path(&manifest_path)
                            .ok()
                            .and_then(|manifest_uri| workspace.get_manifest(&manifest_uri))
                    })?
                });

            if let Some(manifest) = manifest {
                environments.extend(manifest.environments.keys().cloned());
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_environment_discovery_from_subfolder() {
        // Create temp workspace structure
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();

        // Create manifest with environments
        let manifest_content = r#"
environments:
  sepolia:
    description: "Sepolia testnet"
  mainnet:
    description: "Ethereum mainnet"
"#;
        fs::write(workspace_root.join("txtx.yml"), manifest_content).unwrap();

        // Create subfolder with runbook
        let runbooks_dir = workspace_root.join("runbooks").join("operators").join("step-2");
        fs::create_dir_all(&runbooks_dir).unwrap();

        let main_tx_content = r#"
action "test" "evm::call_contract" {
    signer = signer.operator
}
"#;
        fs::write(runbooks_dir.join("main.tx"), main_tx_content).unwrap();

        // Create workspace handler and state
        let workspace_state = SharedWorkspaceState::new();
        let handler = WorkspaceHandler::new(workspace_state.clone());

        // Open the runbook from subfolder (NOT the manifest)
        let main_uri = lsp_types::Url::from_file_path(runbooks_dir.join("main.tx")).unwrap();
        workspace_state.write().open_document(main_uri.clone(), main_tx_content.to_string());

        // Get environments - should find them by searching upward for manifest
        let environments = handler.get_environments();

        assert!(
            environments.contains(&"sepolia".to_string()),
            "Should find 'sepolia' environment from manifest. Found: {:?}",
            environments
        );
        assert!(
            environments.contains(&"mainnet".to_string()),
            "Should find 'mainnet' environment from manifest. Found: {:?}",
            environments
        );
        assert_eq!(
            environments.len(),
            2,
            "Should find exactly 2 environments. Found: {:?}",
            environments
        );
    }
}