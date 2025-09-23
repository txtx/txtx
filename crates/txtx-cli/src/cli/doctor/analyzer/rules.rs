use std::collections::HashMap;
use txtx_core::manifest::WorkspaceManifest;
use txtx_core::validation::ValidationSuggestion;

/// Trait for validation rules
pub trait ValidationRule: Send + Sync {
    /// Unique name for this rule
    #[allow(dead_code)]
    fn name(&self) -> &'static str;

    /// Execute the validation check
    fn check(&self, context: &ValidationContext) -> ValidationOutcome;

    /// Optional description of what this rule validates
    #[allow(dead_code)]
    fn description(&self) -> &'static str {
        "No description provided"
    }
}

/// Context passed to validation rules
#[allow(dead_code)]
pub struct ValidationContext<'a> {
    pub input_name: &'a str,
    pub full_name: &'a str, // e.g., "input.my_var"
    pub manifest: &'a WorkspaceManifest,
    pub environment: Option<&'a str>,
    pub effective_inputs: &'a HashMap<String, String>,
    pub cli_inputs: &'a [(String, String)],
    pub content: &'a str,
    pub file_path: &'a str,
}

/// Outcome of a validation rule
pub enum ValidationOutcome {
    Pass,
    Error {
        message: String,
        context: Option<String>,
        suggestion: Option<ValidationSuggestion>,
        documentation_link: Option<String>,
    },
    Warning {
        message: String,
        suggestion: Option<ValidationSuggestion>,
    },
}

/// Rule: Check if input is defined in the manifest
pub struct InputDefinedRule;

impl ValidationRule for InputDefinedRule {
    fn name(&self) -> &'static str {
        "input_defined"
    }

    fn description(&self) -> &'static str {
        "Validates that all input references are defined in the manifest"
    }

    fn check(&self, ctx: &ValidationContext) -> ValidationOutcome {
        if ctx.effective_inputs.contains_key(ctx.input_name) {
            ValidationOutcome::Pass
        } else {
            let env_name = ctx.environment.unwrap_or("global");
            let mut context_msg = format!("Add '{}' to your txtx.yml file", ctx.input_name);

            if ctx.environment.is_some() && ctx.environment != Some("global") {
                context_msg.push_str(" (consider adding to 'global' if used across environments)");
            }

            ValidationOutcome::Error {
                message: format!(
                    "Input '{}' is not defined in environment '{}' (including inherited values)",
                    ctx.full_name, env_name
                ),
                context: Some(context_msg),
                suggestion: Some(ValidationSuggestion {
                    message: "Add the missing input to your environment".to_string(),
                    example: Some(format!(
                        "environments:\n  {}:\n    {}: \"<value>\"{}",
                        env_name,
                        ctx.input_name,
                        if ctx.environment.is_some() && ctx.environment != Some("global") {
                            "\n  # Or add to 'global' for all environments"
                        } else {
                            ""
                        }
                    )),
                }),
                documentation_link: Some(
                    "https://docs.txtx.sh/concepts/manifest#environments".to_string(),
                ),
            }
        }
    }
}

/// Rule: Check input naming conventions
pub struct InputNamingConventionRule;

impl ValidationRule for InputNamingConventionRule {
    fn name(&self) -> &'static str {
        "input_naming_convention"
    }

    fn description(&self) -> &'static str {
        "Validates input names follow recommended conventions"
    }

    fn check(&self, ctx: &ValidationContext) -> ValidationOutcome {
        // Check for common naming issues
        if ctx.input_name.starts_with('_') {
            ValidationOutcome::Warning {
                message: format!(
                    "Input '{}' starts with underscore, which may indicate a private variable",
                    ctx.input_name
                ),
                suggestion: Some(ValidationSuggestion {
                    message: "Consider using a different naming convention".to_string(),
                    example: Some(format!("Rename to: {}", ctx.input_name.trim_start_matches('_'))),
                }),
            }
        } else if ctx.input_name.contains('-') {
            ValidationOutcome::Warning {
                message: format!(
                    "Input '{}' contains hyphens, consider using underscores",
                    ctx.input_name
                ),
                suggestion: Some(ValidationSuggestion {
                    message: "Use underscores instead of hyphens for consistency".to_string(),
                    example: Some(format!("Rename to: {}", ctx.input_name.replace('-', "_"))),
                }),
            }
        } else if !ctx.input_name.chars().next().map(|c| c.is_lowercase()).unwrap_or(false) {
            ValidationOutcome::Warning {
                message: format!("Input '{}' should start with a lowercase letter", ctx.input_name),
                suggestion: Some(ValidationSuggestion {
                    message: "Use lowercase for input names".to_string(),
                    example: Some(format!("Rename to: {}", ctx.input_name.to_lowercase())),
                }),
            }
        } else {
            ValidationOutcome::Pass
        }
    }
}

/// Rule: Check for CLI input overrides
pub struct CliInputOverrideRule;

