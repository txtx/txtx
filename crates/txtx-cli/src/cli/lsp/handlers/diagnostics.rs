//! Diagnostics handler

use super::Handler;
use crate::cli::lsp::validation::DoctorValidationAdapter;
use crate::cli::lsp::workspace::SharedWorkspaceState;
use lsp_types::*;

pub struct DiagnosticsHandler {
    workspace: SharedWorkspaceState,
    #[allow(dead_code)]
    validator: DoctorValidationAdapter,
}

impl DiagnosticsHandler {
    pub fn new(workspace: SharedWorkspaceState) -> Self {
        Self { workspace, validator: DoctorValidationAdapter::new() }
    }

    #[allow(dead_code)]
    pub fn validate(&self, uri: &Url) -> Option<PublishDiagnosticsParams> {
        let workspace = self.workspace.read();
        let document = workspace.get_document(uri)?;

        // Use doctor validation rules if it's a runbook
        let diagnostics = if document.is_runbook() {
            // Try to find the manifest for this runbook
            let manifest = workspace.get_manifest_for_document(uri);
            eprintln!("[DEBUG] Diagnostics handler - manifest found: {}", manifest.is_some());

            // Use HCL-integrated validation (per ADR-002)
            if let Some(manifest) = manifest {
                crate::cli::lsp::diagnostics_multi_file::validate_with_multi_file_support(
                    uri,
                    document.content(),
                    Some(manifest),
                    None, // TODO: Get environment from workspace state
                    &[],  // TODO: Get CLI inputs from workspace state
                )
            } else {
                // Use HCL-integrated validation with multi-file support
                crate::cli::lsp::diagnostics_hcl_integrated::validate_runbook_with_hcl(
                    uri,
                    document.content(),
                )
            }
        } else {
            Vec::new()
        };

        Some(PublishDiagnosticsParams {
            uri: uri.clone(),
            diagnostics,
            version: Some(document.version()),
        })
    }

    /// Get diagnostics without PublishDiagnosticsParams wrapper
    pub fn get_diagnostics(&self, uri: &Url) -> Vec<Diagnostic> {
        self.get_diagnostics_with_env(uri, None)
    }

    /// Get diagnostics with environment context
    pub fn get_diagnostics_with_env(
        &self,
        uri: &Url,
        environment: Option<&str>,
    ) -> Vec<Diagnostic> {
        let workspace = self.workspace.read();

        if let Some(document) = workspace.get_document(uri) {
            if document.is_runbook() {
                // Try to find the manifest for this runbook
                let manifest = workspace.get_manifest_for_document(uri);

                // Use multi-file aware validation
                if let Some(manifest) = manifest {
                    crate::cli::lsp::diagnostics_multi_file::validate_with_multi_file_support(
                        uri,
                        document.content(),
                        Some(manifest),
                        environment,
                        &[], // TODO: Get CLI inputs from workspace state
                    )
                } else {
                    // Fall back to basic HCL validation
                    crate::cli::lsp::diagnostics::validate_runbook(uri, document.content())
                }
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    }
}

impl Handler for DiagnosticsHandler {
    fn workspace(&self) -> &SharedWorkspaceState {
        &self.workspace
    }
}
