//! Type-safe rule identification system for validation rules
//!
//! This module provides enums and types for identifying validation rules
//! in a type-safe manner, replacing string-based identification.

use std::collections::HashSet;
use std::fmt;
use strum::{AsRefStr, Display, EnumIter, EnumString, IntoStaticStr};

/// Identifies which addons a rule applies to
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddonScope {
    /// Rule applies globally regardless of addon
    Global,
    /// Rule applies to specific addon(s)
    Addons(HashSet<String>),
    /// Rule applies to all addons
    AllAddons,
}

impl AddonScope {
    /// Create a scope for a single addon
    pub fn single(addon: impl Into<String>) -> Self {
        Self::Addons(std::iter::once(addon.into()).collect())
    }

    /// Create a scope for multiple addons
    pub fn multiple<I, S>(addons: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::Addons(addons.into_iter().map(Into::into).collect())
    }

    /// Check if this scope applies given a set of active addons
    pub fn applies_to(&self, active_addons: &HashSet<String>) -> bool {
        match self {
            Self::Global => true,
            Self::AllAddons => !active_addons.is_empty(),
            Self::Addons(required) => !required.is_disjoint(active_addons),
        }
    }
}

/// Internal validation rules built into txtx-core
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
pub enum CoreRuleId {
    // Core validation rules (global)
    UndefinedInput,
    DeprecatedInput,
    RequiredInput,

    // Linter rules (global)
    InputNamingConvention,
    CliInputOverride,
    SensitiveData,
    NoDefaultValues,
    RequiredProductionInputs,

    // Future addon-specific rules can be added here
    // BitcoinAddressFormat,
    // EvmGasLimitRequired,
    // EvmChainIdRequired,
    // SvmProgramIdFormat,
    // StacksContractNameFormat,
    // TelegramBotTokenRequired,
}

impl CoreRuleId {
    /// Returns which addons this rule applies to
    pub fn addon_scope(&self) -> AddonScope {
        use CoreRuleId::*;
        match self {
            // All current rules are global
            UndefinedInput | DeprecatedInput | RequiredInput |
            InputNamingConvention | CliInputOverride |
            SensitiveData | NoDefaultValues | RequiredProductionInputs => AddonScope::Global,

            // Future addon-specific rules would be handled here
            // BitcoinAddressFormat => AddonScope::single("bitcoin"),
            // EvmGasLimitRequired | EvmChainIdRequired => AddonScope::single("evm"),
            // SvmProgramIdFormat => AddonScope::single("svm"),
            // StacksContractNameFormat => AddonScope::single("stacks"),
            // TelegramBotTokenRequired => AddonScope::single("telegram"),
        }
    }

    /// Get a human-readable description of what the rule validates
    pub const fn description(&self) -> &'static str {
        use CoreRuleId::*;
        match self {
            UndefinedInput => "Checks if input references exist in the manifest or CLI inputs",
            DeprecatedInput => "Warns about deprecated input names",
            RequiredInput => "Ensures required inputs are provided in production environments",
            InputNamingConvention => "Validates that inputs follow naming conventions",
            CliInputOverride => "Warns when CLI inputs override environment values",
            SensitiveData => "Detects potential sensitive data in inputs",
            NoDefaultValues => "Ensures production environments don't use default values",
            RequiredProductionInputs => "Ensures required inputs are present in production",
        }
    }
}

/// Identifier for validation rules, supporting both internal and external rules
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RuleIdentifier {
    /// Core rule built into txtx
    Core(CoreRuleId),
    /// External rule defined via configuration (future)
    #[allow(dead_code)] // Reserved for future plugin system
    External(String),
}

impl RuleIdentifier {
    /// Get a string representation of the rule identifier
    pub fn as_str(&self) -> &str {
        match self {
            RuleIdentifier::Core(id) => id.as_ref(),
            RuleIdentifier::External(name) => name.as_str(),
        }
    }

    /// Check if this is a core rule
    pub fn is_core(&self) -> bool {
        matches!(self, RuleIdentifier::Core(_))
    }

    /// Check if this is an external rule
    pub fn is_external(&self) -> bool {
        matches!(self, RuleIdentifier::External(_))
    }
}

impl fmt::Display for RuleIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<CoreRuleId> for RuleIdentifier {
    fn from(id: CoreRuleId) -> Self {
        RuleIdentifier::Core(id)
    }
}

impl AsRef<str> for RuleIdentifier {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_addon_scope_applies_to() {
        let mut active = HashSet::new();
        active.insert("evm".to_string());
        active.insert("bitcoin".to_string());

        // Global scope always applies
        assert!(AddonScope::Global.applies_to(&active));
        assert!(AddonScope::Global.applies_to(&HashSet::new()));

        // AllAddons requires at least one addon
        assert!(AddonScope::AllAddons.applies_to(&active));
        assert!(!AddonScope::AllAddons.applies_to(&HashSet::new()));

        // Specific addon scope
        let evm_scope = AddonScope::single("evm");
        assert!(evm_scope.applies_to(&active));

        let stacks_scope = AddonScope::single("stacks");
        assert!(!stacks_scope.applies_to(&active));

        // Multiple addon scope
        let multi_scope = AddonScope::multiple(["evm", "stacks"]);
        assert!(multi_scope.applies_to(&active)); // Has evm
    }

    #[test]
    fn test_core_rule_id_display() {
        assert_eq!(CoreRuleId::UndefinedInput.to_string(), "undefined_input");
        assert_eq!(CoreRuleId::SensitiveData.to_string(), "sensitive_data");
    }

    #[test]
    fn test_rule_identifier() {
        let core_id = RuleIdentifier::Core(CoreRuleId::UndefinedInput);
        assert!(core_id.is_core());
        assert!(!core_id.is_external());
        assert_eq!(core_id.as_str(), "undefined_input");

        let external_id = RuleIdentifier::External("custom_rule".to_string());
        assert!(!external_id.is_core());
        assert!(external_id.is_external());
        assert_eq!(external_id.as_str(), "custom_rule");
    }
}
