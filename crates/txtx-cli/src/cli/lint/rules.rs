//! Validation rules for txtx runbooks
//!
//! # Adding New Validation Rules
//!
//! To add a new validation rule:
//!
//! 1. Create a validation function with signature:
//!    ```rust
//!    fn validate_my_rule(ctx: &ValidationContext) -> Option<ValidationIssue>
//!    ```
//!
//! 2. Add the rule to `get_default_rules()` function:
//!    ```rust
//!    pub fn get_default_rules() -> Vec<RuleFn> {
//!        vec![
//!            validate_input_defined,
//!            validate_my_rule,  // Add your rule here
//!            // ... other rules
//!        ]
//!    }
//!    ```
//!
//! 3. Add a corresponding CliRuleId variant in `rule_id.rs` if needed
//!
//! # Testing Rules
//!
//! Use the test utilities in the `tests` module to validate rule behavior:
//! ```rust
//! #[test]
//! fn test_my_rule() {
//!     let data = TestContextData::new();
//!     let ctx = data.context("input_name");
//!     let result = validate_my_rule(&ctx);
//!     assert!(result.is_none()); // Expects no issues
//! }
//! ```

use super::rule_id::CliRuleId;
use super::config::LinterConfig;
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
    serde::Deserialize,
    serde::Serialize,
)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    #[serde(alias = "warn")]
    #[strum(to_string = "warning", serialize = "warn")]  // to_string for Display, serialize for parsing
    Warning,
    Info,
    #[serde(alias = "none")]
    #[strum(to_string = "off", serialize = "none")]  // to_string for Display, serialize for parsing
    Off,  // Rule is disabled
}

/// Input-specific context within a validation check
pub struct InputInfo {
    pub name: String,
    pub full_name: String,
}

/// Context passed to validation rules
pub struct ValidationContext<'a> {
    pub manifest: &'a WorkspaceManifest,
    pub environment: Option<&'a str>,
    pub effective_inputs: &'a HashMap<String, String>,
    pub cli_inputs: &'a [(String, String)],
    pub content: &'a str,
    pub file_path: &'a str,
    pub input: InputInfo,
    pub config: Option<&'a LinterConfig>,
}

// ============================================================================
// Data-Driven Rule Configuration
// ============================================================================

const SENSITIVE_PATTERNS: &[&str] = &["password", "secret", "key", "token", "credential"];

// ============================================================================
// Rule Implementations
// ============================================================================

type RuleFn = for<'a> fn(&ValidationContext<'a>) -> Option<ValidationIssue>;

