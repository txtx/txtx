//! Doctor-specific validation rules
//!
//! These rules provide additional validation beyond the basic manifest validation,
//! including naming conventions, security checks, and production requirements.

use super::manifest_validator::{
    ManifestValidationContext, ManifestValidationRule, ValidationOutcome,
};

/// Rule: Check input naming conventions
pub struct InputNamingConventionRule;

impl ManifestValidationRule for InputNamingConventionRule {
    fn name(&self) -> &'static str {
        "input_naming_convention"
    }

    fn description(&self) -> &'static str {
        "Validates that inputs follow naming conventions"
    }

    fn check(&self, ctx: &ManifestValidationContext) -> ValidationOutcome {
        // Check for common naming issues
        if ctx.input_name.contains('-') {
            return ValidationOutcome::Warning {
                message: format!(
                    "Input '{}' contains hyphens. Consider using underscores for consistency",
                    ctx.full_name
                ),
                suggestion: Some(format!("Rename to '{}'", ctx.full_name.replace('-', "_"))),
            };
        }

        if ctx.input_name.chars().any(|c| c.is_uppercase()) {
            return ValidationOutcome::Warning {
                message: format!(
                    "Input '{}' contains uppercase letters. Consider using lowercase for consistency",
                    ctx.full_name
                ),
                suggestion: Some(format!(
                    "Rename to '{}'", 
                    ctx.full_name.to_lowercase()
                )),
            };
        }

        ValidationOutcome::Pass
    }
}

/// Rule: CLI input override warnings
pub struct CliInputOverrideRule;

impl ManifestValidationRule for CliInputOverrideRule {
    fn name(&self) -> &'static str {
        "cli_input_override"
    }

    fn description(&self) -> &'static str {
        "Warns when CLI inputs override environment values"
    }

    fn check(&self, ctx: &ManifestValidationContext) -> ValidationOutcome {
        // Check if this input is being overridden by CLI
        let cli_override = ctx.cli_inputs.iter().find(|(k, _)| k == ctx.input_name);

        if let Some((_, cli_value)) = cli_override {
            if let Some(env_value) = ctx.effective_inputs.get(ctx.input_name) {
                if cli_value != env_value {
                    return ValidationOutcome::Warning {
                        message: format!(
                            "CLI input '{}' overrides environment value",
                            ctx.input_name
                        ),
                        suggestion: Some(format!(
                            "CLI value '{}' will be used instead of environment value '{}'",
                            cli_value, env_value
                        )),
                    };
                }
            }
        }

        ValidationOutcome::Pass
    }
}

/// Rule: Sensitive data detection
pub struct SensitiveDataRule;

impl ManifestValidationRule for SensitiveDataRule {
    fn name(&self) -> &'static str {
        "sensitive_data"
    }

    fn description(&self) -> &'static str {
        "Detects potential sensitive data in inputs"
    }

    fn check(&self, ctx: &ManifestValidationContext) -> ValidationOutcome {
        let sensitive_patterns = [
            "password",
            "passwd",
            "secret",
            "token",
            "key",
            "credential",
            "private",
            "auth",
            "apikey",
            "api_key",
            "access_key",
        ];

        let lower_name = ctx.input_name.to_lowercase();

        for pattern in &sensitive_patterns {
            if lower_name.contains(pattern) {
                if let Some(value) = ctx.effective_inputs.get(ctx.input_name) {
                    // Check if it looks like a placeholder
                    if value.starts_with('<') && value.ends_with('>') {
                        return ValidationOutcome::Warning {
                            message: format!(
                                "Input '{}' appears to contain sensitive data with placeholder value",
                                ctx.full_name
                            ),
                            suggestion: Some(
                                "Ensure this value is properly set before deployment".to_string()
                            ),
                        };
                    }

                    // Check if it's hardcoded (not a reference)
                    if !value.starts_with("${") && !value.starts_with("env.") {
                        return ValidationOutcome::Warning {
                            message: format!(
                                "Input '{}' may contain hardcoded sensitive data",
                                ctx.full_name
                            ),
                            suggestion: Some(
                                "Consider using environment variables or secure secret management"
                                    .to_string(),
                            ),
                        };
                    }
                }
                break;
            }
        }

        ValidationOutcome::Pass
    }
}

/// Rule: No default values (for strict environments)
pub struct NoDefaultValuesRule;

impl ManifestValidationRule for NoDefaultValuesRule {
    fn name(&self) -> &'static str {
        "no_default_values"
    }

    fn description(&self) -> &'static str {
        "Ensures production environments don't use default values"
    }

    fn check(&self, ctx: &ManifestValidationContext) -> ValidationOutcome {
        // Only apply in production environments
        if ctx.environment != Some("production") && ctx.environment != Some("prod") {
            return ValidationOutcome::Pass;
        }

        // Check if this value comes from defaults
        if let Some(defaults) = ctx.manifest.environments.get("defaults") {
            if let Some(default_value) = defaults.get(ctx.input_name) {
                if let Some(env_value) = ctx.effective_inputs.get(ctx.input_name) {
                    if default_value == env_value {
                        return ValidationOutcome::Warning {
                            message: format!(
                                "Production environment is using default value for '{}'",
                                ctx.full_name
                            ),
                            suggestion: Some(
                                "Define an explicit value for production environment".to_string(),
                            ),
                        };
                    }
                }
            }
        }

        ValidationOutcome::Pass
    }
}

