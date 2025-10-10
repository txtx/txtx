//! Linter-specific validation rules
//!
//! These rules provide additional validation beyond the basic manifest validation,
//! including naming conventions, security checks, and production requirements.

use super::manifest_validator::{
    ManifestValidationContext, ManifestValidationRule, ValidationOutcome,
};
use super::rule_id::{CoreRuleId, RuleIdentifier};

/// Rule: Check input naming conventions
pub struct InputNamingConventionRule;

impl ManifestValidationRule for InputNamingConventionRule {
    fn id(&self) -> RuleIdentifier {
        RuleIdentifier::Core(CoreRuleId::InputNamingConvention)
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
    fn id(&self) -> RuleIdentifier {
        RuleIdentifier::Core(CoreRuleId::CliInputOverride)
    }

    fn description(&self) -> &'static str {
        "Warns when CLI inputs override environment values"
    }

    fn check(&self, ctx: &ManifestValidationContext) -> ValidationOutcome {
        match (
            ctx.cli_inputs.iter().find(|(k, _)| k == ctx.input_name),
            ctx.effective_inputs.get(ctx.input_name),
        ) {
            (Some((_, cli_value)), Some(env_value)) if cli_value != env_value => {
                ValidationOutcome::Warning {
                    message: format!("CLI input '{}' overrides environment value", ctx.input_name),
                    suggestion: Some(format!(
                        "CLI value '{}' will be used instead of environment value '{}'",
                        cli_value, env_value
                    )),
                }
            }
            _ => ValidationOutcome::Pass,
        }
    }
}

/// Rule: Sensitive data detection
pub struct SensitiveDataRule;

impl ManifestValidationRule for SensitiveDataRule {
    fn id(&self) -> RuleIdentifier {
        RuleIdentifier::Core(CoreRuleId::SensitiveData)
    }

    fn description(&self) -> &'static str {
        "Detects potential sensitive data in inputs"
    }

    fn check(&self, ctx: &ManifestValidationContext) -> ValidationOutcome {
        const SENSITIVE_PATTERNS: &[&str] = &[
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

        if !SENSITIVE_PATTERNS.iter().any(|&p| lower_name.contains(p)) {
            return ValidationOutcome::Pass;
        }

        let Some(value) = ctx.effective_inputs.get(ctx.input_name) else {
            return ValidationOutcome::Pass;
        };

        if value.starts_with('<') && value.ends_with('>') {
            return ValidationOutcome::Warning {
                message: format!(
                    "Input '{}' appears to contain sensitive data with placeholder value",
                    ctx.full_name
                ),
                suggestion: Some("Ensure this value is properly set before deployment".to_string()),
            };
        }

        if !value.starts_with("${") && !value.starts_with("input.") {
            return ValidationOutcome::Warning {
                message: format!("Input '{}' may contain hardcoded sensitive data", ctx.full_name),
                suggestion: Some(
                    "Consider using environment variables or secure secret management".to_string(),
                ),
            };
        }

        ValidationOutcome::Pass
    }
}

/// Rule: No default values (for strict environments)
pub struct NoDefaultValuesRule;

impl ManifestValidationRule for NoDefaultValuesRule {
    fn id(&self) -> RuleIdentifier {
        RuleIdentifier::Core(CoreRuleId::NoDefaultValues)
    }

    fn description(&self) -> &'static str {
        "Ensures production environments don't use default values"
    }

    fn check(&self, ctx: &ManifestValidationContext) -> ValidationOutcome {
        // Only apply in production environments
        if !matches!(ctx.environment, Some("production" | "prod")) {
            return ValidationOutcome::Pass;
        }

        match (
            ctx.manifest.environments.get("defaults").and_then(|d| d.get(ctx.input_name)),
            ctx.effective_inputs.get(ctx.input_name),
        ) {
            (Some(default_value), Some(env_value)) if default_value == env_value => {
                ValidationOutcome::Warning {
                    message: format!(
                        "Production environment is using default value for '{}'",
                        ctx.full_name
                    ),
                    suggestion: Some(
                        "Define an explicit value for production environment".to_string(),
                    ),
                }
            }
            _ => ValidationOutcome::Pass,
        }
    }
}

/// Rule: Required production inputs
pub struct RequiredProductionInputsRule;

impl ManifestValidationRule for RequiredProductionInputsRule {
    fn id(&self) -> RuleIdentifier {
        RuleIdentifier::Core(CoreRuleId::RequiredProductionInputs)
    }

    fn description(&self) -> &'static str {
        "Ensures required inputs are present in production"
    }

    fn check(&self, ctx: &ManifestValidationContext) -> ValidationOutcome {
        const REQUIRED_PATTERNS: &[&str] = &[
            "api_url",
            "api_endpoint",
            "base_url",
            "api_token",
            "api_key",
            "auth_token",
            "chain_id",
            "network_id",
        ];

        // Only apply in production environments
        if !matches!(ctx.environment, Some("production" | "prod")) {
            return ValidationOutcome::Pass;
        }

        let lower_name = ctx.input_name.to_lowercase();

        if REQUIRED_PATTERNS.iter().any(|&p| lower_name.contains(p))
            && !ctx.effective_inputs.contains_key(ctx.input_name)
        {
            ValidationOutcome::Error {
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
        }
        } else {
            ValidationOutcome::Pass
        }
    }
}

/// Get the default linter validation rules
pub fn get_linter_rules() -> Vec<Box<dyn ManifestValidationRule>> {
    vec![
        Box::new(InputNamingConventionRule),
        Box::new(CliInputOverrideRule),
        Box::new(SensitiveDataRule),
    ]
}

/// Get strict linter validation rules (for production)
pub fn get_strict_linter_rules() -> Vec<Box<dyn ManifestValidationRule>> {
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
    use std::collections::{HashMap, HashSet};
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
            active_addons: HashSet::new(),
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
        let ctx = create_test_context("api-key", "input.api-key", &manifest, &inputs);
        match rule.check(&ctx) {
            ValidationOutcome::Warning { message, .. } => {
                assert!(message.contains("hyphens"));
            }
            _ => panic!("Expected warning for hyphenated name"),
        }

        // Test uppercase name
        let ctx = create_test_context("ApiKey", "input.ApiKey", &manifest, &inputs);
        match rule.check(&ctx) {
            ValidationOutcome::Warning { message, .. } => {
                assert!(message.contains("uppercase"));
            }
            _ => panic!("Expected warning for uppercase name"),
        }

        // Test valid name
        let ctx = create_test_context("api_key", "input.api_key", &manifest, &inputs);
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
        let ctx = create_test_context("api_key", "input.api_key", &manifest, &inputs);

        match rule.check(&ctx) {
            ValidationOutcome::Warning { message, .. } => {
                assert!(message.contains("hardcoded sensitive data"));
            }
            _ => panic!("Expected warning for hardcoded sensitive data"),
        }
    }
}
