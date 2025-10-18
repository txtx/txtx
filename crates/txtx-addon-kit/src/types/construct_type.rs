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

// Macro to generate string constants with reduced duplication.
// These constants enable clean pattern matching against string variables without
// runtime enum conversion: `match s { ConstructType::ACTION => ... }`
//
// Note: An alternative idiomatic approach is to parse strings at boundaries:
//   let construct = ConstructType::from_str(s)?;
//   match construct { ConstructType::Action => ... }
// This provides stronger type safety but requires error handling.
macro_rules! construct_constants {
    ($($name:ident => $value:expr),* $(,)?) => {
        impl ConstructType {
            $(
                #[doc = concat!("String constant for ", $value, " construct")]
                pub const $name: &'static str = $value;
            )*
        }
    };
}

construct_constants! {
    ACTION => "action",
    VARIABLE => "variable",
    OUTPUT => "output",
    SIGNER => "signer",
    ADDON => "addon",
    MODULE => "module",
    FLOW => "flow",
    IMPORT => "import",
    PROMPT => "prompt",
    RUNBOOK => "runbook",
}

impl ConstructType {
    /// Get the string representation as a static string slice.
    ///
    /// This is a const fn version of `as_ref()` for use in const contexts.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Action => "action",
            Self::Variable => "variable",
            Self::Output => "output",
            Self::Signer => "signer",
            Self::Addon => "addon",
            Self::Module => "module",
            Self::Flow => "flow",
            Self::Import => "import",
            Self::Prompt => "prompt",
            Self::Runbook => "runbook",
        }
    }

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


