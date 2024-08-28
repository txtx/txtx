use std::collections::HashMap;

use diagnostics::Diagnostic;
use indexmap::IndexMap;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use types::Value;

use crate::{helpers::fs::FileLocation, AddonDefaults};

pub mod block_id;
pub mod commands;
pub mod diagnostics;
pub mod frontend;
pub mod functions;
pub mod signers;
pub mod types;

pub const CACHED_NONCE: &str = "cached_nonce";

#[cfg(test)]
mod tests;

#[derive(Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Did(pub [u8; 32]);

impl Did {
    pub fn from_components(comps: Vec<impl AsRef<[u8]>>) -> Self {
        let mut hasher = Sha256::new();
        for comp in comps {
            hasher.update(comp);
        }
        let hash = hasher.finalize();
        Did(hash.into())
    }

    pub fn from_hex_string(source_bytes_str: &str) -> Self {
        let bytes = hex::decode(source_bytes_str).expect("invalid hex_string");
        Self::from_bytes(&bytes)
    }

    pub fn from_bytes(source_bytes: &Vec<u8>) -> Self {
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&source_bytes);
        Did(bytes)
    }

    pub fn zero() -> Self {
        Self([0u8; 32])
    }

    pub fn to_string(&self) -> String {
        hex::encode(self.0)
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl Serialize for Did {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("0x{}", self))
    }
}

impl<'de> Deserialize<'de> for Did {
    fn deserialize<D>(deserializer: D) -> Result<Did, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes_hex: String = serde::Deserialize::deserialize(deserializer)?;
        let bytes = hex::decode(&bytes_hex[2..]).map_err(|e| D::Error::custom(e.to_string()))?;
        Ok(Did::from_bytes(&bytes))
    }
}

