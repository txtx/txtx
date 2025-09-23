//! Workspace-related handlers for environment management
//!
//! This module provides custom LSP handlers for workspace operations,
//! specifically for environment selection and management.

use super::SharedWorkspaceState;
use crate::cli::lsp::utils::{environment, file_scanner};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct SetEnvironmentParams {
    pub environment: String,
}

/// Handler for workspace-related requests
pub struct WorkspaceHandler {
    workspace_state: SharedWorkspaceState,
    current_environment: std::sync::RwLock<Option<String>>,
}

impl WorkspaceHandler {
    pub fn new(workspace_state: SharedWorkspaceState) -> Self {
        Self { 
            workspace_state, 
            current_environment: std::sync::RwLock::new(None) 
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
        environments.insert("global".to_string());

        // Read workspace state to find environments
        self.collect_environments_from_documents(&mut environments);
        self.collect_environments_from_manifest(&mut environments);

        // If we don't have many environments, scan the workspace
        if environments.len() <= 2 {
            self.scan_workspace_for_environments(&mut environments);
        }

        let mut env_list: Vec<String> = environments.into_iter().collect();
        env_list.sort();

        eprintln!("[DEBUG] Found environments: {:?}", env_list);
        env_list
    }

    /// Set the current environment for validation
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

    /// Collect environments from open documents
    fn collect_environments_from_documents(&self, environments: &mut HashSet<String>) {
        let workspace = self.workspace_state.read();

        for (uri, _doc) in workspace.documents() {
            if let Some(env) = environment::extract_environment_from_uri(uri) {
                environments.insert(env);
            }
        }
    }

    /// Collect environments from manifest
    fn collect_environments_from_manifest(&self, environments: &mut HashSet<String>) {
        let workspace = self.workspace_state.read();

        // Find manifest in open documents
        let manifest_opt = workspace
            .documents()
            .iter()
            .find(|(uri, _)| {
                uri.path().ends_with("txtx.yml") || uri.path().ends_with("txtx.yaml")
            })
            .and_then(|(uri, _)| workspace.get_manifest_for_document(uri));

        if let Some(manifest) = manifest_opt {
            for env_name in manifest.environments.keys() {
                environments.insert(env_name.clone());
            }
        }
    }

    /// Scan workspace directory for environment files
    fn scan_workspace_for_environments(&self, environments: &mut HashSet<String>) {
        if let Some(workspace_root) = self.find_workspace_root() {
            eprintln!("[DEBUG] Scanning workspace root: {:?}", workspace_root);
            
            if let Ok(tx_files) = file_scanner::find_tx_files(&workspace_root) {
                for file in tx_files {
                    if let Some(env) = environment::extract_environment_from_path(&file) {
                        environments.insert(env);
                    }
                }
            }
        }
    }

    /// Find the workspace root by looking for txtx.yml
    fn find_workspace_root(&self) -> Option<PathBuf> {
        let workspace = self.workspace_state.read();

        // Try to find from open documents
        for (uri, _) in workspace.documents() {
            if let Ok(path) = uri.to_file_path() {
                if let Some(root) = file_scanner::find_txtx_yml_root(&path) {
                    return Some(root);
                }
            }
        }

        None
    }
}