//! Type-safe rule identification for CLI-specific linting rules

use std::fmt;
use strum::{AsRefStr, Display, EnumIter, EnumString, IntoStaticStr};
use txtx_core::validation::{AddonScope, CoreRuleId};

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

/// Identifier for CLI validation rules
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CliRuleIdentifier {
    /// CLI-specific rule
    Cli(CliRuleId),
    /// Core rule reused in CLI
    Core(CoreRuleId),
    /// External rule defined via configuration (future)
    #[allow(dead_code)] // Reserved for future plugin system
    External(String),
}

impl CliRuleIdentifier {
    /// Get a string representation of the rule identifier
    pub fn as_str(&self) -> &str {
        match self {
            Self::Cli(id) => id.as_ref(),
            Self::Core(id) => id.as_ref(),
            Self::External(name) => name.as_str(),
        }
    }
}

impl fmt::Display for CliRuleIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<CliRuleId> for CliRuleIdentifier {
    fn from(id: CliRuleId) -> Self {
        CliRuleIdentifier::Cli(id)
    }
}

impl From<CoreRuleId> for CliRuleIdentifier {
    fn from(id: CoreRuleId) -> Self {
        CliRuleIdentifier::Core(id)
    }
}