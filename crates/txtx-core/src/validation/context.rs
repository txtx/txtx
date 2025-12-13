//! Shared validation context
//!
//! This module provides a unified context for all validation operations,
//! reducing parameter passing and making validation state management cleaner.
//!
//! # C4 Architecture Annotations

use super::types::{LocatedInputRef, ValidationResult};
use crate::kit::types::commands::CommandSpecification;
use crate::manifest::WorkspaceManifest;
use std::collections::HashMap;
use std::path::Path;

/// Shared context for validation operations
///
/// This struct contains all the data needed by various validators,
/// reducing the need to pass multiple parameters through the validation pipeline.
///
#[derive(Clone)]
pub struct ValidationContext {
    /// The content being validated
    pub content: String,

    /// Path to the file being validated
    pub file_path: String,

    /// Optional workspace manifest for environment/input validation
    pub manifest: Option<WorkspaceManifest>,

    /// Current environment name (e.g., "production", "staging")
    pub environment: Option<String>,

    /// CLI inputs provided by the user (key-value pairs)
    pub cli_inputs: Vec<(String, String)>,

    /// Addon specifications for validation
    pub addon_specs: Option<HashMap<String, Vec<(String, CommandSpecification)>>>,

    /// Effective inputs computed from manifest, environment, and CLI
    effective_inputs: Option<HashMap<String, String>>,

    /// Collected input references during validation
    pub input_refs: Vec<LocatedInputRef>,
}

impl ValidationContext {
    /// Create a new validation context with minimal required information
    pub fn new(content: impl Into<String>, file_path: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            file_path: file_path.into(),
            manifest: None,
            environment: None,
            cli_inputs: Vec::new(),
            addon_specs: None,
            effective_inputs: None,
            input_refs: Vec::new(),
        }
    }

    /// Set the workspace manifest
    pub fn with_manifest(mut self, manifest: WorkspaceManifest) -> Self {
        self.manifest = Some(manifest);
        self.effective_inputs = None; // Reset cache
        self
    }

    /// Set the current environment
    pub fn with_environment(mut self, environment: impl Into<String>) -> Self {
        self.environment = Some(environment.into());
        self.effective_inputs = None; // Reset cache
        self
    }

    /// Set CLI inputs
    pub fn with_cli_inputs(mut self, cli_inputs: Vec<(String, String)>) -> Self {
        self.cli_inputs = cli_inputs;
        self.effective_inputs = None; // Reset cache
        self
    }

    /// Set addon specifications
    pub fn with_addon_specs(
        mut self,
        specs: HashMap<String, Vec<(String, CommandSpecification)>>,
    ) -> Self {
        self.addon_specs = Some(specs);
        self
    }

    /// Get the file path as a Path
    pub fn file_path_as_path(&self) -> &Path {
        Path::new(&self.file_path)
    }

    /// Get the current environment as a string reference
    pub fn environment_ref(&self) -> Option<&String> {
        self.environment.as_ref()
    }

    /// Get effective inputs (cached computation)
    pub fn effective_inputs(&mut self) -> &HashMap<String, String> {
        if self.effective_inputs.is_none() {
            self.effective_inputs = Some(self.compute_effective_inputs());
        }
        self.effective_inputs
            .as_ref()
            .expect("effective_inputs was just initialized")
    }

    /// Compute effective inputs from manifest, environment, and CLI
    fn compute_effective_inputs(&self) -> HashMap<String, String> {
        let mut inputs = HashMap::new();

        if let Some(manifest) = &self.manifest {
            // First, add defaults from manifest
            if let Some(defaults) = manifest.environments.get("defaults") {
                inputs.extend(defaults.iter().map(|(k, v)| (k.clone(), v.clone())));
            }

            // Then, overlay the specific environment if provided
            if let Some(env_name) = &self.environment {
                if let Some(env_vars) = manifest.environments.get(env_name) {
                    inputs.extend(env_vars.iter().map(|(k, v)| (k.clone(), v.clone())));
                }
            }
        }

        // Finally, overlay CLI inputs (highest precedence)
        inputs.extend(self.cli_inputs.iter().cloned());

        inputs
    }

    /// Add an input reference found during validation
    pub fn add_input_ref(&mut self, input_ref: LocatedInputRef) {
        self.input_refs.push(input_ref);
    }

    /// Load addon specifications from the registry
    pub fn load_addon_specs(&mut self) -> &HashMap<String, Vec<(String, CommandSpecification)>> {
        if self.addon_specs.is_none() {
            // TODO: This is a stopgap solution until we implement a proper compiler pipeline.
            //
            // Current limitation: txtx-core cannot directly depend on addon implementations
            // (evm, bitcoin, svm, etc.) due to:
            // - Heavy dependencies that would bloat core
            // - WASM compatibility requirements
            // - Optional addon features
            // - Circular dependency concerns
            //
            // Current workaround: Two validation paths exist:
            // 1. Simple validation (here) - returns empty specs, limited validation
            // 2. Full validation (CLI/LSP) - passes in actual addon specs
            //
            // Future solution: A proper compiler pipeline with phases:
            // Parse → Resolve (load addons) → Type Check → Optimize → Codegen
            // The resolver phase would load addon specs based on addon declarations
            // in the runbook, making them available for all subsequent phases.
            // This would eliminate the architectural split between validation paths.
            //
            // For now, return empty map - actual implementation would use addon_registry
            self.addon_specs = Some(HashMap::new());
        }
        self.addon_specs.as_ref().unwrap()
    }
}

