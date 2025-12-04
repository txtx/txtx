//! Type-safe rule identification for CLI-specific linting rules

use strum::{AsRefStr, Display, EnumIter, EnumString, IntoStaticStr};
use txtx_core::validation::AddonScope;

/// CLI-specific linting rules
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    AsRefStr,      // Provides as_ref() -> &str
    Display,       // Provides to_string()
    EnumString,    // Provides from_str()
    IntoStaticStr, // Provides into() -> &'static str
    EnumIter,      // Provides iter() over all variants
)]
#[strum(serialize_all = "snake_case")]
pub enum CliRuleId {
    /// Check if input is defined
    InputDefined,
    /// Check input naming conventions
    InputNamingConvention,
    /// Warn about CLI input overrides
    CliInputOverride,
    /// Detect sensitive data in inputs
    NoSensitiveData,
}

impl CliRuleId {
    /// Get a human-readable description of what the rule validates
    pub const fn description(&self) -> &'static str {
        use CliRuleId::*;
        match self {
            InputDefined => "Validates that inputs are defined in the environment",
            InputNamingConvention => "Checks that inputs follow naming conventions",
            CliInputOverride => "Warns when CLI arguments override environment values",
            NoSensitiveData => "Detects potential sensitive information in inputs",
        }
    }

    /// Returns the scope of addons this rule applies to
    ///
    /// Currently all CLI rules are global in scope.
    pub const fn addon_scope(&self) -> AddonScope {
        AddonScope::Global
    }
}