fn validate_input_defined(ctx: &ValidationContext) -> Option<ValidationIssue> {
    if ctx.effective_inputs.contains_key(&ctx.input.name) {
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

pub(crate) fn validate_naming_convention(ctx: &ValidationContext) -> Option<ValidationIssue> {
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
    // Check if this input is provided via CLI
    let is_cli_input = ctx.cli_inputs.iter().any(|(k, _)| k == &ctx.input.name);

    if !is_cli_input {
        return None;
    }

    // Check if the input was also defined in the manifest
    let mut in_manifest = false;

    // Check global environment
    if let Some(global_env) = ctx.manifest.environments.get("global") {
        if global_env.contains_key(&ctx.input.name) {
            in_manifest = true;
        }
    }

    // Check specific environment (if set)
    if let Some(env_name) = ctx.environment {
        if let Some(env) = ctx.manifest.environments.get(env_name) {
            if env.contains_key(&ctx.input.name) {
                in_manifest = true;
            }
        }
    }

    // Only warn if the CLI input overrides a manifest value
    if in_manifest {
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

/// Run all rules against a context and collect issues
pub fn validate_all(ctx: &ValidationContext<'_>, rules: &[RuleFn]) -> Vec<ValidationIssue> {
    rules.iter().filter_map(|rule| {
        let issue = rule(ctx)?;

        // Check if the rule is disabled or severity is overridden in the config
        if let Some(config) = &ctx.config {
            let rule_id = issue.rule.as_ref();

            // If rule is disabled, skip it
            if config.is_rule_disabled(rule_id) {
                return None;
            }

            // Apply configured severity if available
            if let Some(severity) = config.get_rule_severity(rule_id) {
                let mut modified_issue = issue;
                modified_issue.severity = severity;

                // Skip if severity is Off
                if modified_issue.severity == Severity::Off {
                    return None;
                }

                return Some(modified_issue);
            }
        }

        Some(issue)
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_cli_override_rule_logic() {
        use txtx_core::manifest::WorkspaceManifest;

        // Arrange - create context with CLI input that overrides manifest
        let mut manifest = WorkspaceManifest {
            name: "test".to_string(),
            id: "test-id".to_string(),
            runbooks: vec![],
            environments: Default::default(),
            location: None,
        };

        let mut global_env = std::collections::HashMap::new();
        global_env.insert("API_KEY".to_string(), "global-value".to_string());
        manifest.environments.insert("global".to_string(), global_env.into_iter().collect());

        let effective_inputs = HashMap::from([("API_KEY".to_string(), "cli-value".to_string())]);
        let cli_inputs = vec![("API_KEY".to_string(), "cli-value".to_string())];
        let ctx = ValidationContext {
            manifest: &manifest,
            environment: None,
            effective_inputs: &effective_inputs,
            cli_inputs: &cli_inputs,
            content: "",
            file_path: "test.tx",
            input: InputInfo {
                name: "API_KEY".to_string(),
                full_name: "input.API_KEY".to_string(),
            },
            config: None,
        };

        // Act
        let result = validate_cli_override(&ctx);

        // Assert
        assert!(result.is_some(), "Should warn when CLI overrides manifest value");
        let issue = result.unwrap();
        assert_eq!(issue.rule, CliRuleId::CliInputOverride);
        assert!(issue.message.contains("API_KEY"));
    }

    // Property-based tests
    mod proptests {
        use super::*;
        use proptest::prelude::*;
        use std::collections::HashMap;

        // Generate valid snake_case names (no hyphens, no leading underscore)
        prop_compose! {
            fn valid_snake_case_name()(
                first in "[a-z]",
                rest in "[a-z0-9_]{0,30}"
            ) -> String {
                format!("{}{}", first, rest)
            }
        }

        // Generate names with hyphens (invalid)
        prop_compose! {
            fn name_with_hyphens()(
                parts in prop::collection::vec("[a-z][a-z0-9]{0,10}", 2..5)
            ) -> String {
                parts.join("-")
            }
        }

        // Generate names with leading underscore (invalid)
        prop_compose! {
            fn name_with_leading_underscore()(
                underscores in "_+",
                rest in "[a-z][a-z0-9_]{0,30}"
            ) -> String {
                format!("{}{}", underscores, rest)
            }
        }

        // Generate names containing sensitive patterns
        prop_compose! {
            fn name_with_sensitive_pattern()(
                pattern in prop::sample::select(&["password", "secret", "key", "token", "credential"]),
                prefix in "[a-z0-9_]{0,10}",
                suffix in "[a-z0-9_]{0,10}",
                use_upper in prop::bool::ANY,
            ) -> String {
                let pattern_cased = if use_upper {
                    pattern.to_uppercase()
                } else {
                    pattern.to_string()
                };
                format!("{}{}{}", prefix, pattern_cased, suffix)
            }
        }

        // Helper struct that owns test data and can create ValidationContext references
        struct TestContextData {
            manifest: txtx_core::manifest::WorkspaceManifest,
            effective_inputs: HashMap<String, String>,
            cli_inputs: Vec<(String, String)>,
        }

        impl TestContextData {
            fn new() -> Self {
                Self {
                    manifest: txtx_core::manifest::WorkspaceManifest {
                        name: "test".to_string(),
                        id: "test-id".to_string(),
                        runbooks: vec![],
                        environments: Default::default(),
                        location: None,
                    },
                    effective_inputs: HashMap::new(),
                    cli_inputs: vec![],
                }
            }

            fn context<'a>(&'a self, name: &str) -> ValidationContext<'a> {
                ValidationContext {
                    manifest: &self.manifest,
                    environment: None,
                    effective_inputs: &self.effective_inputs,
                    cli_inputs: &self.cli_inputs,
                    content: "",
                    file_path: "test.tx",
                    input: InputInfo {
                        name: name.to_string(),
                        full_name: format!("input.{}", name),
                    },
                    config: None,
                }
            }
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            /// Property: Valid snake_case names should never trigger naming convention warnings
            #[test]
            fn valid_snake_case_always_passes(name in valid_snake_case_name()) {
                let data = TestContextData::new();
                let ctx = data.context(&name);
                let result = validate_naming_convention(&ctx);
                prop_assert!(
                    result.is_none(),
                    "Valid snake_case name '{}' should not trigger warning",
                    name
                );
            }

            /// Property: Names with hyphens should always trigger warnings
            #[test]
            fn names_with_hyphens_always_warn(name in name_with_hyphens()) {
                let data = TestContextData::new();
                let ctx = data.context(&name);
                let result = validate_naming_convention(&ctx);
                prop_assert!(
                    result.is_some(),
                    "Name with hyphens '{}' should trigger warning",
                    name
                );
                if let Some(issue) = result {
                    prop_assert!(issue.message.contains("hyphens"));
                    prop_assert_eq!(issue.severity, Severity::Warning);
                    prop_assert_eq!(issue.rule, CliRuleId::InputNamingConvention);
                }
            }

            /// Property: Names with leading underscores should always trigger warnings
            #[test]
            fn names_with_leading_underscore_always_warn(name in name_with_leading_underscore()) {
                let data = TestContextData::new();
                let ctx = data.context(&name);
                let result = validate_naming_convention(&ctx);
                prop_assert!(
                    result.is_some(),
                    "Name with leading underscore '{}' should trigger warning",
                    name
                );
                if let Some(issue) = result {
                    prop_assert!(issue.message.contains("underscore"));
                    prop_assert_eq!(issue.severity, Severity::Warning);
                }
            }

            /// Property: Names containing sensitive patterns should trigger warnings (case-insensitive)
            #[test]
            fn sensitive_patterns_detected_case_insensitive(name in name_with_sensitive_pattern()) {
                let data = TestContextData::new();
                let ctx = data.context(&name);
                let result = validate_sensitive_data(&ctx);
                prop_assert!(
                    result.is_some(),
                    "Name '{}' should be detected as sensitive",
                    name
                );
                if let Some(issue) = result {
                    prop_assert!(issue.message.contains("sensitive"));
                    prop_assert_eq!(issue.severity, Severity::Warning);
                    prop_assert_eq!(issue.rule, CliRuleId::NoSensitiveData);
                }
            }

            /// Property: Hyphen replacement suggestion should be valid snake_case
            #[test]
            fn hyphen_replacement_produces_valid_name(name in name_with_hyphens()) {
                let data = TestContextData::new();
                let ctx = data.context(&name);
                let result = validate_naming_convention(&ctx);

                if let Some(issue) = result {
                    if let Some(example) = issue.example {
                        // The example should not contain hyphens
                        prop_assert!(
                            !example.contains('-'),
                            "Example '{}' should not contain hyphens",
                            example
                        );

                        // The example should be the original with underscores
                        // Verify it doesn't trigger the same warning
                        let fixed_data = TestContextData::new();
                        let fixed_ctx = fixed_data.context(&example);
                        let fixed_result = validate_naming_convention(&fixed_ctx);

                        // Should either pass or only warn about other things (not hyphens)
                        if let Some(fixed_issue) = fixed_result {
                            prop_assert!(
                                !fixed_issue.message.contains("hyphens"),
                                "Fixed name should not trigger hyphen warning"
                            );
                        }
                    }
                }
            }

            /// Property: Severity enum should roundtrip through string conversion
            #[test]
            fn severity_enum_roundtrip(
                severity in prop::sample::select(vec![Severity::Error, Severity::Warning, Severity::Info, Severity::Off])
            ) {
                let string = severity.to_string();
                let parsed = Severity::from_str(&string).unwrap();
                prop_assert_eq!(severity, parsed);
            }

            /// Property: Valid names pass all naming and sensitive data rules
            #[test]
            fn valid_names_pass_all_rules(name in valid_snake_case_name()) {
                // Filter out names that happen to contain sensitive patterns
                prop_assume!(!SENSITIVE_PATTERNS.iter().any(|p| name.to_lowercase().contains(p)));

                let data = TestContextData::new();
                let ctx = data.context(&name);

                // Should pass naming convention
                let naming_result = validate_naming_convention(&ctx);
                prop_assert!(naming_result.is_none(),
                    "Valid name '{}' should pass naming convention check, got: {:?}",
                    name, naming_result);

                // Should pass sensitive data check
                let sensitive_result = validate_sensitive_data(&ctx);
                prop_assert!(sensitive_result.is_none(),
                    "Valid name '{}' should pass sensitive data check, got: {:?}",
                    name, sensitive_result);

                // Should pass all rules together
                let rules = get_default_rules();
                let issues = validate_all(&ctx, rules);

                // Filter out input_defined issues (we're only testing naming/sensitive rules)
                let naming_issues: Vec<_> = issues.iter()
                    .filter(|i| i.rule != CliRuleId::InputDefined && i.rule != CliRuleId::CliInputOverride)
                    .collect();

                prop_assert!(naming_issues.is_empty(),
                    "Valid name '{}' should pass all naming/sensitive rules, got issues: {:?}",
                    name, naming_issues);
            }

            /// Property: Suggested fixes for naming issues actually fix the problem
            #[test]
            fn suggested_fixes_resolve_issues(name in name_with_hyphens()) {
                let data = TestContextData::new();
                let ctx = data.context(&name);
                let result = validate_naming_convention(&ctx);

                // Should have an issue with a suggested fix
                prop_assert!(result.is_some(), "Name '{}' should have naming issue", name);

                if let Some(issue) = result {
                    if let Some(suggested_name) = issue.example {
                        // Apply the fix and verify it resolves the issue
                        let fixed_data = TestContextData::new();
                        let fixed_ctx = fixed_data.context(&suggested_name);
                        let fixed_result = validate_naming_convention(&fixed_ctx);

                        // The fix should either:
                        // 1. Completely resolve the issue (no warning), OR
                        // 2. Only have different warnings (not the same hyphen warning)
                        if let Some(fixed_issue) = fixed_result {
                            prop_assert!(
                                !fixed_issue.message.contains("hyphens"),
                                "Suggested fix '{}' for '{}' should resolve hyphen warning, but got: {}",
                                suggested_name, name, fixed_issue.message
                            );
                        }
                    }
                }
            }

            /// Property: Leading underscore removal suggestions produce valid names
            #[test]
            fn underscore_fix_produces_valid_name(name in name_with_leading_underscore()) {
                let data = TestContextData::new();
                let ctx = data.context(&name);
                let result = validate_naming_convention(&ctx);

                prop_assert!(result.is_some(), "Name '{}' should have underscore warning", name);

                if let Some(issue) = result {
                    if let Some(suggested_name) = issue.example {
                        // Suggested name should not start with underscore
                        prop_assert!(
                            !suggested_name.starts_with('_'),
                            "Suggested fix '{}' should not start with underscore",
                            suggested_name
                        );

                        // Suggested name should be the original without leading underscores
                        let trimmed = name.trim_start_matches('_');
                        prop_assert_eq!(
                            &suggested_name, trimmed,
                            "Suggested fix should be the name with underscores removed"
                        );
                    }
                }
            }
        }
    }
}
