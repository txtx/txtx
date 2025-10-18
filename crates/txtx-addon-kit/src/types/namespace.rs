use serde::{Deserialize, Serialize};
use std::fmt;
use strum_macros::{AsRefStr, Display as StrumDisplay, EnumString, IntoStaticStr};

/// Namespace types for txtx addons and functions
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Namespace {
    /// Well-known standard namespace
    WellKnown(WellKnownNamespace),
    /// Custom addon namespace
    Custom(String),
}

/// Well-known namespaces that have special meaning in txtx
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    AsRefStr,
    StrumDisplay,
    EnumString,
    IntoStaticStr,
)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum WellKnownNamespace {
    /// Standard library namespace
    Std,
}

impl Namespace {
    /// Create a standard namespace
    pub fn std() -> Self {
        Namespace::WellKnown(WellKnownNamespace::Std)
    }

    /// Create a custom namespace
    pub fn custom(name: impl Into<String>) -> Self {
        Namespace::Custom(name.into())
    }

    /// Get the string representation of the namespace
    pub fn as_str(&self) -> &str {
        match self {
            Namespace::WellKnown(wk) => wk.as_ref(),
            Namespace::Custom(s) => s.as_str(),
        }
    }

    /// Check if this is the standard namespace
    pub fn is_std(&self) -> bool {
        matches!(self, Namespace::WellKnown(WellKnownNamespace::Std))
    }
}

impl fmt::Display for Namespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<String> for Namespace {
    fn from(s: String) -> Self {
        match s.as_str() {
            "std" => Namespace::std(),
            _ => Namespace::Custom(s),
        }
    }
}

impl From<&str> for Namespace {
    fn from(s: &str) -> Self {
        match s {
            "std" => Namespace::std(),
            _ => Namespace::Custom(s.to_string()),
        }
    }
}

impl From<&String> for Namespace {
    fn from(s: &String) -> Self {
        Namespace::from(s.as_str())
    }
}

impl From<&Namespace> for Namespace {
    fn from(ns: &Namespace) -> Self {
        ns.clone()
    }
}

impl AsRef<str> for Namespace {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

// Allow comparison with &str
impl PartialEq<&str> for Namespace {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<Namespace> for &str {
    fn eq(&self, other: &Namespace) -> bool {
        *self == other.as_str()
    }
}

// Allow comparison with String
impl PartialEq<String> for Namespace {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<Namespace> for String {
    fn eq(&self, other: &Namespace) -> bool {
        self.as_str() == other.as_str()
    }
}