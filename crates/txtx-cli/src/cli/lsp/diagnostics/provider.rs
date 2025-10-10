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

use super::validation_result_to_diagnostics;
use crate::cli::common::addon_registry;
use lsp_types::{Diagnostic, Url};
use std::collections::HashMap;

/// Validates a runbook file and returns diagnostics.
///
/// Currently performs HCL validation with addon specifications.
/// Deeper semantic validation will be added in future iterations.
pub fn validate_runbook(file_uri: &Url, content: &str) -> Vec<Diagnostic> {
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
    let _ = txtx_core::validation::hcl_validator::validate_with_hcl_and_addons(
        content,
        &mut validation_result,
        file_path,
        addon_specs,
    );

    // Convert validation result to LSP diagnostics
    validation_result_to_diagnostics(validation_result)
}

/// Validates multiple runbook files in a workspace.
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
