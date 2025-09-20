use crate::cli::common::addon_registry;
use std::path::Path;
use txtx_core::{
    manifest::WorkspaceManifest,
    validation::{hcl_validator, LocatedInputRef, ValidationError, ValidationResult},
};

pub mod inputs;
pub mod rules;
pub mod validator;

// Re-export types needed by LSP
pub use rules::{ValidationContext, ValidationOutcome, ValidationRule};

/// Analyzes runbook files for validation errors and warnings
pub struct RunbookAnalyzer;

impl RunbookAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Analyze a runbook file with optional manifest context
    pub fn analyze_runbook_with_context(
        &self,
        file_path: &Path,
        content: &str,
        manifest: Option<&WorkspaceManifest>,
        environment: Option<&String>,
        cli_inputs: &[(String, String)],
    ) -> ValidationResult {
        let mut result =
            ValidationResult { errors: Vec::new(), warnings: Vec::new(), suggestions: Vec::new() };

        // Load all addons to get their specifications
        let addons = addon_registry::get_all_addons();
        let addon_specs = addon_registry::extract_addon_specifications(&addons);

        // HCL-based validation with addon specifications
        match hcl_validator::validate_with_hcl_and_addons(
            content,
            &mut result,
            &file_path.to_string_lossy(),
            addon_specs,
        ) {
            Ok(input_refs) => {
                // If we have manifest context, validate inputs with location info
                if let Some(manifest) = manifest {
                    self.validate_inputs_against_manifest_with_locations(
                        &input_refs,
                        content,
                        manifest,
                        environment,
                        &mut result,
                        file_path,
                        cli_inputs,
                    );
                }
            }
            Err(e) => {
                result.errors.push(ValidationError {
                    message: format!("Failed to parse runbook: {}", e),
                    file: file_path.to_string_lossy().to_string(),
                    line: None,
                    column: None,
                    context: None,
                    documentation_link: None,
                });
            }
        }

        result
    }

    /// Validate input references against manifest environment with location information
    pub fn validate_inputs_against_manifest_with_locations(
        &self,
        input_refs: &[LocatedInputRef],
        content: &str,
        manifest: &WorkspaceManifest,
        environment: Option<&String>,
        result: &mut ValidationResult,
        file_path: &Path,
        cli_inputs: &[(String, String)],
    ) {
        inputs::validate_inputs_against_manifest(
            input_refs,
            content,
            manifest,
            environment,
            result,
            file_path,
            cli_inputs,
        );
    }
}