/// Rule: Required production inputs
pub struct RequiredProductionInputsRule;

impl ManifestValidationRule for RequiredProductionInputsRule {
    fn name(&self) -> &'static str {
        "required_production_inputs"
    }

    fn description(&self) -> &'static str {
        "Ensures required inputs are present in production"
    }

    fn check(&self, ctx: &ManifestValidationContext) -> ValidationOutcome {
        // Only apply in production environments
        if ctx.environment != Some("production") && ctx.environment != Some("prod") {
            return ValidationOutcome::Pass;
        }

        // List of patterns that indicate required production inputs
        let required_patterns = [
            "api_url",
            "api_endpoint",
            "base_url",
            "api_token",
            "api_key",
            "auth_token",
            "chain_id",
            "network_id",
        ];

        // Check if this input matches a required pattern
        let lower_name = ctx.input_name.to_lowercase();
        for pattern in &required_patterns {
            if lower_name.contains(pattern) && !ctx.effective_inputs.contains_key(ctx.input_name) {
                return ValidationOutcome::Error {
                    message: format!(
                        "Required production input '{}' is not defined",
                        ctx.full_name
                    ),
                    context: Some(
                        "Production environments must define all API endpoints and authentication tokens".to_string()
                    ),
                    suggestion: Some(
                        "Add this input to your production environment configuration".to_string()
                    ),
                    documentation_link: Some(
                        "https://docs.txtx.sh/deployment/production".to_string()
                    ),
                };
            }
        }

        ValidationOutcome::Pass
    }
}

/// Get the default doctor validation rules
pub fn get_doctor_rules() -> Vec<Box<dyn ManifestValidationRule>> {
    vec![
        Box::new(InputNamingConventionRule),
        Box::new(CliInputOverrideRule),
        Box::new(SensitiveDataRule),
    ]
}

/// Get strict doctor validation rules (for production)
pub fn get_strict_doctor_rules() -> Vec<Box<dyn ManifestValidationRule>> {
    vec![
        Box::new(InputNamingConventionRule),
        Box::new(CliInputOverrideRule),
        Box::new(SensitiveDataRule),
        Box::new(NoDefaultValuesRule),
        Box::new(RequiredProductionInputsRule),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::WorkspaceManifest;
    use std::collections::HashMap;
    use txtx_addon_kit::indexmap::IndexMap;

    fn create_test_context<'a>(
        input_name: &'a str,
        full_name: &'a str,
        manifest: &'a WorkspaceManifest,
        effective_inputs: &'a HashMap<String, String>,
    ) -> ManifestValidationContext<'a> {
        ManifestValidationContext {
            input_name,
            full_name,
            manifest,
            environment: Some("production"),
            effective_inputs,
            cli_inputs: &[],
            content: "",
            file_path: "test.tx",
        }
    }

    #[test]
    fn test_naming_convention_rule() {
        let manifest = WorkspaceManifest {
            name: "test".to_string(),
            id: "test".to_string(),
            runbooks: vec![],
            environments: IndexMap::new(),
            location: None,
        };

        let inputs = HashMap::new();
        let rule = InputNamingConventionRule;

        // Test hyphenated name
        let ctx = create_test_context("api-key", "env.api-key", &manifest, &inputs);
        match rule.check(&ctx) {
            ValidationOutcome::Warning { message, .. } => {
                assert!(message.contains("hyphens"));
            }
            _ => panic!("Expected warning for hyphenated name"),
        }

        // Test uppercase name
        let ctx = create_test_context("ApiKey", "env.ApiKey", &manifest, &inputs);
        match rule.check(&ctx) {
            ValidationOutcome::Warning { message, .. } => {
                assert!(message.contains("uppercase"));
            }
            _ => panic!("Expected warning for uppercase name"),
        }

        // Test valid name
        let ctx = create_test_context("api_key", "env.api_key", &manifest, &inputs);
        match rule.check(&ctx) {
            ValidationOutcome::Pass => {}
            _ => panic!("Expected pass for valid name"),
        }
    }

    #[test]
    fn test_sensitive_data_rule() {
        let manifest = WorkspaceManifest {
            name: "test".to_string(),
            id: "test".to_string(),
            runbooks: vec![],
            environments: IndexMap::new(),
            location: None,
        };

        let mut inputs = HashMap::new();
        inputs.insert("api_key".to_string(), "hardcoded123".to_string());

        let rule = SensitiveDataRule;
        let ctx = create_test_context("api_key", "env.api_key", &manifest, &inputs);

        match rule.check(&ctx) {
            ValidationOutcome::Warning { message, .. } => {
                assert!(message.contains("hardcoded sensitive data"));
            }
            _ => panic!("Expected warning for hardcoded sensitive data"),
        }
    }
}
