//! Manifest validation functionality
//!
//! This module provides validation of runbook inputs against workspace manifests,
//! checking that environment variables and inputs are properly defined.

use super::types::{
    LocatedInputRef, ValidationError, ValidationResult, ValidationSuggestion, ValidationWarning,
};
use crate::manifest::WorkspaceManifest;
use std::collections::HashMap;

/// Configuration for manifest validation
pub struct ManifestValidationConfig {
    /// Whether to use strict validation (e.g., for production environments)
    pub strict_mode: bool,
    /// Additional validation rules to apply
    pub custom_rules: Vec<Box<dyn ManifestValidationRule>>,
}

impl Default for ManifestValidationConfig {
    fn default() -> Self {
        Self { strict_mode: false, custom_rules: Vec::new() }
    }
}

impl ManifestValidationConfig {
    /// Create a strict validation configuration
    pub fn strict() -> Self {
        Self { strict_mode: true, custom_rules: Vec::new() }
    }
}

/// Trait for custom manifest validation rules
pub trait ManifestValidationRule: Send + Sync {
    /// Name of the rule for debugging
    fn name(&self) -> &'static str;

    /// Description of what the rule checks
    fn description(&self) -> &'static str;

    /// Check if the rule applies to this input
    fn check(&self, context: &ManifestValidationContext) -> ValidationOutcome;
}

/// Context provided to validation rules
pub struct ManifestValidationContext<'a> {
    pub input_name: &'a str,
    pub full_name: &'a str,
    pub manifest: &'a WorkspaceManifest,
    pub environment: Option<&'a str>,
    pub effective_inputs: &'a HashMap<String, String>,
    pub cli_inputs: &'a [(String, String)],
    pub content: &'a str,
    pub file_path: &'a str,
}

/// Outcome of a validation rule check
pub enum ValidationOutcome {
    /// Rule passed
    Pass,
    /// Rule failed with error
    Error {
        message: String,
        context: Option<String>,
        suggestion: Option<String>,
        documentation_link: Option<String>,
    },
    /// Rule generated a warning
    Warning { message: String, suggestion: Option<String> },
}

/// Validate input references against a manifest
pub fn validate_inputs_against_manifest(
    input_refs: &[LocatedInputRef],
    content: &str,
    manifest: &WorkspaceManifest,
    environment: Option<&String>,
    result: &mut ValidationResult,
    file_path: &str,
    cli_inputs: &[(String, String)],
    config: ManifestValidationConfig,
) {
    // Build effective inputs from environment hierarchy
    let effective_inputs = build_effective_inputs(manifest, environment, cli_inputs);

    // Add CLI precedence message if applicable
    if !cli_inputs.is_empty() {
        result.suggestions.push(ValidationSuggestion {
            message: format!(
                "{} CLI inputs provided. CLI inputs take precedence over environment values.",
                cli_inputs.len()
            ),
            example: None,
        });
    }

    // Get validation rules based on configuration
    let rules = if config.strict_mode { get_strict_rules() } else { get_default_rules() };

    // Add any custom rules
    let mut all_rules = rules;
    all_rules.extend(config.custom_rules);

    // Process each input reference through all rules
    for input_ref in input_refs {
        let input_name = strip_input_prefix(&input_ref.name);

        // Create validation context
        let context = ManifestValidationContext {
            input_name,
            full_name: &input_ref.name,
            manifest,
            environment: environment.as_ref().map(|s| s.as_str()),
            effective_inputs: &effective_inputs,
            cli_inputs,
            content,
            file_path,
        };

        // Run each rule and process outcomes
        for rule in &all_rules {
            match rule.check(&context) {
                ValidationOutcome::Pass => continue,

                ValidationOutcome::Error {
                    message,
                    context: ctx,
                    suggestion,
                    documentation_link,
                } => {
                    result.errors.push(ValidationError {
                        message,
                        file: file_path.to_string(),
                        line: Some(input_ref.line),
                        column: Some(input_ref.column),
                        context: ctx,
                        documentation_link,
                    });

                    if let Some(suggestion) = suggestion {
                        result
                            .suggestions
                            .push(ValidationSuggestion { message: suggestion, example: None });
                    }
                }

                ValidationOutcome::Warning { message, suggestion } => {
                    result.warnings.push(ValidationWarning {
                        message,
                        file: file_path.to_string(),
                        line: Some(input_ref.line),
                        column: Some(input_ref.column),
                        suggestion,
                    });
                }
            }
        }
    }
}

/// Build effective inputs by merging manifest environments with CLI inputs
fn build_effective_inputs(
    manifest: &WorkspaceManifest,
    environment: Option<&String>,
    cli_inputs: &[(String, String)],
) -> HashMap<String, String> {
    let mut inputs = HashMap::new();

    // First, add global environment (txtx's default environment)
    if let Some(global) = manifest.environments.get("global") {
        inputs.extend(global.iter().map(|(k, v)| (k.clone(), v.clone())));
    }

    // Then, overlay the specific environment if provided
    if let Some(env_name) = environment {
        if let Some(env_vars) = manifest.environments.get(env_name) {
            inputs.extend(env_vars.iter().map(|(k, v)| (k.clone(), v.clone())));
        }
    }

    // Finally, overlay CLI inputs (highest precedence)
    for (key, value) in cli_inputs {
        inputs.insert(key.clone(), value.clone());
    }

    inputs
}

/// Strip common input prefixes
fn strip_input_prefix(name: &str) -> &str {
    name.strip_prefix("input.")
        .or_else(|| name.strip_prefix("env."))
        .or_else(|| name.strip_prefix("var."))
        .unwrap_or(name)
}

