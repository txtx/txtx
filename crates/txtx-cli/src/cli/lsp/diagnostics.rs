//! Real-time diagnostics using runbook validation
//!
//! # C4 Architecture Annotations
//! @c4-component Diagnostics Handler
//! @c4-container LSP Server
//! @c4-description Provides real-time validation diagnostics to IDE
//! @c4-technology Rust
//! @c4-uses Linter Engine "Via linter adapter for validation"
//! @c4-responsibility Validate runbooks on document changes
//! @c4-responsibility Convert validation errors to LSP diagnostics
//! @c4-responsibility Publish diagnostics to IDE

use crate::cli::common::addon_registry;
use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range, Url};
use std::collections::HashMap;

/// Validate a runbook file and return diagnostics
///
/// This is a simplified version that focuses on HCL validation first.
/// We'll add deeper semantic validation in future iterations.
pub fn validate_runbook(file_uri: &Url, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Create a validation result to collect errors
    let mut validation_result = txtx_core::validation::ValidationResult {
        errors: Vec::new(),
        warnings: Vec::new(),
        suggestions: Vec::new(),
    };

    let file_path = file_uri.path();

    // Load all addons to get their specifications
    let addons = addon_registry::get_all_addons();
    let addon_specs = addon_registry::extract_addon_specifications(&addons);

    // Run HCL validation with addon specifications
    match txtx_core::validation::hcl_validator::validate_with_hcl_and_addons(
        content,
        &mut validation_result,
        file_path,
        addon_specs,
    ) {
        Ok(_) | Err(_) => {
            // Convert validation errors to LSP diagnostics
            for error in validation_result.errors {
                let range = Range {
                    start: Position {
                        line: error.line.unwrap_or(1).saturating_sub(1) as u32,
                        character: error.column.unwrap_or(1).saturating_sub(1) as u32,
                    },
                    end: Position {
                        line: error.line.unwrap_or(1).saturating_sub(1) as u32,
                        character: (error.column.unwrap_or(1) + 20) as u32,
                    },
                };

                diagnostics.push(Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: None,
                    code_description: None,
                    source: Some("txtx".to_string()),
                    message: error.message,
                    related_information: None,
                    tags: None,
                    data: None,
                });
            }

            // Convert warnings
            for warning in validation_result.warnings {
                let range = Range {
                    start: Position {
                        line: warning.line.unwrap_or(1).saturating_sub(1) as u32,
                        character: warning.column.unwrap_or(1).saturating_sub(1) as u32,
                    },
                    end: Position {
                        line: warning.line.unwrap_or(1).saturating_sub(1) as u32,
                        character: (warning.column.unwrap_or(1) + 20) as u32,
                    },
                };

                diagnostics.push(Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::WARNING),
                    code: None,
                    code_description: None,
                    source: Some("txtx".to_string()),
                    message: warning.message,
                    related_information: None,
                    tags: None,
                    data: None,
                });
            }
        }
    }

    diagnostics
}

/// Validate multiple runbook files in a workspace
#[allow(dead_code)]
pub fn validate_workspace(files: HashMap<Url, String>) -> HashMap<Url, Vec<Diagnostic>> {
    let mut all_diagnostics = HashMap::new();

    // Validate each file independently for now
    for (uri, content) in files {
        let diagnostics = validate_runbook(&uri, &content);
        if !diagnostics.is_empty() {
            all_diagnostics.insert(uri, diagnostics);
        }
    }

    all_diagnostics
}
