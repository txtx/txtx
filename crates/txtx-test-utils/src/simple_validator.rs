//! Simple validation wrapper for tests
//!
//! This provides a minimal interface to the existing validation logic
//!
//! ## Known Limitations
//!
//! 1. Circular dependency detection between actions is not implemented
//! 2. Deep addon configuration validation only checks for presence of fields

use crate::addon_registry::{extract_addon_specifications, get_all_addons};
use crate::builders::ValidationResult;
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_core::manifest::WorkspaceManifest;
use txtx_core::validation::{
    hcl_validator, ValidationContext, ValidationContextExt, ValidationResult as CoreResult,
};

/// Validate runbook content using the existing validation infrastructure
pub fn validate_content(content: &str) -> ValidationResult {
    // Create core validation result
    let mut core_result =
        CoreResult { errors: Vec::new(), warnings: Vec::new(), suggestions: Vec::new() };

    // Get addon specifications
    let addons = get_all_addons();
    let addon_specs = extract_addon_specifications(&addons);

    // Run validation
    let _ = hcl_validator::validate_with_hcl_and_addons(
        content,
        &mut core_result,
        "test.tx",
        addon_specs,
    );

    // Convert errors to our type
    let errors: Vec<Diagnostic> = core_result
        .errors
        .into_iter()
        .map(|e| Diagnostic::error_from_string(e.message.clone()))
        .collect();

    ValidationResult { success: errors.is_empty(), errors, warnings: vec![] }
}

/// Validate runbook content with manifest and environment support using ValidationContext
pub fn validate_content_with_manifest(
    content: &str,
    manifest: Option<WorkspaceManifest>,
    environment: Option<String>,
    cli_inputs: Vec<(String, String)>,
) -> ValidationResult {
    // Create core validation result
    let mut core_result =
        CoreResult { errors: Vec::new(), warnings: Vec::new(), suggestions: Vec::new() };

    // Get addon specifications
    let addons = get_all_addons();
    let addon_specs = extract_addon_specifications(&addons);

    // Create validation context
    let mut context = ValidationContext::new(content.to_string(), "test.tx".to_string())
        .with_addon_specs(addon_specs.clone())
        .with_cli_inputs(cli_inputs);

    // Add manifest if provided
    if let Some(m) = manifest {
        context = context.with_manifest(m);
    }

    // Add environment if provided
    if let Some(env) = environment {
        context = context.with_environment(env);
    }

    // Run full validation pipeline
    let validation_result = context.validate_full(&mut core_result);

    // Handle validation errors
    if let Err(e) = validation_result {
        core_result.errors.push(txtx_core::validation::ValidationError {
            message: e,
            file: "test.tx".to_string(),
            line: None,
            column: None,
            context: None,
            documentation_link: None,
        });
    }

    // Convert errors to our type
    let errors: Vec<Diagnostic> = core_result
        .errors
        .into_iter()
        .map(|e| Diagnostic::error_from_string(e.message.clone()))
        .collect();

    let warnings: Vec<Diagnostic> = core_result
        .warnings
        .into_iter()
        .map(|w| Diagnostic::warning_from_string(w.message.clone()))
        .collect();

    ValidationResult { success: errors.is_empty(), errors, warnings }
}