impl std::fmt::Display for Did {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl std::fmt::Debug for Did {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "0x{}", self.to_string())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RunbookDid(pub Did);

impl RunbookDid {
    pub fn value(&self) -> Did {
        self.0.clone()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct RunbookId {
    /// Canonical name of the org authoring the workspace
    pub org: Option<String>,
    /// Canonical name of the workspace supporting the runbook
    pub workspace: Option<String>,
    /// Canonical name of the runbook
    pub name: String,
}

impl RunbookId {
    pub fn did(&self) -> RunbookDid {
        let mut comps = vec![];
        if let Some(ref org) = self.org {
            comps.push(org.as_bytes());
        }
        if let Some(ref workspace) = self.workspace {
            comps.push(workspace.as_bytes());
        }
        comps.push(self.name.as_bytes());
        let did = Did::from_components(comps);
        RunbookDid(did)
    }

    pub fn zero() -> RunbookId {
        RunbookId { org: None, workspace: None, name: "".into() }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PackageDid(pub Did);

impl PackageDid {
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct PackageId {
    /// Id of the Runbook
    pub runbook_id: RunbookId,
    /// Location of the package within the workspace
    pub package_location: FileLocation,
    /// Name of the package
    pub package_name: String,
}

impl PackageId {
    pub fn did(&self) -> PackageDid {
        let did = Did::from_components(vec![
            self.runbook_id.did().as_bytes(),
            self.package_name.to_string().as_bytes(),
            // todo(lgalabru): This should be done upstream.
            // Serializing is allowing us to get a canonical location.
            serde_json::json!(self.package_location).to_string().as_bytes(),
        ]);
        PackageDid(did)
    }

    pub fn zero() -> PackageId {
        PackageId {
            runbook_id: RunbookId::zero(),
            package_location: FileLocation::working_dir(),
            package_name: "".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Ord, PartialOrd)]
pub struct ConstructDid(pub Did);

impl ConstructDid {
    pub fn value(&self) -> Did {
        self.0.clone()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    pub fn to_string(&self) -> String {
        self.0.to_string()
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
    pub fn did(&self) -> ConstructDid {
        let did = Did::from_components(vec![
            self.package_id.did().as_bytes(),
            self.construct_type.to_string().as_bytes(),
            self.construct_name.to_string().as_bytes(),
            // todo(lgalabru): This should be done upstream.
            // Serializing is allowing us to get a canonical location.
            serde_json::json!(self.construct_location).to_string().as_bytes(),
        ]);
        ConstructDid(did)
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
    pub name: String,
    pub storage: IndexMap<String, Value>,
}

impl ValueStore {
    pub fn new(name: &str, uuid: &Did) -> ValueStore {
        ValueStore { name: name.to_string(), uuid: uuid.clone(), storage: IndexMap::new() }
    }

    pub fn get_defaulting_string(
        &self,
        key: &str,
        defaults: &AddonDefaults,
    ) -> Result<String, Diagnostic> {
        let Some(value) = self.storage.get(key) else {
            let res = defaults.store.get_expected_string(key)?;
            return Ok(res.to_string());
        };
        let Some(value) = value.as_string() else {
            return Err(Diagnostic::error_from_string(format!(
                "store '{}': value associated to '{}' mismatch (expected string)",
                self.name, key
            )));
        };
        Ok(value.to_string())
    }

    pub fn get_defaulting_integer(
        &self,
        key: &str,
        defaults: &AddonDefaults,
    ) -> Result<i128, Diagnostic> {
        let Some(value) = self.storage.get(key) else {
            let res = defaults.store.get_expected_integer(key)?;
            return Ok(res);
        };
        let Some(value) = value.as_integer() else {
            return Err(Diagnostic::error_from_string(format!(
                "store '{}': value associated to '{}' mismatch (expected uint)",
                self.name, key
            )));
        };
        Ok(value)
    }

    pub fn get_defaulting_uint(
        &self,
        key: &str,
        defaults: &AddonDefaults,
    ) -> Result<u64, Diagnostic> {
        let Some(value) = self.storage.get(key) else {
            let res = defaults.store.get_expected_uint(key)?;
            return Ok(res);
        };
        let Some(value) = value.as_uint() else {
            return Err(Diagnostic::error_from_string(format!(
                "store '{}': value associated to '{}' mismatch (expected positive integer)",
                self.name, key
            )));
        };
        value.map_err(|e| {
            Diagnostic::error_from_string(format!(
                "store '{}': value associated to '{}' mismatch (expected positive integer): {}",
                self.name, key, e
            ))
        })
    }

    pub fn get_optional_defaulting_string(
        &self,
        key: &str,
        defaults: &AddonDefaults,
    ) -> Result<Option<String>, Diagnostic> {
        let Some(value) = self.storage.get(key) else {
            let res = defaults.store.get_string(key).and_then(|s| Some(s.to_string()));
            return Ok(res);
        };
        let Some(value) = value.as_string() else {
            return Err(Diagnostic::error_from_string(format!(
                "store '{}': value associated to '{}' mismatch (expected string)",
                self.name, key
            )));
        };
        Ok(Some(value.to_string()))
    }

    pub fn get_expected_value(&self, key: &str) -> Result<&Value, Diagnostic> {
        let Some(value) = self.storage.get(key) else {
            return Err(Diagnostic::error_from_string(format!(
                "unable to retrieve key '{}' from store '{}'",
                key, self.name,
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

    pub fn get_scoped_bool(&self, scope: &str, key: &str) -> Option<bool> {
        if let Some(Value::Bool(bool)) = self.get_scoped_value(scope, key) {
            Some(*bool)
        } else {
            None
        }
    }

    pub fn clear_autoincrementable_nonce(&mut self) {
        self.storage.swap_remove(&format!("{}:autoincrement", CACHED_NONCE));
    }

    pub fn set_autoincrementable_nonce(&mut self, key: &str, initial_value: u64) {
        self.storage.insert(
            format!("{}:autoincrement", CACHED_NONCE),
            Value::integer((initial_value + 1).into()),
        );
        self.storage
            .insert(format!("{}:{}", CACHED_NONCE, key), Value::integer(initial_value.into()));
    }

    pub fn get_autoincremented_nonce(&mut self, key: &str) -> Option<i128> {
        let value = match self.storage.get(&format!("{}:{}", CACHED_NONCE, key)) {
            None => match self.storage.get(&format!("{}:autoincrement", CACHED_NONCE)) {
                None => return None,
                Some(Value::Integer(value)) => {
                    let value_to_return = value.clone();
                    self.storage.insert(
                        format!("{}:autoincrement", CACHED_NONCE),
                        Value::integer(value_to_return + 1),
                    );
                    self.storage.insert(
                        format!("{}:{}", CACHED_NONCE, key),
                        Value::integer(value_to_return.clone()),
                    );
                    value_to_return
                }
                _ => unreachable!(),
            },
            Some(Value::Integer(value)) => *value,
            _ => unreachable!(),
        };
        Some(value)
    }

    pub fn get_value(&self, key: &str) -> Option<&Value> {
        self.storage.get(key)
    }

    pub fn get_uint(&self, key: &str) -> Result<Option<u64>, String> {
        self.storage.get(key).map(|v| v.expect_uint()).transpose()
    }

    pub fn get_string(&self, key: &str) -> Option<&str> {
        self.storage.get(key).and_then(|v| v.as_string())
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.storage.get(key).and_then(|v| v.as_bool())
    }

    pub fn get_expected_bool(&self, key: &str) -> Result<bool, Diagnostic> {
        let Some(value) = self.storage.get(key) else {
            return Err(Diagnostic::error_from_string(format!(
                "unable to retrieve key '{}' from store '{}'",
                key, self.name,
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
                "unable to retrieve key '{}' from store '{}'",
                key, self.name,
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
                "unable to retrieve key '{}' from store '{}'",
                key, self.name,
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

    pub fn get_expected_object(&self, key: &str) -> Result<IndexMap<String, Value>, Diagnostic> {
        let Some(value) = self.storage.get(key) else {
            return Err(Diagnostic::error_from_string(format!(
                "unable to retrieve key '{}' from store '{}'",
                key, self.name,
            )));
        };
        let Some(result) = value.as_object() else {
            return Err(Diagnostic::error_from_string(format!(
                "store '{}': value associated to '{}' mismatch (expected object)",
                self.name, key
            )));
        };
        Ok(result.clone())
    }

    pub fn get_expected_integer(&self, key: &str) -> Result<i128, Diagnostic> {
        let Some(value) = self.storage.get(key) else {
            return Err(Diagnostic::error_from_string(format!(
                "unable to retrieve key '{}' from store '{}'",
                key, self.name,
            )));
        };
        let Some(value) = value.as_integer() else {
            return Err(Diagnostic::error_from_string(format!(
                "store '{}': value associated to '{}' mismatch (expected integer)",
                self.name, key
            )));
        };
        Ok(value)
    }

    pub fn get_expected_uint(&self, key: &str) -> Result<u64, Diagnostic> {
        let Some(value) = self.storage.get(key) else {
            return Err(Diagnostic::error_from_string(format!(
                "unable to retrieve key '{}' from store '{}'",
                key, self.name,
            )));
        };
        let Some(value) = value.as_uint() else {
            return Err(Diagnostic::error_from_string(format!(
                "store '{}': value associated to '{}' mismatch (expected positive integer)",
                self.name, key
            )));
        };
        value.map_err(|e| {
            Diagnostic::error_from_string(format!(
                "store '{}': value associated to '{}' mismatch (expected positive integer): {}",
                self.name, key, e,
            ))
        })
    }

    pub fn get_expected_buffer_bytes(&self, key: &str) -> Result<Vec<u8>, Diagnostic> {
        let Some(value) = self.storage.get(key) else {
            return Err(Diagnostic::error_from_string(format!(
                "unable to retrieve key '{}' from store '{}'",
                key, self.name,
            )));
        };

        let bytes = match value {
            Value::Buffer(bytes) => bytes.clone(),
            Value::String(bytes) => {
                let bytes = if bytes.starts_with("0x") {
                    crate::hex::decode(&bytes[2..]).unwrap()
                } else {
                    crate::hex::decode(&bytes).unwrap()
                };
                bytes
            }
            Value::Addon(data) => data.bytes.clone(),
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

    pub fn iter(&self) -> indexmap::map::Iter<String, Value> {
        self.storage.iter()
    }

    pub fn len(&self) -> usize {
        self.storage.len()
    }
}

#[derive(Debug, Clone)]
pub struct AuthorizationContext {
    pub workspace_location: FileLocation,
}

impl AuthorizationContext {
    pub fn new(workspace_location: FileLocation) -> Self {
        Self { workspace_location }
    }

    pub fn empty() -> Self {
        Self { workspace_location: FileLocation::working_dir() }
    }
}

#[derive(Debug)]
pub enum ContractSourceTransform {
    FindAndReplace(String, String),
}

pub struct AddonPostProcessingResult {
    pub dependencies: HashMap<ConstructDid, Vec<ConstructDid>>,
    pub transforms: HashMap<ConstructDid, Vec<ContractSourceTransform>>,
}

impl AddonPostProcessingResult {
    pub fn new() -> AddonPostProcessingResult {
        AddonPostProcessingResult { dependencies: HashMap::new(), transforms: HashMap::new() }
    }
}
