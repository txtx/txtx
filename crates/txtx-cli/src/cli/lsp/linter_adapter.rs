//! Adapter for converting txtx linter results to LSP diagnostics
//!
//! Bridges the linter engine's validation output with the LSP protocol's diagnostic format.

use crate::cli::linter::{Linter, LinterConfig, Format};
use crate::cli::lsp::diagnostics::to_lsp_diagnostic;
use crate::cli::lsp::workspace::{
    manifest_converter::lsp_manifest_to_workspace_manifest, Manifest,
};
use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range, Url};
use std::path::PathBuf;

/// Validate a runbook file with both HCL and linter validation rules
pub fn validate_runbook_with_linter_rules(
    file_uri: &Url,
    content: &str,
    lsp_manifest: Option<&Manifest>,
    environment: Option<&str>,
    cli_inputs: &[(String, String)],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let file_path = file_uri.path();

    // Convert LSP manifest to workspace manifest if available
    let workspace_manifest = lsp_manifest.map(lsp_manifest_to_workspace_manifest);

    // Create linter config
    let config = LinterConfig::new(
        workspace_manifest.as_ref().map(|_| PathBuf::from("./txtx.yml")),
        None, // No specific runbook
        environment.map(String::from),
        cli_inputs.to_vec(),
        Format::Json,
    );

    // Create and run linter
    match Linter::new(&config) {
        Ok(linter) => {
            let result = linter.validate_content(
                content,
                file_path,
                workspace_manifest,
                environment.map(String::from).as_ref(),
            );

            // Convert errors and warnings to LSP diagnostics using unified converter
            for error in &result.errors {
                diagnostics.push(to_lsp_diagnostic(error));
            }

            for warning in &result.warnings {
                diagnostics.push(to_lsp_diagnostic(warning));
            }
        }
        Err(err) => {
            // If linting fails completely, add an error diagnostic
            diagnostics.push(Diagnostic {
                range: Range {
                    start: Position { line: 0, character: 0 },
                    end: Position { line: 0, character: 0 },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: Some("txtx-linter".to_string()),
                message: format!("Failed to run linter: {}", err),
                related_information: None,
                tags: None,
                data: None,
            });
        }
    }

    diagnostics
}