impl ValidationRule for CliInputOverrideRule {
    fn name(&self) -> &'static str {
        "cli_input_override"
    }

    fn description(&self) -> &'static str {
        "Checks if inputs are overridden by CLI arguments"
    }

    fn check(&self, ctx: &ValidationContext) -> ValidationOutcome {
        // Only check if input is defined
        if !ctx.effective_inputs.contains_key(ctx.input_name) {
            return ValidationOutcome::Pass;
        }

        // Check if this input is overridden by CLI
        let is_overridden = ctx.cli_inputs.iter().any(|(k, _)| k == ctx.input_name);

        if is_overridden {
            // Check if there's a manifest value being overridden
            let has_manifest_value = ctx
                .manifest
                .environments
                .get(ctx.environment.unwrap_or("global"))
                .and_then(|env| env.get(ctx.input_name))
                .is_some();

            if has_manifest_value {
                ValidationOutcome::Warning {
                    message: format!(
                        "Input '{}' is defined in manifest but overridden by CLI argument",
                        ctx.input_name
                    ),
                    suggestion: Some(ValidationSuggestion {
                        message: "CLI inputs take precedence over environment values".to_string(),
                        example: None,
                    }),
                }
            } else {
                ValidationOutcome::Pass
            }
        } else {
            ValidationOutcome::Pass
        }
    }
}

/// Rule: Check for sensitive data in input names
pub struct SensitiveDataRule;

impl ValidationRule for SensitiveDataRule {
    fn name(&self) -> &'static str {
        "no_sensitive_data"
    }

    fn description(&self) -> &'static str {
        "Warns about potentially sensitive data in input names"
    }

    fn check(&self, ctx: &ValidationContext) -> ValidationOutcome {
        let sensitive_patterns = [
            "password",
            "passwd",
            "pwd",
            "secret",
            "key",
            "token",
            "credential",
            "cred",
            "private",
            "priv",
        ];

        let lower_name = ctx.input_name.to_lowercase();

        if sensitive_patterns.iter().any(|pattern| lower_name.contains(pattern)) {
            ValidationOutcome::Warning {
                message: format!(
                    "Input '{}' appears to contain sensitive information",
                    ctx.input_name
                ),
                suggestion: Some(ValidationSuggestion {
                    message: "Consider using environment variables or a secure secret manager"
                        .to_string(),
                    example: Some(format!(
                        "# Set via environment variable:\nexport {}=\"${{VAULT_SECRET}}\"",
                        ctx.input_name.to_uppercase()
                    )),
                }),
            }
        } else {
            ValidationOutcome::Pass
        }
    }
}

/// Get the default set of validation rules
pub fn get_default_rules() -> Vec<Box<dyn ValidationRule>> {
    vec![
        Box::new(InputDefinedRule),
        Box::new(InputNamingConventionRule),
        Box::new(CliInputOverrideRule),
        Box::new(SensitiveDataRule),
    ]
}

/// Get strict validation rules for production environments
#[allow(dead_code)]
pub fn get_strict_rules() -> Vec<Box<dyn ValidationRule>> {
    let mut rules = get_default_rules();

    // Add production-specific rules
    rules.push(Box::new(NoDefaultValuesRule));
    rules.push(Box::new(RequiredProductionInputsRule));

    rules
}

/// Rule: No default/example values in production
#[allow(dead_code)]
struct NoDefaultValuesRule;

impl ValidationRule for NoDefaultValuesRule {
    fn name(&self) -> &'static str {
        "no_default_values"
    }

    fn description(&self) -> &'static str {
        "Ensures no default or example values are used in production"
    }

    fn check(&self, ctx: &ValidationContext) -> ValidationOutcome {
        if let Some(value) = ctx.effective_inputs.get(ctx.input_name) {
            let lower_value = value.to_lowercase();

            if lower_value.contains("default")
                || lower_value.contains("example")
                || lower_value.contains("test")
                || lower_value.contains("demo")
                || value == "changeme"
                || value == "replaceme"
            {
                ValidationOutcome::Error {
                    message: format!(
                        "Input '{}' appears to have a placeholder value: '{}'",
                        ctx.input_name, value
                    ),
                    context: Some("Production environments require real values".to_string()),
                    suggestion: Some(ValidationSuggestion {
                        message: "Replace with actual production value".to_string(),
                        example: None,
                    }),
                    documentation_link: None,
                }
            } else {
                ValidationOutcome::Pass
            }
        } else {
            ValidationOutcome::Pass
        }
    }
}

/// Rule: Ensure critical inputs are present in production
#[allow(dead_code)]
struct RequiredProductionInputsRule;