#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use strum::IntoEnumIterator;

    #[test]
    fn test_from_str() {
        assert_eq!(ConstructType::from_str("action").unwrap(), ConstructType::Action);
        assert_eq!(ConstructType::from_str("variable").unwrap(), ConstructType::Variable);
        assert_eq!(ConstructType::from_str("output").unwrap(), ConstructType::Output);
        assert_eq!(ConstructType::from_str("signer").unwrap(), ConstructType::Signer);
        assert!(ConstructType::from_str("invalid").is_err());
    }

    #[test]
    fn test_as_ref() {
        assert_eq!(ConstructType::Action.as_ref(), "action");
        assert_eq!(ConstructType::Variable.as_ref(), "variable");
        assert_eq!(ConstructType::Output.as_ref(), "output");
    }

    #[test]
    fn test_display() {
        assert_eq!(ConstructType::Action.to_string(), "action");
        assert_eq!(ConstructType::Variable.to_string(), "variable");
    }

    #[test]
    fn test_into_static_str() {
        let s: &'static str = ConstructType::Action.into();
        assert_eq!(s, "action");
        let s: &'static str = ConstructType::Variable.into();
        assert_eq!(s, "variable");
    }

    #[test]
    fn test_constants() {
        assert_eq!(ConstructType::ACTION, "action");
        assert_eq!(ConstructType::VARIABLE, "variable");
        assert_eq!(ConstructType::Action.as_str(), ConstructType::ACTION);
    }

    /// Critical test: Ensures string constants stay synchronized with Strum serialization.
    ///
    /// This verifies that the macro-generated constants, the as_str() const fn,
    /// and Strum's derived traits all produce consistent string representations.
    #[test]
    fn test_constants_match_strum_serialization() {
        // Each constant must exactly match what Strum serializes the enum to
        assert_eq!(ConstructType::ACTION, ConstructType::Action.as_ref());
        assert_eq!(ConstructType::VARIABLE, ConstructType::Variable.as_ref());
        assert_eq!(ConstructType::OUTPUT, ConstructType::Output.as_ref());
        assert_eq!(ConstructType::SIGNER, ConstructType::Signer.as_ref());
        assert_eq!(ConstructType::ADDON, ConstructType::Addon.as_ref());
        assert_eq!(ConstructType::MODULE, ConstructType::Module.as_ref());
        assert_eq!(ConstructType::FLOW, ConstructType::Flow.as_ref());
        assert_eq!(ConstructType::IMPORT, ConstructType::Import.as_ref());
        assert_eq!(ConstructType::PROMPT, ConstructType::Prompt.as_ref());
        assert_eq!(ConstructType::RUNBOOK, ConstructType::Runbook.as_ref());

        // Verify as_str() const fn matches
        assert_eq!(ConstructType::Action.as_str(), ConstructType::ACTION);
        assert_eq!(ConstructType::Variable.as_str(), ConstructType::VARIABLE);
        assert_eq!(ConstructType::Output.as_str(), ConstructType::OUTPUT);
        assert_eq!(ConstructType::Signer.as_str(), ConstructType::SIGNER);
        assert_eq!(ConstructType::Addon.as_str(), ConstructType::ADDON);
        assert_eq!(ConstructType::Module.as_str(), ConstructType::MODULE);
        assert_eq!(ConstructType::Flow.as_str(), ConstructType::FLOW);
        assert_eq!(ConstructType::Import.as_str(), ConstructType::IMPORT);
        assert_eq!(ConstructType::Prompt.as_str(), ConstructType::PROMPT);
        assert_eq!(ConstructType::Runbook.as_str(), ConstructType::RUNBOOK);

        // Verify to_string() (from Display trait) matches
        assert_eq!(ConstructType::Action.to_string(), ConstructType::ACTION);
        assert_eq!(ConstructType::Variable.to_string(), ConstructType::VARIABLE);
        assert_eq!(ConstructType::Output.to_string(), ConstructType::OUTPUT);
        assert_eq!(ConstructType::Signer.to_string(), ConstructType::SIGNER);
        assert_eq!(ConstructType::Addon.to_string(), ConstructType::ADDON);
        assert_eq!(ConstructType::Module.to_string(), ConstructType::MODULE);
        assert_eq!(ConstructType::Flow.to_string(), ConstructType::FLOW);
        assert_eq!(ConstructType::Import.to_string(), ConstructType::IMPORT);
        assert_eq!(ConstructType::Prompt.to_string(), ConstructType::PROMPT);
        assert_eq!(ConstructType::Runbook.to_string(), ConstructType::RUNBOOK);
    }

    #[test]
    fn test_all() {
        let all: Vec<_> = ConstructType::all().collect();
        assert_eq!(all.len(), 10);
        assert!(all.contains(&ConstructType::Action));
        assert!(all.contains(&ConstructType::Variable));
    }

    #[test]
    fn test_all_str() {
        let all = ConstructType::all_str();
        assert_eq!(all.len(), 10);
        assert!(all.contains(&"action"));
        assert!(all.contains(&"variable"));
    }

    #[test]
    fn test_enum_iter() {
        let all_types: Vec<_> = ConstructType::iter().collect();
        assert_eq!(all_types.len(), 10);
        assert_eq!(all_types[0], ConstructType::Action);
        assert_eq!(all_types[9], ConstructType::Runbook);
    }

    #[test]
    fn test_serde_serialization() {
        use serde_json;

        let ct = ConstructType::Action;
        let json = serde_json::to_string(&ct).unwrap();
        assert_eq!(json, "\"action\"");

        let ct = ConstructType::Variable;
        let json = serde_json::to_string(&ct).unwrap();
        assert_eq!(json, "\"variable\"");
    }

    #[test]
    fn test_serde_deserialization() {
        use serde_json;

        let ct: ConstructType = serde_json::from_str("\"action\"").unwrap();
        assert_eq!(ct, ConstructType::Action);

        let ct: ConstructType = serde_json::from_str("\"variable\"").unwrap();
        assert_eq!(ct, ConstructType::Variable);

        // Invalid type should fail
        let result: Result<ConstructType, _> = serde_json::from_str("\"invalid\"");
        assert!(result.is_err());
    }
}
