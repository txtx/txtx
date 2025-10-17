use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumString};

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
/// // Convert to string
/// assert_eq!(construct.as_ref(), "action");
/// assert_eq!(construct.to_string(), "action");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, AsRefStr, Display, EnumString)]
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
    // Note: These constants duplicate the Strum `serialize_all = "lowercase"` configuration,
    // but serve an important ergonomic purpose: they enable clean pattern matching against
    // string variables without runtime enum conversion.
    //
    // Example usage:
    //   match block_type_str {
    //       ConstructType::ACTION => { ... }   // Clean, no runtime cost
    //       ConstructType::VARIABLE => { ... }
    //   }
    //
    // Without constants, we'd need verbose patterns:
    //   match block_type_str {
    //       s if s == ConstructType::Action.as_ref() => { ... }  // Less readable
    //   }
    //
    // Or runtime enum conversion:
    //   let construct = ConstructType::from_str(block_type_str)?;  // Runtime overhead
    //   match construct { ... }
    //
    // Trade-offs:
    //   Pro: Zero-cost pattern matching, clean syntax throughout codebase
    //   Con: Duplication requires manual sync with Strum serialization
    //
    // Safety: Tests in this module verify constants match Strum serialization at compile time.

    /// String constant for action construct
    pub const ACTION: &'static str = "action";
    /// String constant for variable construct
    pub const VARIABLE: &'static str = "variable";
    /// String constant for output construct
    pub const OUTPUT: &'static str = "output";
    /// String constant for signer construct
    pub const SIGNER: &'static str = "signer";
    /// String constant for addon construct
    pub const ADDON: &'static str = "addon";
    /// String constant for module construct
    pub const MODULE: &'static str = "module";
    /// String constant for flow construct
    pub const FLOW: &'static str = "flow";
    /// String constant for import construct
    pub const IMPORT: &'static str = "import";
    /// String constant for prompt construct
    pub const PROMPT: &'static str = "prompt";
    /// String constant for runbook construct
    pub const RUNBOOK: &'static str = "runbook";

    /// Get the string representation as a static string slice.
    ///
    /// This is equivalent to `as_ref()` but as a const fn.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Action => Self::ACTION,
            Self::Variable => Self::VARIABLE,
            Self::Output => Self::OUTPUT,
            Self::Signer => Self::SIGNER,
            Self::Addon => Self::ADDON,
            Self::Module => Self::MODULE,
            Self::Flow => Self::FLOW,
            Self::Import => Self::IMPORT,
            Self::Prompt => Self::PROMPT,
            Self::Runbook => Self::RUNBOOK,
        }
    }

    /// Returns an array of all valid construct type strings.
    ///
    /// Useful for validation and error messages.
    pub const fn all() -> &'static [&'static str] {
        &[
            Self::ACTION,
            Self::VARIABLE,
            Self::OUTPUT,
            Self::SIGNER,
            Self::ADDON,
            Self::MODULE,
            Self::FLOW,
            Self::IMPORT,
            Self::PROMPT,
            Self::RUNBOOK,
        ]
    }
}

/// Custom serde serializer for ConstructType that maintains string format compatibility.
///
/// This allows ConstructType to be stored internally as an enum while serializing
/// to/from strings for JSON, databases, and APIs.
pub fn serialize_construct_type<S>(ct: &ConstructType, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(ct.as_ref())
}

/// Custom serde deserializer for ConstructType that parses from string format.
///
/// This allows backward compatibility with existing serialized data that stores
/// construct types as strings.
pub fn deserialize_construct_type<'de, D>(deserializer: D) -> Result<ConstructType, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    use std::str::FromStr;

    let s = String::deserialize(deserializer)?;
    ConstructType::from_str(&s).map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

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
    fn test_constants() {
        assert_eq!(ConstructType::ACTION, "action");
        assert_eq!(ConstructType::VARIABLE, "variable");
        assert_eq!(ConstructType::Action.as_str(), ConstructType::ACTION);
    }

    /// Critical test: Ensures string constants stay synchronized with Strum serialization.
    ///
    /// This test exists because we intentionally duplicate serialization information for
    /// ergonomic reasons (see comment above const definitions). The duplication enables
    /// zero-cost pattern matching like `match s { ConstructType::ACTION => ... }` without
    /// requiring runtime enum conversion.
    ///
    /// This test catches any desync between:
    /// 1. The `#[strum(serialize_all = "lowercase")]` attribute
    /// 2. The manually defined `pub const ACTION: &'static str` constants
    /// 3. The `as_str()` const fn implementation
    ///
    /// If this test fails, it means a constant was changed without updating Strum
    /// serialization (or vice versa). Both must be kept in sync.
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

        // Also verify as_str() const fn matches (it uses the constants internally)
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

        // Verify to_string() (from Display trait) also matches
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
        let all = ConstructType::all();
        assert!(all.contains(&"action"));
        assert!(all.contains(&"variable"));
        assert_eq!(all.len(), 10);
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
