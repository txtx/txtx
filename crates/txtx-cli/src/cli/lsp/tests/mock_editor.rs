//! Mock editor for testing LSP state management.
//!
//! This module provides [`MockEditor`] for simulating editor interactions
//! with the LSP server. It allows testing state management, validation caching,
//! and dependency tracking in isolation.

use crate::cli::lsp::workspace::{SharedWorkspaceState, ValidationStatus};
use lsp_types::{Diagnostic, Url};
use std::collections::HashMap;

/// Mock editor for testing LSP interactions.
///
/// Simulates an LSP client (like VS Code) by providing methods to:
/// - Open, change, and close documents
/// - Switch environments
/// - Simulate validation cycles
/// - Assert on validation state
///
/// Includes fluent assertion methods for readable test code.
///
/// # Examples
///
/// ```
/// # use txtx_cli::cli::lsp::tests::mock_editor::MockEditor;
/// # use lsp_types::Url;
/// let mut editor = MockEditor::new();
/// let uri = Url::parse("file:///test.tx").unwrap();
///
/// editor.open_document(uri.clone(), "content".to_string());
/// editor.assert_needs_validation(&uri);
///
/// editor.validate_document(&uri, vec![]);
/// editor.assert_no_validation_needed(&uri);
/// ```
pub struct MockEditor {
    /// The workspace state being tested.
    workspace: SharedWorkspaceState,
    /// Documents opened in the editor.
    open_documents: HashMap<Url, String>,
    /// Diagnostics received from LSP.
    received_diagnostics: HashMap<Url, Vec<Diagnostic>>,
    /// Current environment selection.
    current_environment: Option<String>,
}

impl MockEditor {
    /// Creates a new mock editor with empty state.
    pub fn new() -> Self {
        Self {
            workspace: SharedWorkspaceState::new(),
            open_documents: HashMap::new(),
            received_diagnostics: HashMap::new(),
            current_environment: None,
        }
    }

    /// Simulates opening a document.
    ///
    /// Notifies the workspace state and tracks the document internally.
    ///
    /// # Arguments
    ///
    /// * `uri` - The document URI
    /// * `content` - Initial document content
    pub fn open_document(&mut self, uri: Url, content: String) {
        self.workspace.write().open_document(uri.clone(), content.clone());
        self.open_documents.insert(uri, content);
    }

    /// Simulates changing a document.
    ///
    /// Updates the workspace state with new content.
    ///
    /// # Arguments
    ///
    /// * `uri` - The document URI
    /// * `new_content` - Updated document content
    pub fn change_document(&mut self, uri: &Url, new_content: String) {
        self.workspace.write().update_document(uri, new_content.clone());
        self.open_documents.insert(uri.clone(), new_content);
    }

    /// Simulates closing a document.
    ///
    /// Removes the document from workspace state and internal tracking.
    ///
    /// # Arguments
    ///
    /// * `uri` - The document URI
    pub fn close_document(&mut self, uri: &Url) {
        self.workspace.write().close_document(uri);
        self.open_documents.remove(uri);
    }

    /// Simulates switching environment.
    ///
    /// Changes the current environment selection in the workspace.
    ///
    /// # Arguments
    ///
    /// * `environment` - The environment name (e.g., "production", "staging")
    pub fn switch_environment(&mut self, environment: String) {
        self.workspace.write().set_current_environment(Some(environment.clone()));
        self.current_environment = Some(environment);
    }

    /// Simulate receiving diagnostics from LSP
    pub fn receive_diagnostics(&mut self, uri: Url, diagnostics: Vec<Diagnostic>) {
        self.received_diagnostics.insert(uri, diagnostics);
    }

    /// Get the workspace state
    pub fn workspace(&self) -> &SharedWorkspaceState {
        &self.workspace
    }

    /// Get diagnostics for a document
    pub fn get_diagnostics(&self, uri: &Url) -> Option<&Vec<Diagnostic>> {
        self.received_diagnostics.get(uri)
    }

    /// Get current environment
    pub fn get_environment(&self) -> Option<&String> {
        self.current_environment.as_ref()
    }

    /// Sets the current environment and marks all runbooks dirty.
    ///
    /// This simulates an environment switch in the LSP client (e.g., when the user
    /// selects a different environment from a dropdown in VS Code). The workspace
    /// automatically marks all runbooks as dirty when the environment changes.
    ///
    /// # Arguments
    ///
    /// * `environment` - The new environment name, or `None` to clear
    ///
    /// # Example
    ///
    /// ```ignore
    /// editor.set_environment(Some("production".to_string()));
    /// editor.assert_is_dirty(&runbook_uri); // Runbook marked dirty after env change
    /// ```
    pub fn set_environment(&mut self, environment: Option<String>) {
        self.workspace.write().set_current_environment(environment.clone());
        self.current_environment = environment;
    }

    /// Clears all dirty documents by marking them as clean.
    ///
    /// This simulates the state after all pending validations have been completed.
    /// Useful in tests to establish a clean baseline before testing subsequent changes.
    ///
    /// # Side Effects
    ///
    /// For each dirty document:
    /// - Updates validation state to `Clean`
    /// - Sets content hash to current content
    /// - Clears diagnostics
    /// - Removes from dirty set
    ///
    /// # Example
    ///
    /// ```ignore
    /// editor.open_document(uri.clone(), "content".to_string());
    /// editor.clear_dirty(); // Simulate validation completed
    /// editor.assert_not_dirty(&uri); // Document now clean
    /// ```
    pub fn clear_dirty(&mut self) {
        let mut workspace = self.workspace.write();
        let dirty_docs: Vec<Url> = workspace.get_dirty_documents().iter().cloned().collect();
        for uri in dirty_docs {
            // Mark each as clean by updating validation state
            if let Some(content) = self.open_documents.get(&uri) {
                let content_hash = crate::cli::lsp::workspace::WorkspaceState::compute_content_hash(content);
                workspace.update_validation_state(
                    &uri,
                    ValidationStatus::Clean,
                    content_hash,
                    vec![],
                );
            }
        }
    }