impl ValidationRule for RequiredProductionInputsRule {
    fn name(&self) -> &'static str {
        "required_production_inputs"
    }

    fn description(&self) -> &'static str {
        "Ensures critical inputs are defined for production"
    }

    fn check(&self, ctx: &ValidationContext) -> ValidationOutcome {
        // Only check in production environment
        if ctx.environment != Some("production") && ctx.environment != Some("prod") {
            return ValidationOutcome::Pass;
        }

        // Common required inputs for production
        let required_inputs = [
            "api_key",
            "api_secret",
            "api_token",
            "database_url",
            "db_url",
            "db_connection",
            "rpc_url",
            "rpc_endpoint",
            "private_key",
            "signing_key",
        ];

        // Check if this is one of the required inputs and it's missing
        if required_inputs.iter().any(|&req| ctx.input_name == req)
            && !ctx.effective_inputs.contains_key(ctx.input_name)
        {
            ValidationOutcome::Error {
                message: format!("Required production input '{}' is not defined", ctx.input_name),
                context: Some("This input is critical for production deployments".to_string()),
                suggestion: Some(ValidationSuggestion {
                    message: "Add this input to your production environment".to_string(),
                    example: Some(format!(
                        "environments:\n  production:\n    {}: \"<secure-value>\"",
                        ctx.input_name
                    )),
                }),
                documentation_link: None,
            }
        } else {
            ValidationOutcome::Pass
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use txtx_addon_kit::indexmap::IndexMap;
    use txtx_core::manifest::WorkspaceManifest;

    fn create_test_context<'a>(
        input_name: &'a str,
        full_name: &'a str,
        effective_inputs: &'a HashMap<String, String>,
        cli_inputs: &'a [(String, String)],
        environment: Option<&'a str>,
        manifest: &'a WorkspaceManifest,
    ) -> ValidationContext<'a> {
        ValidationContext {
            input_name,
            full_name,
            manifest,
            environment,
            effective_inputs,
            cli_inputs,
            content: "test content",
            file_path: "test.tx",
        }
    }

    #[test]
    fn test_input_defined_rule_pass() {
        let rule = InputDefinedRule;
        let mut inputs = HashMap::new();
        inputs.insert("my_var".to_string(), "value".to_string());
        let manifest = WorkspaceManifest::new("test".to_string());

        let ctx =
            create_test_context("my_var", "input.my_var", &inputs, &[], Some("test"), &manifest);

        match rule.check(&ctx) {
            ValidationOutcome::Pass => {}
            _ => panic!("Expected Pass"),
        }
    }

    #[test]
    fn test_input_defined_rule_fail() {
        let rule = InputDefinedRule;
        let inputs = HashMap::new();
        let manifest = WorkspaceManifest::new("test".to_string());

        let ctx = create_test_context(
            "missing_var",
            "input.missing_var",
            &inputs,
            &[],
            Some("test"),
            &manifest,
        );

        match rule.check(&ctx) {
            ValidationOutcome::Error { message, .. } => {
                assert!(message.contains("not defined"));
            }
            _ => panic!("Expected Error"),
        }
    }

    #[test]
    fn test_naming_convention_underscore() {
        let rule = InputNamingConventionRule;
        let inputs = HashMap::new();
        let manifest = WorkspaceManifest::new("test".to_string());

        let ctx = create_test_context(
            "_private_var",
            "input._private_var",
            &inputs,
            &[],
            Some("test"),
            &manifest,
        );

        match rule.check(&ctx) {
            ValidationOutcome::Warning { message, .. } => {
                assert!(message.contains("underscore"));
            }
            _ => panic!("Expected Warning"),
        }
    }

    #[test]
    fn test_cli_override_rule() {
        let rule = CliInputOverrideRule;
        let mut inputs = HashMap::new();
        inputs.insert("api_key".to_string(), "manifest_value".to_string());

        let mut manifest = WorkspaceManifest::new("test".to_string());
        let mut env_inputs = IndexMap::new();
        env_inputs.insert("api_key".to_string(), "manifest_value".to_string());
        manifest.environments.insert("test".to_string(), env_inputs);

        let cli_inputs = vec![("api_key".to_string(), "cli_value".to_string())];
        let ctx = ValidationContext {
            input_name: "api_key",
            full_name: "input.api_key",
            manifest: &manifest,
            environment: Some("test"),
            effective_inputs: &inputs,
            cli_inputs: &cli_inputs,
            content: "test content",
            file_path: "test.tx",
        };

        match rule.check(&ctx) {
            ValidationOutcome::Warning { message, .. } => {
                assert!(message.contains("overridden by CLI"));
            }
            _ => panic!("Expected Warning"),
        }
    }

    #[test]
    fn test_sensitive_data_rule() {
        let rule = SensitiveDataRule;
        let inputs = HashMap::new();
        let manifest = WorkspaceManifest::new("test".to_string());

        let test_cases = vec!["password", "api_key", "secret_token", "private_key"];

        for sensitive_name in test_cases {
            let full_name = format!("input.{}", sensitive_name);
            let ctx = create_test_context(
                sensitive_name,
                &full_name,
                &inputs,
                &[],
                Some("test"),
                &manifest,
            );

            match rule.check(&ctx) {
                ValidationOutcome::Warning { message, .. } => {
                    assert!(message.contains("sensitive"));
                }
                _ => panic!("Expected Warning for {}", sensitive_name),
            }
        }
    }
}
