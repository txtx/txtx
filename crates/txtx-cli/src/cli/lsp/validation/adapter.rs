//! Adapter to integrate linter validation into LSP diagnostics

use crate::cli::linter::{Linter, LinterConfig, Format};
use crate::cli::lsp::diagnostics::validation_result_to_diagnostics;
use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range, Url};
use std::path::PathBuf;
use txtx_core::manifest::WorkspaceManifest;

/// Adapter that runs linter validation rules and produces LSP diagnostics
#[derive(Clone)]
pub struct LinterValidationAdapter {
    // We'll create a new linter for each validation since our new linter
    // owns its config
}

impl LinterValidationAdapter {
    /// Create a new adapter
    pub fn new() -> Self {
        Self {}
    }

    /// Run validation on a document and return diagnostics
    #[allow(dead_code)] // Used by LSP handlers for async implementation
    pub fn validate_document(
        &self,
        uri: &Url,
        content: &str,
        manifest: Option<&WorkspaceManifest>,
    ) -> Vec<Diagnostic> {
        // Extract file path from URI
        let file_path = uri.path();

        // Create linter config for this validation
        let config = LinterConfig::new(
            manifest.map(|_| PathBuf::from("./txtx.yml")), // TODO: Get actual manifest path
            None, // No specific runbook
            None, // No environment for now
            Vec::new(), // No CLI inputs
            Format::Json, // Format doesn't matter for programmatic use
        );

        // Create linter
        let linter = match Linter::new(&config) {
            Ok(l) => l,
            Err(err) => {
                // If we can't create the linter, return an error diagnostic
                return vec![Diagnostic {
                    range: Range {
                        start: Position { line: 0, character: 0 },
                        end: Position { line: 0, character: 0 },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: None,
                    code_description: None,
                    source: Some("txtx-linter".to_string()),
                    message: format!("Failed to initialize linter: {}", err),
                    related_information: None,
                    tags: None,
                    data: None,
                }];
            }
        };

        // Run validation
        let result = linter.validate_content(
            content,
            file_path,
            manifest.map(|_| PathBuf::from("./txtx.yml")).as_ref(),
            None, // No environment for now
        );

        // Convert validation results to diagnostics
        validation_result_to_diagnostics(result)
    }

    /// Set active environment for validation
    #[allow(dead_code)] // Kept for API compatibility, may be used when async is fully implemented
    pub fn set_environment(&mut self, _environment: String) {
        // The new linter doesn't store state, environment is passed per validation
        // This is now a no-op but kept for API compatibility
    }
}