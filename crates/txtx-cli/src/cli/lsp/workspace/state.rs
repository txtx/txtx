//! Centralized workspace state management
//!
//! This module provides the main WorkspaceState that coordinates
//! documents, manifests, and their relationships.

use super::{manifests::find_manifest_for_runbook, Document, Manifest};
use lsp_types::Url;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// The workspace state containing all documents and parsed information
#[derive(Debug)]
pub struct WorkspaceState {
    /// All open documents indexed by URI
    documents: HashMap<Url, Document>,
    /// Parsed manifests indexed by their URI
    manifests: HashMap<Url, Manifest>,
    /// Map from runbook URI to its manifest URI
    runbook_to_manifest: HashMap<Url, Url>,
    /// Cached environment variables for quick lookup
    environment_vars: HashMap<String, HashMap<String, String>>,
    /// The currently selected environment from VS Code
    current_environment: Option<String>,
}

impl WorkspaceState {
    /// Create a new empty workspace state
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            manifests: HashMap::new(),
            runbook_to_manifest: HashMap::new(),
            environment_vars: HashMap::new(),
            current_environment: None,
        }
    }

    /// Open a document in the workspace
    pub fn open_document(&mut self, uri: Url, content: String) {
        let document = Document::new(uri.clone(), content.clone());

        // If it's a manifest, parse and index it
        if document.is_manifest() {
            self.index_manifest(&uri, &content);
        }
        // If it's a runbook, find its manifest
        else if document.is_runbook() {
            self.index_runbook(&uri);
        }

        self.documents.insert(uri, document);
    }

    /// Update an existing document
    pub fn update_document(&mut self, uri: &Url, content: String) {
        if let Some(doc) = self.documents.get_mut(uri) {
            doc.update(content.clone());

            // Re-index if it's a manifest
            if doc.is_manifest() {
                self.index_manifest(uri, &content);
            }
        }
    }

    /// Close a document
    pub fn close_document(&mut self, uri: &Url) {
        self.documents.remove(uri);

        // Clean up manifest data if closing a manifest
        if uri.path().ends_with(".toml") {
            self.manifests.remove(uri);
            // Remove runbook associations
            self.runbook_to_manifest.retain(|_, manifest_uri| manifest_uri != uri);
        }
    }

    /// Get a document by URI
    pub fn get_document(&self, uri: &Url) -> Option<&Document> {
        self.documents.get(uri)
    }

    /// Get all open documents
    #[allow(dead_code)]
    pub fn documents(&self) -> &HashMap<Url, Document> {
        &self.documents
    }

    /// Get a manifest by URI
    #[allow(dead_code)]
    pub fn get_manifest(&self, uri: &Url) -> Option<&Manifest> {
        self.manifests.get(uri)
    }

    /// Get the manifest for a runbook
    pub fn get_manifest_for_runbook(&self, runbook_uri: &Url) -> Option<&Manifest> {
        self.runbook_to_manifest
            .get(runbook_uri)
            .and_then(|manifest_uri| self.manifests.get(manifest_uri))
    }

    /// Get the manifest for a document (alias for get_manifest_for_runbook)
    pub fn get_manifest_for_document(&self, document_uri: &Url) -> Option<&Manifest> {
        self.get_manifest_for_runbook(document_uri)
    }

    /// Get environment variables for a specific environment
    #[allow(dead_code)]
    pub fn get_environment_vars(&self, env_name: &str) -> Option<&HashMap<String, String>> {
        self.environment_vars.get(env_name)
    }

    /// Parse and index a manifest
    fn index_manifest(&mut self, uri: &Url, content: &str) {
        eprintln!("[DEBUG] Indexing manifest: {}", uri);
        match Manifest::parse(uri.clone(), content) {
            Ok(manifest) => {
                eprintln!(
                    "[DEBUG] Manifest parsed successfully with {} runbooks",
                    manifest.runbooks.len()
                );
                // Update environment cache
                for (env_name, vars) in &manifest.environments {
                    self.environment_vars.insert(env_name.clone(), vars.clone());
                }

                // Update runbook associations
                for runbook in &manifest.runbooks {
                    if let Some(runbook_uri) = &runbook.absolute_uri {
                        self.runbook_to_manifest.insert(runbook_uri.clone(), uri.clone());
                    }
                }

                self.manifests.insert(uri.clone(), manifest);
            }
            Err(e) => {
                eprintln!("Failed to parse manifest {}: {}", uri, e);
            }
        }
    }

    /// Index a runbook by finding its manifest
    fn index_runbook(&mut self, runbook_uri: &Url) {
        if let Some(manifest_uri) = find_manifest_for_runbook(runbook_uri) {
            self.runbook_to_manifest.insert(runbook_uri.clone(), manifest_uri.clone());

            // Try to load the manifest if we haven't already
            if !self.manifests.contains_key(&manifest_uri) {
                if let Ok(content) = std::fs::read_to_string(manifest_uri.path()) {
                    self.index_manifest(&manifest_uri, &content);
                }
            }
        }
    }

    /// Get the currently selected environment
    pub fn get_current_environment(&self) -> Option<String> {
        self.current_environment.clone()
    }

    /// Set the currently selected environment
    pub fn set_current_environment(&mut self, environment: Option<String>) {
        self.current_environment = environment;
    }
}

/// Thread-safe wrapper for WorkspaceState
pub struct SharedWorkspaceState {
    inner: Arc<RwLock<WorkspaceState>>,
}

impl SharedWorkspaceState {
    /// Create a new shared workspace state
    pub fn new() -> Self {
        Self { inner: Arc::new(RwLock::new(WorkspaceState::new())) }
    }

    /// Get a read lock on the workspace state
    pub fn read(&self) -> std::sync::RwLockReadGuard<WorkspaceState> {
        self.inner.read().unwrap()
    }

    /// Get a write lock on the workspace state
    pub fn write(&self) -> std::sync::RwLockWriteGuard<WorkspaceState> {
        self.inner.write().unwrap()
    }

    /// Clone the shared reference
    pub fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_document_lifecycle() {
        let mut workspace = WorkspaceState::new();
        let uri = Url::parse("file:///test.tx").unwrap();

        // Open document
        workspace.open_document(uri.clone(), "initial content".to_string());
        assert!(workspace.get_document(&uri).is_some());

        // Update document
        workspace.update_document(&uri, "updated content".to_string());
        let doc = workspace.get_document(&uri).unwrap();
        assert_eq!(doc.content(), "updated content");
        assert_eq!(doc.version(), 2);

        // Close document
        workspace.close_document(&uri);
        assert!(workspace.get_document(&uri).is_none());
    }

    #[test]
    fn test_manifest_indexing() {
        let mut workspace = WorkspaceState::new();
        let manifest_uri = Url::parse("file:///project/txtx.yml").unwrap();
        let manifest_content = r#"
runbooks:
  - name: deploy
    location: runbooks/deploy.tx

environments:
  prod:
    api_key: prod_key
        "#;

        workspace.open_document(manifest_uri.clone(), manifest_content.to_string());

        // Check manifest was parsed
        assert!(workspace.get_manifest(&manifest_uri).is_some());

        // Check environment vars were cached
        let prod_vars = workspace.get_environment_vars("prod").unwrap();
        assert_eq!(prod_vars.get("api_key").unwrap(), "prod_key");
    }
}