    /// Asserts that a document is dirty (needs re-validation).
    ///
    /// This is an alias for [`assert_dirty`](Self::assert_dirty) provided for
    /// consistency with test naming conventions (`assert_is_dirty` reads more
    /// naturally in test code).
    ///
    /// # Panics
    ///
    /// Panics if the document is not marked as dirty.
    pub fn assert_is_dirty(&self, uri: &Url) {
        self.assert_dirty(uri);
    }

    /// Assert document needs validation
    pub fn assert_needs_validation(&self, uri: &Url) {
        let workspace = self.workspace.read();
        let content = self.open_documents.get(uri).expect("Document not open");
        assert!(
            workspace.needs_validation(uri, content),
            "Document {} should need validation",
            uri
        );
    }

    /// Assert document doesn't need validation
    pub fn assert_no_validation_needed(&self, uri: &Url) {
        let workspace = self.workspace.read();
        let content = self.open_documents.get(uri).expect("Document not open");
        assert!(
            !workspace.needs_validation(uri, content),
            "Document {} should not need validation",
            uri
        );
    }

    /// Assert validation status
    pub fn assert_validation_status(&self, uri: &Url, expected: ValidationStatus) {
        let workspace = self.workspace.read();
        let state = workspace
            .get_validation_state(uri)
            .expect("No validation state for document");
        assert_eq!(
            state.status, expected,
            "Expected status {:?}, got {:?}",
            expected, state.status
        );
    }

    /// Assert document is dirty
    pub fn assert_dirty(&self, uri: &Url) {
        let workspace = self.workspace.read();
        assert!(
            workspace.get_dirty_documents().contains(uri),
            "Document {} should be dirty",
            uri
        );
    }

    /// Assert document is not dirty
    pub fn assert_not_dirty(&self, uri: &Url) {
        let workspace = self.workspace.read();
        assert!(
            !workspace.get_dirty_documents().contains(uri),
            "Document {} should not be dirty",
            uri
        );
    }

    /// Assert dependency exists
    pub fn assert_dependency(&self, dependent: &Url, depends_on: &Url) {
        let workspace = self.workspace.read();
        let deps = workspace
            .dependencies()
            .get_dependencies(dependent)
            .expect("No dependencies found");
        assert!(
            deps.contains(depends_on),
            "Expected {} to depend on {}",
            dependent,
            depends_on
        );
    }

    /// Assert cycle detected
    pub fn assert_cycle(&self) {
        let mut workspace = self.workspace.write();
        let cycle = workspace.dependencies_mut().detect_cycles();
        assert!(cycle.is_some(), "Expected cycle to be detected");
    }

    /// Assert no cycle
    pub fn assert_no_cycle(&self) {
        let mut workspace = self.workspace.write();
        let cycle = workspace.dependencies_mut().detect_cycles();
        assert!(cycle.is_none(), "Expected no cycle");
    }

    /// Simulates a full validation cycle.
    ///
    /// Computes content hash, determines status from diagnostics, and updates
    /// the workspace validation state. This mimics what the real LSP server
    /// does after validating a document.
    ///
    /// # Arguments
    ///
    /// * `uri` - The document that was validated
    /// * `diagnostics` - Diagnostics produced by validation
    ///
    /// # Panics
    ///
    /// Panics if the document is not currently open.
    pub fn validate_document(&mut self, uri: &Url, diagnostics: Vec<Diagnostic>) {
        use crate::cli::lsp::workspace::WorkspaceState;

        let content = self.open_documents.get(uri).expect("Document not open");
        let content_hash = WorkspaceState::compute_content_hash(content);

        let status = ValidationStatus::from_diagnostics(&diagnostics);

        self.workspace
            .write()
            .update_validation_state(uri, status, content_hash, diagnostics.clone());

        self.receive_diagnostics(uri.clone(), diagnostics);
    }
}

impl Default for MockEditor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::lsp::tests::test_utils::url;

    #[test]
    fn test_mock_editor_basic_operations() {
        let mut editor = MockEditor::new();
        let uri = url("test.tx");

        // Open document
        editor.open_document(uri.clone(), "content".to_string());
        assert!(editor.open_documents.contains_key(&uri));

        // Change document
        editor.change_document(&uri, "new content".to_string());
        assert_eq!(editor.open_documents.get(&uri).unwrap(), "new content");

        // Close document
        editor.close_document(&uri);
        assert!(!editor.open_documents.contains_key(&uri));
    }

    #[test]
    fn test_mock_editor_validation() {
        let mut editor = MockEditor::new();
        let uri = url("test.tx");

        editor.open_document(uri.clone(), "content".to_string());

        // Initially needs validation
        editor.assert_needs_validation(&uri);

        // After validation, shouldn't need it
        editor.validate_document(&uri, vec![]);
        editor.assert_no_validation_needed(&uri);
        editor.assert_validation_status(&uri, ValidationStatus::Clean);
    }

    #[test]
    fn test_mock_editor_environment_switch() {
        let mut editor = MockEditor::new();
        let uri = url("test.tx");

        editor.open_document(uri.clone(), "input.api_key".to_string());
        editor.switch_environment("sepolia".to_string());

        assert_eq!(editor.get_environment(), Some(&"sepolia".to_string()));

        let workspace = editor.workspace.read();
        assert_eq!(
            workspace.get_current_environment(),
            Some("sepolia".to_string())
        );
    }
}
