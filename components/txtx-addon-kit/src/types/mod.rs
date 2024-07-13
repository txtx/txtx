use diagnostics::{Diagnostic, DiagnosticLevel};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::Iter;
use std::collections::HashMap;
use types::{BufferData, PrimitiveValue, TypeSpecification, Value};
use uuid::Uuid;
use sha2::{Digest, Sha256};

use crate::{helpers::fs::FileLocation, AddonDefaults};

pub mod block_id;
pub mod commands;
pub mod diagnostics;
pub mod frontend;
pub mod functions;
pub mod types;
pub mod wallets;

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Ord, PartialOrd)]
pub struct Did(pub [u8; 32]);

impl Did {

    pub fn zero() -> Self {
        Self([0u8; 32])
    }

    pub fn to_string(&self) -> String {
        hex::encode(self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RunbookUuid(pub Did);

impl RunbookUuid {
    pub fn value(&self) -> Did {
        self.0.clone()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct RunbookId {
    /// Canonical name of the org behind the project
    pub org: Option<String>,
    /// Canonical name of the project supporting the runbook
    pub project: Option<String>,
    /// Canonical name of the runbook
    pub name: String,
}

impl RunbookId {
    pub fn did(&self) -> RunbookUuid {
        let mut hasher = Sha256::new();
        if let Some(ref org) = self.org {
            hasher.update(org.as_bytes());
        }
        if let Some(ref project) = self.project {
            hasher.update(project.as_bytes());
        }
        hasher.update(self.name.as_bytes());
        let hash = hasher.finalize();
        RunbookUuid(Did(hash.into()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PackageUuid(pub Did);

impl PackageUuid {
    pub fn value(&self) -> Did {
        self.0.clone()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct PackageId {
    /// Id of the Runbook
    pub runbook_id: RunbookId,
    /// Location of the package within the project
    pub package_location: FileLocation,
    /// Name of the package
    pub package_name: String,
}

impl PackageId {
    pub fn did(&self) -> PackageUuid {
        let mut hasher = Sha256::new();
        hasher.update(self.runbook_id.did().value().0);
        hasher.update(self.package_location.to_string());
        hasher.update(self.package_name.to_string());
        let hash = hasher.finalize();
        PackageUuid(Did(hash.into()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Ord, PartialOrd)]
pub struct ConstructUuid(pub Did);

impl ConstructUuid {
    pub fn value(&self) -> Did {
        self.0.clone()
    }
}

#[derive(Debug, Clone)]
pub struct ConstructId {
    /// Id of the Package
    pub package_id: PackageId,
    /// Location of the file enclosing the construct
    pub construct_location: FileLocation,
    /// Type of construct (e.g. `input` in `input.value``)
    pub construct_type: String,
    /// Name of construct (e.g. `value` in `input.value``)
    pub construct_name: String,
}

impl ConstructId {
    pub fn did(&self) -> ConstructUuid {
        let mut hasher = Sha256::new();
        hasher.update(self.package_id.did().value().0);
        hasher.update(self.construct_location.to_string());
        hasher.update(self.construct_type.to_string());
        hasher.update(self.construct_name.to_string());
        let hash = hasher.finalize();
        ConstructUuid(Did(hash.into()))
    }
}

#[derive(Debug, Clone)]
pub struct Construct {
    /// Id of the Construct
    pub construct_id: ConstructId,
}

#[derive(Debug, Clone)]
pub struct ValueStore {
    pub uuid: Did,
    name: String,
    storage: HashMap<String, Value>,
}

impl ValueStore {
    pub fn new(name: &str, uuid: &Did) -> ValueStore {
        ValueStore {
            name: name.to_string(),
            uuid: uuid.clone(),
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

    pub fn insert_scoped_value(&mut self, scope: &str, key: &str, value: Value) {
        self.storage.insert(format!("{}:{}", scope, key), value);
    }

    pub fn get_scoped_value(&self, scope: &str, key: &str) -> Option<&Value> {
        self.storage.get(&format!("{}:{}", scope, key))
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
                let bytes = if bytes.starts_with("0x") {
                    crate::hex::decode(&bytes[2..]).unwrap()
                } else {
                    crate::hex::decode(&bytes).unwrap()
                };
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

    pub fn iter(&self) -> Iter<String, Value> {
        self.storage.iter()
    }

    pub fn len(&self) -> usize {
        self.storage.len()
    }
}