/// Get default validation rules
fn get_default_rules() -> Vec<Box<dyn ManifestValidationRule>> {
    vec![Box::new(UndefinedInputRule), Box::new(DeprecatedInputRule)]
}

/// Get strict validation rules (for production environments)
fn get_strict_rules() -> Vec<Box<dyn ManifestValidationRule>> {
    vec![Box::new(UndefinedInputRule), Box::new(DeprecatedInputRule), Box::new(RequiredInputRule)]
}

// Built-in validation rules

/// Rule: Check for undefined inputs
struct UndefinedInputRule;

impl ManifestValidationRule for UndefinedInputRule {
    fn name(&self) -> &'static str {
        "undefined_input"
    }

    fn description(&self) -> &'static str {
        "Checks if input references exist in the manifest or CLI inputs"
    }

    fn check(&self, context: &ManifestValidationContext) -> ValidationOutcome {
        // Check if the input exists in effective inputs
        if !context.effective_inputs.contains_key(context.input_name) {
            // Check if it's provided via CLI
            let cli_provided = context.cli_inputs.iter().any(|(k, _)| k == context.input_name);

            if !cli_provided {
                return ValidationOutcome::Error {
                    message: format!("Undefined input '{}'", context.full_name),
                    context: Some(format!(
                        "Input '{}' is not defined in the {} environment or provided via CLI",
                        context.input_name,
                        context.environment.unwrap_or("default")
                    )),
                    suggestion: Some(format!(
                        "Define '{}' in your manifest or provide it via CLI: --input {}=value",
                        context.input_name, context.input_name
                    )),
                    documentation_link: Some(
                        "https://docs.txtx.rs/manifests/environments".to_string(),
                    ),
                };
            }
        }

        ValidationOutcome::Pass
    }
}

/// Rule: Check for deprecated inputs
struct DeprecatedInputRule;

impl ManifestValidationRule for DeprecatedInputRule {
    fn name(&self) -> &'static str {
        "deprecated_input"
    }

    fn description(&self) -> &'static str {
        "Warns about deprecated input names"
    }

    fn check(&self, context: &ManifestValidationContext) -> ValidationOutcome {
        // List of deprecated inputs and their replacements
        let deprecated_inputs =
            [("api_key", "api_token"), ("endpoint_url", "api_url"), ("rpc_endpoint", "rpc_url")];

        for (deprecated, replacement) in deprecated_inputs {
            if context.input_name == deprecated {
                return ValidationOutcome::Warning {
                    message: format!("Input '{}' is deprecated", context.full_name),
                    suggestion: Some(format!("Use '{}' instead", replacement)),
                };
            }
        }

        ValidationOutcome::Pass
    }
}

/// Rule: Check for required inputs (strict mode only)
struct RequiredInputRule;

impl ManifestValidationRule for RequiredInputRule {
    fn name(&self) -> &'static str {
        "required_input"
    }

    fn description(&self) -> &'static str {
        "Ensures required inputs are provided in production environments"
    }

    fn check(&self, context: &ManifestValidationContext) -> ValidationOutcome {
        // In strict mode, certain inputs are required
        let required_for_production = ["api_url", "api_token", "chain_id"];

        // Only check if we're in production environment
        if context.environment == Some("production") || context.environment == Some("prod") {
            for required in required_for_production {
                // Check if this is a reference to a required input
                if context.input_name.contains(required)
                    && !context.effective_inputs.contains_key(required)
                {
                    return ValidationOutcome::Warning {
                        message: format!(
                            "Required input '{}' not found for production environment",
                            required
                        ),
                        suggestion: Some(format!(
                            "Ensure '{}' is defined in your production environment",
                            required
                        )),
                    };
                }
            }
        }

        ValidationOutcome::Pass
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
        production.insert("chain_id".to_string(), "1".to_string());
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
    fn test_undefined_input_detection() {
        let manifest = create_test_manifest();
        let mut result = ValidationResult::new();

        let input_refs =
            vec![LocatedInputRef { name: "env.undefined_var".to_string(), line: 10, column: 5 }];

        validate_inputs_against_manifest(
            &input_refs,
            "test content",
            &manifest,
            Some(&"production".to_string()),
            &mut result,
            "test.tx",
            &[],
            ManifestValidationConfig::default(),
        );

        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].message.contains("Undefined input"));
    }

    #[test]
    fn test_cli_input_precedence() {
        let manifest = create_test_manifest();
        let mut result = ValidationResult::new();

        let input_refs =
            vec![LocatedInputRef { name: "env.cli_provided".to_string(), line: 10, column: 5 }];

        let cli_inputs = vec![("cli_provided".to_string(), "cli-value".to_string())];

        validate_inputs_against_manifest(
            &input_refs,
            "test content",
            &manifest,
            Some(&"production".to_string()),
            &mut result,
            "test.tx",
            &cli_inputs,
            ManifestValidationConfig::default(),
        );

        // Should not error because CLI input is provided
        assert_eq!(result.errors.len(), 0);

        // Should have suggestion about CLI precedence
        assert_eq!(result.suggestions.len(), 1);
        assert!(result.suggestions[0].message.contains("CLI inputs provided"));
    }

    #[test]
    fn test_strict_mode_validation() {
        let manifest = create_test_manifest();
        let mut result = ValidationResult::new();

        // Reference exists but let's test strict mode warnings
        let input_refs =
            vec![LocatedInputRef { name: "env.api_url".to_string(), line: 10, column: 5 }];

        validate_inputs_against_manifest(
            &input_refs,
            "test content",
            &manifest,
            Some(&"production".to_string()),
            &mut result,
            "test.tx",
            &[],
            ManifestValidationConfig::strict(),
        );

        // In strict mode, we should get no errors for valid inputs
        assert_eq!(result.errors.len(), 0);
    }
}
