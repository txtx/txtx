use super::rules::{ValidationContext, ValidationOutcome, ValidationRule};
use std::collections::HashMap;
use std::path::Path;
use txtx_core::{
    manifest::WorkspaceManifest,
    validation::{LocatedInputRef, ValidationError, ValidationResult},
};

/// Data-driven input validator
#[allow(dead_code)]
pub struct InputValidator {
    #[allow(dead_code)]
    rules: Vec<Box<dyn ValidationRule>>,
}

#[allow(dead_code)]
impl InputValidator {
    /// Create a new validator with the default rule set
    pub fn new() -> Self {
        Self { rules: super::rules::get_default_rules() }
    }

    /// Create a validator with a custom set of rules
    pub fn with_rules(rules: Vec<Box<dyn ValidationRule>>) -> Self {
        Self { rules }
    }

    /// Create a validator for strict/production environments
    pub fn strict() -> Self {
        Self { rules: super::rules::get_strict_rules() }
    }

    /// Add additional rules to the validator
    pub fn add_rule(mut self, rule: Box<dyn ValidationRule>) -> Self {
        self.rules.push(rule);
        self
    }

    /// Add multiple additional rules
    pub fn add_rules(mut self, rules: Vec<Box<dyn ValidationRule>>) -> Self {
        self.rules.extend(rules);
        self
    }

    /// Get information about loaded rules
    pub fn describe_rules(&self) -> Vec<(&'static str, &'static str)> {
        self.rules.iter().map(|rule| (rule.name(), rule.description())).collect()
    }

    /// Main validation entry point - data-driven approach
    pub fn validate_inputs(
        &self,
        input_refs: &[LocatedInputRef],
        content: &str,
        manifest: &WorkspaceManifest,
        environment: Option<&String>,
        result: &mut ValidationResult,
        file_path: &Path,
        cli_inputs: &[(String, String)],
    ) {
        // Build effective inputs from environment hierarchy
        let effective_inputs = build_effective_inputs(manifest, environment, cli_inputs);

        // Add CLI precedence message if applicable
        if !cli_inputs.is_empty() {
            result.suggestions.push(txtx_core::validation::ValidationSuggestion {
                message: format!(
                    "{} CLI inputs provided. CLI inputs take precedence over environment values.",
                    cli_inputs.len()
                ),
                example: None,
            });
        }

        // Process each input reference through all rules
        for input_ref in input_refs {
            let input_name = strip_input_prefix(&input_ref.name);

            // Create validation context
            let context = ValidationContext {
                input_name,
                full_name: &input_ref.name,
                manifest,
                environment: environment.as_ref().map(|s| s.as_str()),
                effective_inputs: &effective_inputs,
                cli_inputs,
                content,
                file_path: &file_path.to_string_lossy(),
            };

            // Run each rule and process outcomes
            for rule in &self.rules {
                match rule.check(&context) {
                    ValidationOutcome::Pass => continue,

                    ValidationOutcome::Error {
                        message,
                        context: ctx,
                        suggestion,
                        documentation_link,
                    } => {
                        // Find location in source
                        let (line, column) = find_input_location(content, &input_ref.name)
                            .map(|(l, c)| (Some(l), Some(c)))
                            .unwrap_or((None, None));

                        result.errors.push(ValidationError {
                            message,
                            file: file_path.to_string_lossy().to_string(),
                            line,
                            column,
                            context: ctx,
                            documentation_link,
                        });

                        if let Some(suggestion) = suggestion {
                            result.suggestions.push(suggestion);
                        }
                    }

                    ValidationOutcome::Warning { message, suggestion } => {
                        // Find location in source
                        let (line, column) = find_input_location(content, &input_ref.name)
                            .map(|(l, c)| (Some(l), Some(c)))
                            .unwrap_or((None, None));

                        result.warnings.push(txtx_core::validation::ValidationWarning {
                            message,
                            file: file_path.to_string_lossy().to_string(),
                            line,
                            column,
                            suggestion: suggestion.as_ref().map(|s| s.message.clone()),
                        });

                        if let Some(suggestion) = suggestion {
                            result.suggestions.push(suggestion);
                        }
                    }
                }
            }
        }
    }
}

impl Default for InputValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Build effective inputs by merging global, environment, and CLI inputs
#[allow(dead_code)]
fn build_effective_inputs(
    manifest: &WorkspaceManifest,
    environment: Option<&String>,
    cli_inputs: &[(String, String)],
) -> HashMap<String, String> {
    let mut effective_inputs = HashMap::new();

    // First, add all inputs from global environment
    if let Some(global_inputs) = manifest.environments.get("global") {
        for (key, value) in global_inputs {
            effective_inputs.insert(key.clone(), value.clone());
        }
    }

    // Then, overlay the specific environment (if different from global)
    if let Some(env_name) = environment {
        if env_name != "global" {
            if let Some(env_inputs) = manifest.environments.get(env_name) {
                for (key, value) in env_inputs {
                    effective_inputs.insert(key.clone(), value.clone());
                }
            }
        }
    }

    // Apply CLI input overrides
    for (key, value) in cli_inputs {
        effective_inputs.insert(key.clone(), value.clone());
    }

    effective_inputs
}

/// Strip "input." prefix from input name if present
#[allow(dead_code)]
fn strip_input_prefix(name: &str) -> &str {
    if name.starts_with("input.") {
        &name[6..]
    } else {
        name
    }
}

/// Helper function to find the line and column of an input reference in the source
#[allow(dead_code)]
fn find_input_location(content: &str, input_name: &str) -> Option<(usize, usize)> {
    for (line_idx, line) in content.lines().enumerate() {
        if let Some(col_idx) = line.find(input_name) {
            return Some((line_idx + 1, col_idx + 1));
        }
    }
    None
}
