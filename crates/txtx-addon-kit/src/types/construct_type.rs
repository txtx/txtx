use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumIter, EnumString, IntoStaticStr};

/// Central definition of all txtx language construct types.
///
/// This enum provides a type-safe way to work with construct type identifiers,
/// eliminating magic strings throughout the codebase.
///
/// # Examples
///
/// ```
/// use txtx_addon_kit::types::construct_type::ConstructType;
/// use std::str::FromStr;
///
/// // Parse from string
/// let construct = ConstructType::from_str("action").unwrap();
/// assert_eq!(construct, ConstructType::Action);
///
/// // Convert to string (multiple ways provided by Strum)
/// assert_eq!(construct.as_ref(), "action");           // AsRefStr
/// assert_eq!(construct.to_string(), "action");        // Display
/// let s: &'static str = construct.into();             // IntoStaticStr
/// assert_eq!(s, "action");
/// ```
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    AsRefStr,      // Provides as_ref() -> &str
    Display,       // Provides to_string()
    EnumString,    // Provides from_str()
    IntoStaticStr, // Provides into() -> &'static str
    EnumIter,      // Provides iter() over all variants
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ConstructType {
    Action,
    Variable,
    Output,
    Signer,
    Addon,
    Module,
    Flow,
    Import,
    Prompt,
    Runbook,
}

impl ConstructType {
    /// Returns an iterator over all construct type variants.
    ///
    /// Useful for validation and error messages.
    ///
    /// # Example
    /// ```
    /// use txtx_addon_kit::types::construct_type::ConstructType;
    /// let count = ConstructType::all().count();
    /// assert_eq!(count, 10);
    /// ```
    pub fn all() -> impl Iterator<Item = Self> {
        use strum::IntoEnumIterator;
        Self::iter()
    }

    /// Returns a Vec of all valid construct type strings.
    ///
    /// # Example
    /// ```
    /// use txtx_addon_kit::types::construct_type::ConstructType;
    /// let strings = ConstructType::all_str();
    /// assert!(strings.contains(&"action"));
    /// ```
    pub fn all_str() -> Vec<&'static str> {
        Self::all().map(|ct| ct.into()).collect()
    }
}

// Implement PartialEq with &str for ergonomic comparisons
impl PartialEq<&str> for ConstructType {
    fn eq(&self, other: &&str) -> bool {
        self.as_ref() == *other
    }
}

impl PartialEq<ConstructType> for &str {
    fn eq(&self, other: &ConstructType) -> bool {
        *self == other.as_ref()
    }
}

impl PartialEq<String> for ConstructType {
    fn eq(&self, other: &String) -> bool {
        self.as_ref() == other.as_str()
    }
}

impl PartialEq<ConstructType> for String {
    fn eq(&self, other: &ConstructType) -> bool {
        self.as_str() == other.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partialeq_with_str() {
        // Test ConstructType == &str
        assert_eq!(ConstructType::Action, "action");
        assert_eq!(ConstructType::Variable, "variable");
        assert_ne!(ConstructType::Action, "variable");

        // Test &str == ConstructType
        assert_eq!("action", ConstructType::Action);
        assert_eq!("variable", ConstructType::Variable);
        assert_ne!("action", ConstructType::Variable);
    }

    #[test]
    fn test_partialeq_with_string() {
        let action_str = "action".to_string();
        let variable_str = "variable".to_string();

        // Test ConstructType == String
        assert_eq!(ConstructType::Action, action_str);
        assert_eq!(ConstructType::Variable, variable_str);
        assert_ne!(ConstructType::Action, variable_str);

        // Test String == ConstructType
        assert_eq!(action_str, ConstructType::Action);
        assert_eq!(variable_str, ConstructType::Variable);
        assert_ne!(action_str, ConstructType::Variable);
    }

    #[test]
    fn test_block_type_string_comparison() {
        // Simulates the pattern used in test utils
        let block_type = "action".to_string();
        assert!(block_type == ConstructType::Action);
        assert!(ConstructType::Action == block_type);
    }
}
