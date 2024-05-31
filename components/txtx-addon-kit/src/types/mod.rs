use std::collections::HashMap;
use std::str::FromStr;

use diagnostics::{Diagnostic, DiagnosticLevel};
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use types::{BufferData, PrimitiveValue, TypeSpecification, Value};
use uuid::Uuid;

use crate::AddonDefaults;

pub mod commands;
pub mod diagnostics;
pub mod frontend;
pub mod functions;
pub mod types;
pub mod wallets;

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ConstructUuid {
    Local(Uuid),
}

impl ConstructUuid {
    pub fn new() -> Self {
        Self::Local(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: &Uuid) -> Self {
        Self::Local(uuid.clone())
    }

    pub fn value(&self) -> Uuid {
        match &self {
            Self::Local(v) => v.clone(),
        }
    }
}

impl Serialize for ConstructUuid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Local(uuid) => serializer.serialize_str(&format!("local:{}", uuid.to_string())),
        }
    }
}

impl<'de> Deserialize<'de> for ConstructUuid {
    fn deserialize<D>(deserializer: D) -> Result<ConstructUuid, D::Error>
    where
        D: Deserializer<'de>,
    {
        let uuid: String = serde::Deserialize::deserialize(deserializer)?;
        match uuid.strip_prefix("local:") {
            Some(result) => {
                let uuid = Uuid::from_str(&result).map_err(D::Error::custom)?;
                Ok(ConstructUuid::from_uuid(&uuid))
            }
            None => Err(D::Error::custom(
                "UUID string must be prefixed with 'local:'",
            )),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PackageUuid {
    Local(Uuid),
}

impl Serialize for PackageUuid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Local(uuid) => serializer.serialize_str(&format!("local:{}", uuid.to_string())),
        }
    }
}

impl PackageUuid {
    pub fn new() -> Self {
        Self::Local(Uuid::new_v4())
    }

    pub fn value(&self) -> Uuid {
        match &self {
            Self::Local(v) => v.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ValueStore {
    name: String,
    storage: HashMap<String, Value>,
}

impl ValueStore {
    pub fn new(name: &str) -> ValueStore {
        ValueStore {
            name: name.to_string(),
            storage: HashMap::new(),
        }
    }

    pub fn get_defaulting_string(
        &self,
        key: &str,
        defaults: &AddonDefaults,
    ) -> Result<String, Diagnostic> {
        let Some(value) = self.storage.get(key) else {
            let res =
                defaults
                    .keys
                    .get(key)
                    .map(|x| x.as_str())
                    .ok_or(Diagnostic::error_from_string(format!(
                        "store '{}': unable to retrieve key '{}'",
                        self.name, key
                    )))?;
            return Ok(res.to_string());
        };
        let Some(value) = value.as_string() else {
            return Err(Diagnostic {
                span: None,
                location: None,
                message: format!(
                    "store '{}': value associated to '{}' mismatch (expected string)",
                    self.name, key
                ),
                level: DiagnosticLevel::Error,
                documentation: None,
                example: None,
                parent_diagnostic: None,
            });
        };
        Ok(value.to_string())
    }

    pub fn get_expected_value(&self, key: &str) -> Result<&Value, Diagnostic> {
        let Some(value) = self.storage.get(key) else {
            return Err(Diagnostic::error_from_string(format!(
                "store '{}': unable to retrieve key '{}'",
                self.name, key
            )));
        };
        Ok(value)
    }

    pub fn get_value(&self, key: &str) -> Option<&Value> {
        self.storage.get(key)
    }

    pub fn get_string(&self, key: &str) -> Option<&str> {
        self.storage.get(key).and_then(|v| v.as_string())
    }

    pub fn get_expected_bool(&self, key: &str) -> Result<bool, Diagnostic> {
        let Some(value) = self.storage.get(key) else {
            return Err(Diagnostic::error_from_string(format!(
                "store '{}': unable to retrieve key '{}'",
                self.name, key
            )));
        };
        let Some(value) = value.as_bool() else {
            return Err(Diagnostic::error_from_string(format!(
                "store '{}': value associated to '{}' mismatch (expected bool)",
                self.name, key
            )));
        };
        Ok(value)
    }

    pub fn get_expected_string(&self, key: &str) -> Result<&str, Diagnostic> {
        let Some(value) = self.storage.get(key) else {
            return Err(Diagnostic::error_from_string(format!(
                "store '{}': unable to retrieve key '{}'",
                self.name, key
            )));
        };
        let Some(value) = value.as_string() else {
            return Err(Diagnostic::error_from_string(format!(
                "store '{}': value associated to '{}' mismatch (expected string)",
                self.name, key
            )));
        };
        Ok(value)
    }

    pub fn get_expected_array(&self, key: &str) -> Result<&Vec<Value>, Diagnostic> {
        let Some(value) = self.storage.get(key) else {
            return Err(Diagnostic::error_from_string(format!(
                "store '{}': unable to retrieve key '{}'",
                self.name, key
            )));
        };
        let Some(value) = value.as_array() else {
            return Err(Diagnostic::error_from_string(format!(
                "store '{}': value associated to '{}' mismatch (expected array)",
                self.name, key
            )));
        };
        Ok(value)
    }

    pub fn get_expected_uint(&self, key: &str) -> Result<u64, Diagnostic> {
        let Some(value) = self.storage.get(key) else {
            return Err(Diagnostic::error_from_string(format!(
                "store '{}': unable to retrieve key '{}'",
                self.name, key
            )));
        };
        let Some(value) = value.as_uint() else {
            return Err(Diagnostic::error_from_string(format!(
                "store '{}': value associated to '{}' mismatch (expected uint)",
                self.name, key
            )));
        };
        Ok(value)
    }

    pub fn get_expected_buffer(
        &self,
        key: &str,
        typing: &TypeSpecification,
    ) -> Result<BufferData, Diagnostic> {
        let Some(value) = self.storage.get(key) else {
            return Err(Diagnostic::error_from_string(format!(
                "store '{}': unable to retrieve key '{}'",
                self.name, key
            )));
        };

        let bytes = match value {
            Value::Primitive(PrimitiveValue::Buffer(bytes)) => bytes.clone(),
            Value::Primitive(PrimitiveValue::String(bytes)) => {
                let bytes = crate::hex::decode(&bytes[2..]).unwrap();
                BufferData {
                    bytes,
                    typing: typing.clone(),
                }
            }
            _ => {
                return Err(Diagnostic::error_from_string(format!(
                    "store '{}': value associated to '{}' mismatch (expected buffer)",
                    self.name, key
                )))
            }
        };
        Ok(bytes)
    }

    pub fn insert(&mut self, key: &str, value: Value) {
        self.storage.insert(key.to_string(), value);
    }
}
