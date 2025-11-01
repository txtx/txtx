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
//!     let ctx = create_test_context("input_name".to_string());
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
pub struct ValidationContext {
    pub manifest: WorkspaceManifest,
    pub environment: Option<String>,
    pub effective_inputs: HashMap<String, String>,
    pub cli_inputs: Vec<(String, String)>,
    pub content: String,
    pub file_path: String,
    pub input: InputInfo,
    pub config: Option<LinterConfig>,
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

/// Run all rules against a context and collect issues
pub fn validate_all(ctx: &ValidationContext, rules: &[RuleFn]) -> Vec<ValidationIssue> {
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
    fn test_severity_display() {
        assert_eq!(Severity::Error.to_string(), "error");
        assert_eq!(Severity::Warning.to_string(), "warning"); // primary serialization
        assert_eq!(Severity::Info.to_string(), "info");
        assert_eq!(Severity::Off.to_string(), "off"); // primary serialization
    }

    #[test]
    fn test_severity_from_str() {
        // Test successful parsing
        assert_eq!(Severity::from_str("error").unwrap(), Severity::Error);
        assert_eq!(Severity::from_str("warning").unwrap(), Severity::Warning);
        assert_eq!(Severity::from_str("info").unwrap(), Severity::Info);
        assert_eq!(Severity::from_str("off").unwrap(), Severity::Off);

        // Test aliases
        assert_eq!(Severity::from_str("warn").unwrap(), Severity::Warning);
        assert_eq!(Severity::from_str("none").unwrap(), Severity::Off);

        // Test invalid input
        assert!(Severity::from_str("invalid").is_err());
    }

    #[test]
    fn test_severity_iteration() {
        use strum::IntoEnumIterator;

        let all_severities: Vec<Severity> = Severity::iter().collect();
        assert_eq!(all_severities.len(), 4);
        assert!(all_severities.contains(&Severity::Error));
        assert!(all_severities.contains(&Severity::Warning));
        assert!(all_severities.contains(&Severity::Info));
        assert!(all_severities.contains(&Severity::Off));
    }

    #[test]
    fn test_severity_as_ref() {
        // Test AsRefStr trait
        assert_eq!(Severity::Error.as_ref(), "error");
        assert_eq!(Severity::Warning.as_ref(), "warning");
        assert_eq!(Severity::Info.as_ref(), "info");
        assert_eq!(Severity::Off.as_ref(), "off");
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

        // Helper to create a minimal ValidationContext
        fn create_test_context(name: String) -> ValidationContext {
            ValidationContext {
                manifest: txtx_core::manifest::WorkspaceManifest {
                    name: "test".to_string(),
                    id: "test-id".to_string(),
                    runbooks: vec![],
                    environments: Default::default(),
                    location: None,
                },
                environment: None,
                effective_inputs: HashMap::new(),
                cli_inputs: vec![],
                content: String::new(),
                file_path: String::new(),
                input: InputInfo {
                    name: name.clone(),
                    full_name: format!("input.{}", name),
                },
                config: None,
            }
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            /// Property: Valid snake_case names should never trigger naming convention warnings
            #[test]
            fn valid_snake_case_always_passes(name in valid_snake_case_name()) {
                let ctx = create_test_context(name.clone());
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
                let ctx = create_test_context(name.clone());
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
                let ctx = create_test_context(name.clone());
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
                let ctx = create_test_context(name.clone());
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
                let ctx = create_test_context(name);
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
                        let fixed_ctx = create_test_context(example);
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
        }
    }
}
