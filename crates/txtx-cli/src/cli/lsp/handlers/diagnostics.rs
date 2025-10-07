//! Diagnostics handler for LSP validation requests

use super::Handler;
use crate::cli::lsp::validation::LinterValidationAdapter;
use crate::cli::lsp::workspace::SharedWorkspaceState;
use lsp_types::*;
use std::collections::HashMap;

#[derive(Clone)]
pub struct DiagnosticsHandler {
    workspace: SharedWorkspaceState,
    #[allow(dead_code)]
    validator: LinterValidationAdapter,
}

impl DiagnosticsHandler {
    pub fn new(workspace: SharedWorkspaceState) -> Self {
        Self { workspace, validator: LinterValidationAdapter::new() }
    }

    #[allow(dead_code)]
    pub fn validate(&self, uri: &Url) -> Vec<PublishDiagnosticsParams> {
        let workspace = self.workspace.read();
        let diagnostics_by_file = self.get_diagnostics_with_env(uri, None);

        // Return PublishDiagnosticsParams for all affected files
        diagnostics_by_file
            .into_iter()
            .filter_map(|(file_uri, diagnostics)| {
                let document = workspace.get_document(&file_uri)?;
                Some(PublishDiagnosticsParams {
                    uri: file_uri,
                    diagnostics,
                    version: Some(document.version()),
                })
            })
            .collect()
    }

    /// Returns diagnostics for a document without environment context.
    ///
    /// Returns all diagnostics grouped by file. For multi-file runbooks, this includes
    /// diagnostics for all files in the runbook.
    pub fn get_diagnostics(&self, uri: &Url) -> HashMap<Url, Vec<Diagnostic>> {
        self.get_diagnostics_with_env(uri, None)
    }

    /// Returns diagnostics for a document with optional environment context.
    ///
    /// # Arguments
    ///
    /// * `uri` - Document URI to validate
    /// * `environment` - Environment name for context-specific validation (e.g., "production", "staging")
    ///
    /// # Returns
    ///
    /// Diagnostics grouped by file URI. For multi-file runbooks, includes diagnostics for
    /// all files in the runbook. For single files, includes only diagnostics for that file.
    pub fn get_diagnostics_with_env(
        &self,
        uri: &Url,
        environment: Option<&str>,
    ) -> HashMap<Url, Vec<Diagnostic>> {
        let workspace = self.workspace.read();
        let Some(document) = workspace.get_document(uri) else {
            return HashMap::new();
        };

        if !document.is_runbook() {
            return HashMap::new();
        }

        // Choose validation strategy based on manifest availability
        match workspace.get_manifest_for_document(uri) {
            Some(manifest) => {
                crate::cli::lsp::diagnostics_multi_file::validate_with_multi_file_support(
                    uri,
                    document.content(),
                    Some(manifest),
                    environment,
                    &[], // CLI inputs managed by workspace
                )
            }
            None => {
                let diagnostics = crate::cli::lsp::diagnostics::validate_runbook(uri, document.content());
                let mut result = HashMap::new();
                if !diagnostics.is_empty() {
                    result.insert(uri.clone(), diagnostics);
                }
                result
            }
        }
    }

    /// Validates a document and updates its validation state in the workspace cache.
    ///
    /// This method performs validation using the specified environment context and
    /// automatically updates the workspace's validation state cache with the results.
    /// This ensures the cache stays synchronized with actual validation results.
    ///
    /// # Arguments
    ///
    /// * `uri` - The URI of the document to validate
    /// * `environment` - Optional environment name for environment-specific validation
    ///
    /// # Returns
    ///
    /// Diagnostics grouped by file URI. For multi-file runbooks, includes diagnostics
    /// for all files in the runbook.
    ///
    /// # Side Effects
    ///
    /// Updates the workspace's validation cache for each file with:
    /// - Validation status (Clean, Warning, or Error)
    /// - Content hash of the validated document
    /// - Current environment context
    /// - The diagnostics themselves
    pub fn validate_and_update_state(
        &self,
        uri: &Url,
        environment: Option<&str>,
    ) -> HashMap<Url, Vec<Diagnostic>> {
        use crate::cli::lsp::workspace::{ValidationStatus, WorkspaceState};

        let diagnostics_by_file = self.get_diagnostics_with_env(uri, environment);

        // Update validation state in workspace for each file
        let mut workspace = self.workspace.write();
        for (file_uri, diagnostics) in &diagnostics_by_file {
            if let Some(document) = workspace.get_document(file_uri) {
                let content_hash = WorkspaceState::compute_content_hash(document.content());
                let status = ValidationStatus::from_diagnostics(diagnostics);

                workspace.update_validation_state(file_uri, status, content_hash, diagnostics.clone());
            }
        }

        diagnostics_by_file
    }

    /// Gets all documents that need re-validation.
    ///
    /// Returns a list of URIs for documents that have been marked as dirty and
    /// require re-validation. This includes documents whose dependencies have
    /// changed (cascade validation) or whose environment context has changed.
    ///
    /// # Returns
    ///
    /// A vector of URIs for all dirty documents. May be empty if no documents
    /// need validation.
    pub fn get_dirty_documents(&self) -> Vec<Url> {
        self.workspace
            .read()
            .get_dirty_documents()
            .iter()
            .cloned()
            .collect()
    }
}

impl Handler for DiagnosticsHandler {
    fn workspace(&self) -> &SharedWorkspaceState {
        &self.workspace
    }
}
