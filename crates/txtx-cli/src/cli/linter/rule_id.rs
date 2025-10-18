//! Type-safe rule identification for CLI-specific linting rules

use std::fmt;
use txtx_core::validation::{AddonScope, CoreRuleId};

/// CLI-specific linting rules
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    /// Get a string representation suitable for display and configuration
    pub const fn as_str(&self) -> &'static str {
        use CliRuleId::*;
        match self {
            InputDefined => "input_defined",
            InputNamingConvention => "input_naming_convention",
            CliInputOverride => "cli_input_override",
            NoSensitiveData => "no_sensitive_data",
        }
    }

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

impl fmt::Display for CliRuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
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
            Self::Cli(id) => id.as_str(),
            Self::Core(id) => id.as_str(),
            Self::External(name) => name.as_str(),
        }
    }

    /// Get the addon scope for this rule
    pub fn addon_scope(&self) -> AddonScope {
        match self {
            Self::Cli(id) => id.addon_scope(),
            Self::Core(id) => id.addon_scope(),
            Self::External(_) => AddonScope::Global, // Default for now
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_rule_id_display() {
        assert_eq!(CliRuleId::InputDefined.to_string(), "input_defined");
        assert_eq!(CliRuleId::NoSensitiveData.to_string(), "no_sensitive_data");
    }

    #[test]
    fn test_cli_rule_identifier() {
        let cli_id = CliRuleIdentifier::Cli(CliRuleId::InputDefined);
        assert_eq!(cli_id.as_str(), "input_defined");

        let core_id = CliRuleIdentifier::Core(CoreRuleId::UndefinedInput);
        assert_eq!(core_id.as_str(), "undefined_input");

        let external_id = CliRuleIdentifier::External("custom".to_string());
        assert_eq!(external_id.as_str(), "custom");
    }
}