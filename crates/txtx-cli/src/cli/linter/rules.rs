//! Validation rules for txtx runbooks

use super::rule_id::CliRuleId;
use std::borrow::Cow;
use std::collections::HashMap;
use strum::{AsRefStr, Display, EnumIter, EnumString, IntoStaticStr};
use txtx_core::manifest::WorkspaceManifest;

// ============================================================================
// Core Types
// ============================================================================

/// Represents a validation issue found by a rule
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub rule: CliRuleId,
    pub severity: Severity,
    pub message: Cow<'static, str>,
    pub help: Option<Cow<'static, str>>,
    pub example: Option<String>,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    AsRefStr,      // Provides as_ref() -> &str
    Display,       // Provides to_string()
    EnumString,    // Provides from_str()
    IntoStaticStr, // Provides into() -> &'static str
    EnumIter,      // Provides iter() over all variants
)]
#[strum(serialize_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

/// Input-specific context within a validation check
pub struct InputInfo {
    pub name: String,
    pub full_name: String,
}

/// Context passed to validation rules
pub struct ValidationContext {
    pub manifest: WorkspaceManifest,
    pub environment: Option<String>,
    pub effective_inputs: HashMap<String, String>,
    pub cli_inputs: Vec<(String, String)>,
    pub content: String,
    pub file_path: String,
    pub input: InputInfo,
}

// ============================================================================
// Data-Driven Rule Configuration
// ============================================================================

const SENSITIVE_PATTERNS: &[&str] = &["password", "secret", "key", "token", "credential"];

// ============================================================================
// Rule Implementations
// ============================================================================

type RuleFn = fn(&ValidationContext) -> Option<ValidationIssue>;

fn validate_input_defined(ctx: &ValidationContext) -> Option<ValidationIssue> {
    if ctx.effective_inputs.contains_key(&ctx.input.name) {
        return None;
    }

    let env_name = ctx.environment.as_deref().unwrap_or("global");
    Some(ValidationIssue {
        rule: CliRuleId::InputDefined,
        severity: Severity::Error,
        message: Cow::Owned(format!(
            "Input '{}' is not defined in environment '{}'",
            ctx.input.full_name, env_name
        )),
        help: Some(Cow::Owned(format!(
            "Add '{}' to your txtx.yml file",
            ctx.input.name
        ))),
        example: Some(format!(
            "environments:\n  {}:\n    inputs:\n      {}: \"<value>\"",
            env_name, ctx.input.name
        )),
    })
}

fn validate_naming_convention(ctx: &ValidationContext) -> Option<ValidationIssue> {
    if ctx.input.name.starts_with('_') {
        return Some(ValidationIssue {
            rule: CliRuleId::InputNamingConvention,
            severity: Severity::Warning,
            message: Cow::Owned(format!(
                "Input '{}' starts with underscore",
                ctx.input.name
            )),
            help: Some(Cow::Borrowed(
                "Consider using a different naming convention",
            )),
            example: Some(ctx.input.name.trim_start_matches('_').to_string()),
        });
    }

    if ctx.input.name.contains('-') {
        return Some(ValidationIssue {
            rule: CliRuleId::InputNamingConvention,
            severity: Severity::Warning,
            message: Cow::Owned(format!("Input '{}' contains hyphens", ctx.input.name)),
            help: Some(Cow::Borrowed("Use underscores instead of hyphens")),
            example: Some(ctx.input.name.replace('-', "_")),
        });
    }

    None
}

fn validate_cli_override(ctx: &ValidationContext) -> Option<ValidationIssue> {
    if !ctx.effective_inputs.contains_key(&ctx.input.name) {
        return None;
    }

    let is_overridden = ctx.cli_inputs.iter().any(|(k, _)| k == &ctx.input.name);
    if is_overridden {
        Some(ValidationIssue {
            rule: CliRuleId::CliInputOverride,
            severity: Severity::Warning,
            message: Cow::Owned(format!(
                "Input '{}' is overridden by CLI argument",
                ctx.input.name
            )),
            help: Some(Cow::Borrowed(
                "CLI inputs take precedence over environment values",
            )),
            example: None,
        })
    } else {
        None
    }
}

fn validate_sensitive_data(ctx: &ValidationContext) -> Option<ValidationIssue> {
    let lower_name = ctx.input.name.to_lowercase();

    if SENSITIVE_PATTERNS
        .iter()
        .any(|pattern| lower_name.contains(pattern))
    {
        Some(ValidationIssue {
            rule: CliRuleId::NoSensitiveData,
            severity: Severity::Warning,
            message: Cow::Owned(format!(
                "Input '{}' may contain sensitive information",
                ctx.input.name
            )),
            help: Some(Cow::Borrowed(
                "Consider using environment variables or a secure secret manager",
            )),
            example: Some(format!(
                "export {}=\"${{VAULT_SECRET}}\"",
                ctx.input.name.to_uppercase()
            )),
        })
    } else {
        None
    }
}

// ============================================================================
// Public API
// ============================================================================

/// Get all default validation rules
pub fn get_default_rules() -> &'static [RuleFn] {
    &[
        validate_input_defined,
        validate_naming_convention,
        validate_cli_override,
        validate_sensitive_data,
    ]
}

/// Get strict validation rules (same as default for now)
pub fn get_strict_rules() -> &'static [RuleFn] {
    get_default_rules()
}

/// Run all rules against a context and collect issues
pub fn validate_all(ctx: &ValidationContext, rules: &[RuleFn]) -> Vec<ValidationIssue> {
    rules.iter().filter_map(|rule| rule(ctx)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_severity_display() {
        assert_eq!(Severity::Error.to_string(), "error");
        assert_eq!(Severity::Warning.to_string(), "warning");
    }

    #[test]
    fn test_severity_from_str() {
        // Test successful parsing
        assert_eq!(Severity::from_str("error").unwrap(), Severity::Error);
        assert_eq!(Severity::from_str("warning").unwrap(), Severity::Warning);

        // Test invalid input
        assert!(Severity::from_str("invalid").is_err());
    }

    #[test]
    fn test_severity_iteration() {
        use strum::IntoEnumIterator;

        let all_severities: Vec<Severity> = Severity::iter().collect();
        assert_eq!(all_severities.len(), 2);
        assert!(all_severities.contains(&Severity::Error));
        assert!(all_severities.contains(&Severity::Warning));
    }

    #[test]
    fn test_severity_as_ref() {
        // Test AsRefStr trait
        assert_eq!(Severity::Error.as_ref(), "error");
        assert_eq!(Severity::Warning.as_ref(), "warning");
    }
}
