//! Validation rules for txtx runbooks

use super::rule_id::CliRuleId;
use std::borrow::Cow;
use std::collections::HashMap;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// Input-specific context within a validation check
pub struct InputInfo<'a> {
    pub name: &'a str,
    pub full_name: &'a str,
}

/// Context passed to validation rules
pub struct ValidationContext<'env, 'content> {
    pub manifest: &'env WorkspaceManifest,
    pub environment: Option<&'env str>,
    pub effective_inputs: &'env HashMap<String, String>,
    pub cli_inputs: &'env [(String, String)],
    pub content: &'content str,
    pub file_path: &'content str,
    pub input: InputInfo<'content>,
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
    if ctx.effective_inputs.contains_key(ctx.input.name) {
        return None;
    }

    let env_name = ctx.environment.unwrap_or("global");
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
    if !ctx.effective_inputs.contains_key(ctx.input.name) {
        return None;
    }

    let is_overridden = ctx.cli_inputs.iter().any(|(k, _)| k == ctx.input.name);
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