/// Builder pattern for ValidationContext
pub struct ValidationContextBuilder {
    context: ValidationContext,
}

impl ValidationContextBuilder {
    /// Create a new builder
    pub fn new(content: impl Into<String>, file_path: impl Into<String>) -> Self {
        Self { context: ValidationContext::new(content, file_path) }
    }

    /// Set the workspace manifest
    pub fn manifest(mut self, manifest: WorkspaceManifest) -> Self {
        self.context.manifest = Some(manifest);
        self
    }

    /// Set the current environment
    pub fn environment(mut self, environment: impl Into<String>) -> Self {
        self.context.environment = Some(environment.into());
        self
    }

    /// Set CLI inputs
    pub fn cli_inputs(mut self, cli_inputs: Vec<(String, String)>) -> Self {
        self.context.cli_inputs = cli_inputs;
        self
    }

    /// Set addon specifications
    pub fn addon_specs(
        mut self,
        specs: HashMap<String, Vec<(String, CommandSpecification)>>,
    ) -> Self {
        self.context.addon_specs = Some(specs);
        self
    }

    /// Build the ValidationContext
    pub fn build(self) -> ValidationContext {
        self.context
    }
}

/// Extension trait for ValidationContext to support different validation styles
pub trait ValidationContextExt {
    /// Run HCL validation with this context
    fn validate_hcl(&mut self, result: &mut ValidationResult) -> Result<(), String>;

    /// Run manifest validation with this context
    fn validate_manifest(
        &mut self,
        config: super::ManifestValidationConfig,
        result: &mut ValidationResult,
    );

    /// Run full validation pipeline
    fn validate_full(&mut self, result: &mut ValidationResult) -> Result<(), String>;
}

impl ValidationContextExt for ValidationContext {
    fn validate_hcl(&mut self, result: &mut ValidationResult) -> Result<(), String> {
        // Delegate to HCL validator
        if let Some(specs) = self.addon_specs.clone() {
            let refs = super::hcl_validator::validate_with_hcl_and_addons(
                &self.content,
                result,
                &self.file_path,
                specs,
            )?;
            self.input_refs = refs.inputs;
        } else {
            let refs =
                super::hcl_validator::validate_with_hcl(&self.content, result, &self.file_path)?;
            self.input_refs = refs.inputs;
        }
        Ok(())
    }

    fn validate_manifest(
        &mut self,
        config: super::ManifestValidationConfig,
        result: &mut ValidationResult,
    ) {
        if let Some(manifest) = &self.manifest {
            super::manifest_validator::validate_inputs_against_manifest(
                &self.input_refs,
                &self.content,
                manifest,
                self.environment.as_ref(),
                result,
                &self.file_path,
                &self.cli_inputs,
                config,
            );
        }
    }

    fn validate_full(&mut self, result: &mut ValidationResult) -> Result<(), String> {
        // First run HCL validation
        self.validate_hcl(result)?;

        // Then run manifest validation if we have a manifest
        if self.manifest.is_some() {
            let config = if self.environment.as_deref() == Some("production")
                || self.environment.as_deref() == Some("prod")
            {
                // Use strict validation with linter rules for production
                let mut cfg = super::ManifestValidationConfig::strict();
                cfg.custom_rules.extend(super::linter_rules::get_strict_linter_rules());
                cfg
            } else {
                // Use default validation with standard linter rules
                let mut cfg = super::ManifestValidationConfig::default();
                cfg.custom_rules.extend(super::linter_rules::get_linter_rules());
                cfg
            };

            self.validate_manifest(config, result);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use txtx_addon_kit::indexmap::IndexMap;

    fn create_test_manifest() -> WorkspaceManifest {
        let mut environments = IndexMap::new();

        let mut defaults = IndexMap::new();
        defaults.insert("api_url".to_string(), "https://api.example.com".to_string());
        environments.insert("defaults".to_string(), defaults);

        let mut production = IndexMap::new();
        production.insert("api_url".to_string(), "https://api.prod.example.com".to_string());
        production.insert("api_token".to_string(), "prod-token".to_string());
        environments.insert("production".to_string(), production);

        WorkspaceManifest {
            name: "test".to_string(),
            id: "test-id".to_string(),
            runbooks: Vec::new(),
            environments,
            location: None,
        }
    }

    #[test]
    fn test_validation_context_builder() {
        let manifest = create_test_manifest();
        let context = ValidationContextBuilder::new("test content", "test.tx")
            .manifest(manifest)
            .environment("production")
            .cli_inputs(vec![("debug".to_string(), "true".to_string())])
            .build();

        assert_eq!(context.content, "test content");
        assert_eq!(context.file_path, "test.tx");
        assert_eq!(context.environment, Some("production".to_string()));
        assert_eq!(context.cli_inputs.len(), 1);
    }

    #[test]
    fn test_effective_inputs() {
        let manifest = create_test_manifest();
        let mut context = ValidationContext::new("test", "test.tx")
            .with_manifest(manifest)
            .with_environment("production")
            .with_cli_inputs(vec![("api_url".to_string(), "https://override.com".to_string())]);

        let inputs = context.effective_inputs();

        // CLI should override manifest value
        assert_eq!(inputs.get("api_url"), Some(&"https://override.com".to_string()));
        // Production value should be present
        assert_eq!(inputs.get("api_token"), Some(&"prod-token".to_string()));
    }
}
