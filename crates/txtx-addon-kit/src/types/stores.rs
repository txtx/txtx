use std::collections::HashMap;

use indexmap::IndexMap;

use crate::{
    constants::{DocumentationKey, RunbookKey},
    types::{types::ThirdPartySignatureStatus, AuthorizationContext},
};

use super::{
    commands::CommandInput, diagnostics::Diagnostic, types::Value, ConstructDid, Did, CACHED_NONCE,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueStore {
    pub uuid: Did,
    pub name: String,
    pub inputs: ValueMap,
    pub defaults: ValueMap,
}

impl ValueStore {
    pub fn new(name: &str, uuid: &Did) -> ValueStore {
        ValueStore {
            name: name.to_string(),
            uuid: uuid.clone(),
            inputs: ValueMap::new(),
            defaults: ValueMap::new(),
        }
    }
    pub fn tmp() -> ValueStore {
        ValueStore {
            name: "".to_string(),
            uuid: Did::zero(),
            inputs: ValueMap::new(),
            defaults: ValueMap::new(),
        }
    }
    pub fn with_inputs(mut self, inputs: &ValueMap) -> Self {
        self.inputs = inputs.clone();
        self
    }

    pub fn append_inputs(mut self, new_inputs: &ValueMap) -> Self {
        self.inputs = self.inputs.append_store(&new_inputs.store);
        self
    }

    pub fn with_defaults(mut self, defaults: &ValueMap) -> Self {
        self.defaults = defaults.clone();
        self
    }
    pub fn with_inputs_from_map(mut self, inputs: &HashMap<String, Value>) -> Self {
        for (key, value) in inputs.iter() {
            self.inputs.insert(key, value.clone());
        }
        self
    }
    pub fn with_inputs_from_vec(mut self, inputs: &Vec<(String, Value)>) -> Self {
        for (k, v) in inputs.iter() {
            self.inputs.insert(k, v.clone());
        }
        self
    }

    pub fn check(
        self,
        instance_name: &str,
        spec_inputs: &Vec<CommandInput>,
    ) -> Result<Self, Diagnostic> {
        for input in spec_inputs.iter() {
            match input.optional {
                true => continue,
                false => match self.inputs.get_value(&input.name) {
                    // Uncomment for strict type-checking on all values:
                    // Some(value) => match input.check_value(value) {
                    //     Ok(_) => continue,
                    //     Err(e) => return Err(e),
                    // },
                    Some(value) => match input.as_object() {
                        Some(_) => match input.check_value(value) {
                            Ok(_) => continue,
                            Err(e) => return Err(e),
                        },
                        None => continue,
                    },
                    None => match self.defaults.get_value(&input.name) {
                        // Uncomment for strict type-checking on all values:
                        // Some(value) => match input.check_value(value) {
                        //     Ok(_) => continue,
                        //     Err(e) => return Err(e),
                        // },
                        Some(value) => match input.as_object() {
                            Some(_) => match input.check_value(value) {
                                Ok(_) => continue,
                                Err(e) => return Err(e),
                            },
                            None => continue,
                        },
                        None => {
                            return Err(Diagnostic::error_from_string(format!(
                                "Could not execute command '{}': Required input '{}' missing",
                                instance_name, input.name
                            )));
                        }
                    },
                },
            };
        }
        Ok(self)
    }

    // Expected values: if both inputs/defaults yield an error, we should return the input's Diagnostic
    pub fn get_expected_value(&self, key: impl AsRef<str>) -> Result<&Value, Diagnostic> {
        let key = key.as_ref();
        match self.inputs.get_expected_value(key) {
            Ok(val) => Ok(val),
            Err(e) => self.defaults.get_expected_value(key).or(Err(e)),
        }
        .map_err(|e| e)
    }

    pub fn get_expected_construct_did(&self, key: impl AsRef<str>) -> Result<ConstructDid, Diagnostic> {
        let key = key.as_ref();
        match self.inputs.get_expected_construct_did(key) {
            Ok(val) => Ok(val),
            Err(e) => self.defaults.get_expected_construct_did(key).or(Err(e)),
        }
        .map_err(|e| e)
    }

    pub fn get_expected_string(&self, key: impl AsRef<str>) -> Result<&str, Diagnostic> {
        let key = key.as_ref();
        match self.inputs.get_expected_string(key) {
            Ok(val) => Ok(val),
            Err(e) => self.defaults.get_expected_string(key).or(Err(e)),
        }
        .map_err(|e| e)
    }

    pub fn get_expected_integer(&self, key: impl AsRef<str>) -> Result<i128, Diagnostic> {
        let key = key.as_ref();
        match self.inputs.get_expected_integer(key) {
            Ok(val) => Ok(val),
            Err(e) => self.defaults.get_expected_integer(key).or(Err(e)),
        }
        .map_err(|e| e)
    }

    pub fn get_expected_uint(&self, key: impl AsRef<str>) -> Result<u64, Diagnostic> {
        let key = key.as_ref();
        match self.inputs.get_expected_uint(key) {
            Ok(val) => Ok(val),
            Err(e) => self.defaults.get_expected_uint(key).or(Err(e)),
        }
        .map_err(|e| e)
    }
    pub fn get_expected_bool(&self, key: impl AsRef<str>) -> Result<bool, Diagnostic> {
        let key = key.as_ref();
        match self.inputs.get_expected_bool(key) {
            Ok(val) => Ok(val),
            Err(e) => self.defaults.get_expected_bool(key).or(Err(e)),
        }
        .map_err(|e| e)
    }

    pub fn get_expected_array(&self, key: impl AsRef<str>) -> Result<&Vec<Value>, Diagnostic> {
        let key = key.as_ref();
        match self.inputs.get_expected_array(key) {
            Ok(val) => Ok(val),
            Err(e) => self.defaults.get_expected_array(key).or(Err(e)),
        }
        .map_err(|e| e)
    }

    pub fn get_expected_map(&self, key: impl AsRef<str>) -> Result<&Vec<Value>, Diagnostic> {
        let key = key.as_ref();
        match self.inputs.get_expected_map(key) {
            Ok(val) => Ok(val),
            Err(e) => self.defaults.get_expected_map(key).or(Err(e)),
        }
        .map_err(|e| e)
    }

    pub fn get_expected_object(&self, key: impl AsRef<str>) -> Result<IndexMap<String, Value>, Diagnostic> {
        let key = key.as_ref();
        match self.inputs.get_expected_object(key) {
            Ok(val) => Ok(val),
            Err(e) => self.defaults.get_expected_object(key).or(Err(e)),
        }
        .map_err(|e| e)
    }

    pub fn get_expected_buffer_bytes(&self, key: impl AsRef<str>) -> Result<Vec<u8>, Diagnostic> {
        let key = key.as_ref();
        match self.inputs.get_expected_buffer_bytes(key) {
            Ok(val) => Ok(val),
            Err(e) => self.defaults.get_expected_buffer_bytes(key).or(Err(e)),
        }
        .map_err(|e| e)
    }

    // Optional values
    pub fn get_string(&self, key: impl AsRef<str>) -> Option<&str> {
        let key = key.as_ref();
        self.inputs.get_string(key).or(self.defaults.get_string(key))
    }

    pub fn get_value(&self, key: impl AsRef<str>) -> Option<&Value> {
        let key = key.as_ref();
        self.inputs.get_value(key).or(self.defaults.get_value(key))
    }

    pub fn get_uint(&self, key: impl AsRef<str>) -> Result<Option<u64>, String> {
        let key = key.as_ref();
        self.inputs
            .get_uint(key)
            .map_or_else(|_| self.defaults.get_uint(key).map_err(|e| e), |val| Ok(val))
    }

    pub fn get_u8(&self, key: impl AsRef<str>) -> Result<Option<u8>, String> {
        let key = key.as_ref();
        self.inputs
            .get_integer(key)
            .or(self.defaults.get_integer(key))
            .map(|v| {
                u8::try_from(v).map_err(|e| format!("invalid u8 for value '{key}': {e}").into())
            })
            .transpose()
    }

    pub fn get_bool(&self, key: impl AsRef<str>) -> Option<bool> {
        let key = key.as_ref();
        self.inputs.get_bool(key).or(self.defaults.get_bool(key))
    }

    pub fn get_third_party_signature_status(&self) -> Option<ThirdPartySignatureStatus> {
        self.inputs
            .get_third_party_signature_status()
            .or(self.defaults.get_third_party_signature_status())
    }

    pub fn get_integer(&self, key: impl AsRef<str>) -> Option<i128> {
        let key = key.as_ref();
        self.inputs.get_integer(key).or(self.defaults.get_integer(key))
    }

    pub fn get_i64(&self, key: impl AsRef<str>) -> Result<Option<i64>, Diagnostic> {
        let key = key.as_ref();
        self.inputs
            .get_integer(key)
            .or(self.defaults.get_integer(key))
            .map(|v| {
                i64::try_from(v).map_err(|e| format!("invalid i64 for value '{key}': {e}").into())
            })
            .transpose()
    }

    pub fn get_array(&self, key: impl AsRef<str>) -> Option<&Box<Vec<Value>>> {
        let key = key.as_ref();
        self.inputs.get_array(key).or(self.defaults.get_array(key))
    }

    pub fn get_map(&self, key: impl AsRef<str>) -> Option<&Box<Vec<Value>>> {
        let key = key.as_ref();
        self.inputs.get_map(key).or(self.defaults.get_map(key))
    }

    pub fn get_object(&self, key: impl AsRef<str>) -> Option<&IndexMap<String, Value>> {
        let key = key.as_ref();
        self.inputs.get_object(key).or(self.defaults.get_object(key))
    }

    // Scoped values
    pub fn insert_scoped_value(&mut self, scope: &str, key: impl ToString, value: Value) {
        self.inputs.insert(format!("{}:{}", scope, key.to_string()), value);
    }

    pub fn clear_scoped_value(&mut self, scope: &str, key: impl AsRef<str>) {
        let key = key.as_ref();
        self.inputs.store.swap_remove(&format!("{}:{}", scope, key));
    }

    pub fn remove_scoped_value(&mut self, scope: &str, key: impl AsRef<str>) -> Option<Value> {
        let key = key.as_ref();
        self.inputs.store.shift_remove(&format!("{}:{}", scope, key))
    }

    pub fn get_scoped_value(&self, scope: &str, key: impl AsRef<str>) -> Option<&Value> {
        let key = key.as_ref();
        self.inputs.get_value(&format!("{}:{}", scope, key))
    }

    pub fn get_scoped_integer(&self, scope: &str, key: impl AsRef<str>) -> Option<i128> {
        let key = key.as_ref();
        self.inputs.get_integer(&format!("{}:{}", scope, key))
    }

    pub fn get_scoped_bool(&self, scope: &str, key: impl AsRef<str>) -> Option<bool> {
        if let Some(Value::Bool(bool)) = self.get_scoped_value(scope, key) {
            Some(*bool)
        } else {
            None
        }
    }

    pub fn get_expected_scoped_value(&self, scope: &str, key: impl AsRef<str>) -> Result<&Value, Diagnostic> {
        let key = key.as_ref();
        match self.inputs.get_expected_value(&format!("{}:{}", scope, key)) {
            Ok(val) => Ok(val),
            Err(e) => self.defaults.get_expected_value(&format!("{}:{}", scope, key)).or(Err(e)),
        }
        .map_err(|e| e)
    }

    pub fn get_expected_scoped_buffer_bytes(
        &self,
        scope: &str,
        key: &str,
    ) -> Result<Vec<u8>, Diagnostic> {
        match self.inputs.get_expected_buffer_bytes(&format!("{}:{}", scope, key)) {
            Ok(val) => Ok(val),
            Err(e) => {
                self.defaults.get_expected_buffer_bytes(&format!("{}:{}", scope, key)).or(Err(e))
            }
        }
        .map_err(|e| e)
    }

    // Nonce helpers
    pub fn clear_autoincrementable_nonce(&mut self) {
        self.inputs.clear_autoincrementable_nonce();
    }

    pub fn set_autoincrementable_nonce(&mut self, key: impl AsRef<str>, initial_value: u64) {
        let key = key.as_ref();
        self.inputs.set_autoincrementable_nonce(key, initial_value);
    }

    pub fn get_autoincremented_nonce(&mut self, key: impl AsRef<str>) -> Option<i128> {
        let key = key.as_ref();
        self.inputs.get_autoincremented_nonce(key)
    }

    // General helpers
    pub fn insert(&mut self, key: impl ToString, value: Value) {
        self.inputs.insert(key, value);
    }

    pub fn iter(&self) -> indexmap::map::Iter<String, Value> {
        self.inputs.iter()
    }

    pub fn len(&self) -> usize {
        self.inputs.len()
    }
    pub fn get_mut(&mut self, key: impl AsRef<str>) -> Option<&mut Value> {
        let key = key.as_ref();
        self.inputs.get_mut(key)
    }

    pub fn append_no_override(&mut self, other: &ValueStore) {
        for (key, value) in &other.inputs.store {
            if self.inputs.get_value(&key).is_none() {
                self.inputs.insert(&key, value.clone());
            }
        }
    }

    // Other
    pub fn get_markdown(
        &self,
        auth_context: &AuthorizationContext,
    ) -> Result<Option<String>, Diagnostic> {
        let markdown = self
            .inputs
            .get_value(DocumentationKey::Markdown.as_ref())
            .and_then(|v| v.as_string().map(|s| s.to_string()))
            .or_else(|| {
                self.defaults.get_value(DocumentationKey::Markdown.as_ref()).and_then(|v| v.as_string().map(|s| s.to_string()))
            });

        if markdown.is_some() {
            return Ok(markdown);
        }

        let Some(markdown_filepath) = self
            .inputs
            .get_string(DocumentationKey::MarkdownFilepath.as_ref())
            .or_else(|| self.defaults.get_string(DocumentationKey::MarkdownFilepath.as_ref()))
        else {
            return Ok(None);
        };

        let markdown_content_path_buf = std::path::PathBuf::from(markdown_filepath);
        let markdown_content = auth_context
            .get_file_location_from_path_buf(&markdown_content_path_buf)
            .map_err(|e| {
                Diagnostic::error_from_string(format!(
                    "Failed to get file location for markdown file: {}",
                    e
                ))
            })?
            .read_content_as_utf8()
            .map_err(|e| {
                Diagnostic::error_from_string(format!(
                    "Failed to read markdown file content: {}",
                    e
                ))
            })?;

        Ok(Some(markdown_content))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddonDefaults {
    pub uuid: Did,
    pub name: String,
    pub store: ValueMap,
}

impl AddonDefaults {
    pub fn new(key: &str) -> AddonDefaults {
        AddonDefaults { store: ValueMap::new(), name: key.to_string(), uuid: Did::zero() }
    }
    pub fn insert(&mut self, key: impl ToString, value: Value) {
        self.store.insert(key, value);
    }
    pub fn iter(&self) -> indexmap::map::Iter<String, Value> {
        self.store.iter()
    }
    pub fn contains_key(&self, key: impl AsRef<str>) -> bool {
        let key = key.as_ref();
        self.store.contains_key(key)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueMap {
    pub store: IndexMap<String, Value>,
}
impl ValueMap {
    pub fn new() -> ValueMap {
        Self { store: IndexMap::new() }
    }
    pub fn with_store(mut self, store: &IndexMap<String, Value>) -> Self {
        self.store = store.clone();
        self
    }
    pub fn append_store(mut self, new_store: &IndexMap<String, Value>) -> Self {
        for (k, v) in new_store.into_iter() {
            if !self.store.contains_key(k) {
                self.store.insert(k.to_string(), v.clone());
            }
        }
        self
    }

    pub fn get_expected_value(&self, key: impl AsRef<str>) -> Result<&Value, Diagnostic> {
        let key = key.as_ref();
        let Some(value) = self.store.get(key) else {
            return Err(Diagnostic::error_from_string(format!(
                "unable to retrieve value '{}'",
                key
            )));
        };
        Ok(value)
    }

    pub fn get_expected_bool(&self, key: impl AsRef<str>) -> Result<bool, Diagnostic> {
        let key = key.as_ref();
        let Some(value) = self.store.get(key) else {
            return Err(Diagnostic::error_from_string(format!(
                "unable to retrieve bool '{}'",
                key,
            )));
        };
        let Some(value) = value.as_bool() else {
            return Err(Diagnostic::error_from_string(format!(
                "value associated with '{}' type mismatch: expected bool",
                key
            )));
        };
        Ok(value)
    }

    pub fn get_expected_construct_did(&self, key: impl AsRef<str>) -> Result<ConstructDid, Diagnostic> {
        let key = key.as_ref();
        let value = self.get_expected_string(key)?;
        let construct_did = ConstructDid::from_hex_string(value);
        Ok(construct_did)
    }

    pub fn get_expected_string(&self, key: impl AsRef<str>) -> Result<&str, Diagnostic> {
        let key = key.as_ref();
        let Some(value) = self.store.get(key) else {
            return Err(Diagnostic::error_from_string(format!(
                "unable to retrieve string '{}'",
                key,
            )));
        };
        let Some(value) = value.as_string() else {
            return Err(Diagnostic::error_from_string(format!(
                "value associated with '{}' type mismatch: expected string",
                key
            )));
        };
        Ok(value)
    }

    pub fn get_expected_array(&self, key: impl AsRef<str>) -> Result<&Vec<Value>, Diagnostic> {
        let key = key.as_ref();
        let Some(value) = self.store.get(key) else {
            return Err(Diagnostic::error_from_string(format!(
                "unable to retrieve array '{}'",
                key,
            )));
        };
        let Some(value) = value.as_array() else {
            return Err(Diagnostic::error_from_string(format!(
                "value associated with '{}' type mismatch: expected array",
                key
            )));
        };
        Ok(value)
    }

    pub fn get_expected_map(&self, key: impl AsRef<str>) -> Result<&Vec<Value>, Diagnostic> {
        let key = key.as_ref();
        let Some(value) = self.store.get(key) else {
            return Err(Diagnostic::error_from_string(
                format!("unable to retrieve map '{}'", key,),
            ));
        };
        let Some(value) = value.as_array() else {
            return Err(Diagnostic::error_from_string(format!(
                "value associated with '{}' type mismatch: expected map",
                key
            )));
        };
        Ok(value)
    }

    pub fn get_expected_object(&self, key: impl AsRef<str>) -> Result<IndexMap<String, Value>, Diagnostic> {
        let key = key.as_ref();
        let Some(value) = self.store.get(key) else {
            return Err(Diagnostic::error_from_string(format!(
                "unable to retrieve object '{}'",
                key,
            )));
        };
        let Some(result) = value.as_object() else {
            return Err(Diagnostic::error_from_string(format!(
                "value associated with '{}' type mismatch: expected object",
                key
            )));
        };
        Ok(result.clone())
    }

    pub fn get_expected_integer(&self, key: impl AsRef<str>) -> Result<i128, Diagnostic> {
        let key = key.as_ref();
        let Some(value) = self.store.get(key) else {
            return Err(Diagnostic::error_from_string(format!(
                "unable to retrieve integer '{}'",
                key,
            )));
        };
        let Some(value) = value.as_integer() else {
            return Err(Diagnostic::error_from_string(format!(
                "value associated with '{}' type mismatch: expected integer",
                key
            )));
        };
        Ok(value)
    }

    pub fn get_expected_uint(&self, key: impl AsRef<str>) -> Result<u64, Diagnostic> {
        let key = key.as_ref();
        let Some(value) = self.store.get(key) else {
            return Err(Diagnostic::error_from_string(format!(
                "unable to retrieve uint '{}'",
                key,
            )));
        };
        let Some(value) = value.as_uint() else {
            return Err(Diagnostic::error_from_string(format!(
                "value associated with '{}' type mismatch: expected positive integer",
                key
            )));
        };
        value.map_err(|e| {
            Diagnostic::error_from_string(format!(
                "value associated with '{}' type mismatch: expected positive integer: {}",
                key, e,
            ))
        })
    }

    pub fn get_expected_buffer_bytes(&self, key: impl AsRef<str>) -> Result<Vec<u8>, Diagnostic> {
        let key = key.as_ref();
        let Some(value) = self.store.get(key) else {
            return Err(Diagnostic::error_from_string(format!(
                "unable to retrieve buffer '{}'",
                key,
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
                    "value associated with '{}' type mismatch: expected buffer",
                    key
                )))
            }
        };
        Ok(bytes)
    }

    pub fn get_scoped_value(&self, scope: &str, key: impl AsRef<str>) -> Option<&Value> {
        let key = key.as_ref();
        self.store.get(&format!("{}:{}", scope, key))
    }

    pub fn get_scoped_bool(&self, scope: &str, key: impl AsRef<str>) -> Option<bool> {
        if let Some(Value::Bool(bool)) = self.get_scoped_value(scope, key) {
            Some(*bool)
        } else {
            None
        }
    }

    pub fn clear_autoincrementable_nonce(&mut self) {
        self.store.swap_remove(&format!("{}:autoincrement", CACHED_NONCE));
    }

    pub fn set_autoincrementable_nonce(&mut self, key: impl AsRef<str>, initial_value: u64) {
        let key = key.as_ref();
        self.store.insert(
            format!("{}:autoincrement", CACHED_NONCE),
            Value::integer((initial_value + 1).into()),
        );
        self.store
            .insert(format!("{}:{}", CACHED_NONCE, key), Value::integer(initial_value.into()));
    }

    pub fn get_autoincremented_nonce(&mut self, key: impl AsRef<str>) -> Option<i128> {
        let key = key.as_ref();
        let value = match self.store.get(&format!("{}:{}", CACHED_NONCE, key)) {
            None => match self.store.get(&format!("{}:autoincrement", CACHED_NONCE)) {
                None => return None,
                Some(Value::Integer(value)) => {
                    let value_to_return = value.clone();
                    self.store.insert(
                        format!("{}:autoincrement", CACHED_NONCE),
                        Value::integer(value_to_return + 1),
                    );
                    self.store.insert(
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

    pub fn get_value(&self, key: impl AsRef<str>) -> Option<&Value> {
        let key = key.as_ref();
        self.store.get(key)
    }

    pub fn get_uint(&self, key: impl AsRef<str>) -> Result<Option<u64>, String> {
        let key = key.as_ref();
        self.store.get(key).map(|v| v.expect_uint()).transpose()
    }

    pub fn get_integer(&self, key: impl AsRef<str>) -> Option<i128> {
        let key = key.as_ref();
        self.store.get(key).and_then(|v| v.as_integer())
    }

    pub fn get_string(&self, key: impl AsRef<str>) -> Option<&str> {
        let key = key.as_ref();
        self.store.get(key).and_then(|v| v.as_string())
    }

    pub fn get_bool(&self, key: impl AsRef<str>) -> Option<bool> {
        let key = key.as_ref();
        self.store.get(key).and_then(|v| v.as_bool())
    }

    pub fn get_third_party_signature_status(&self) -> Option<ThirdPartySignatureStatus> {
        self.store
            .get(RunbookKey::ThirdPartySignatureStatus.as_ref())
            .and_then(|v| v.as_third_party_signature_status())
    }

    pub fn get_array(&self, key: impl AsRef<str>) -> Option<&Box<Vec<Value>>> {
        let key = key.as_ref();
        self.store.get(key).and_then(|v| v.as_array())
    }

    pub fn get_map(&self, key: impl AsRef<str>) -> Option<&Box<Vec<Value>>> {
        let key = key.as_ref();
        self.store.get(key).and_then(|v| v.as_array())
    }

    pub fn get_object(&self, key: impl AsRef<str>) -> Option<&IndexMap<String, Value>> {
        let key = key.as_ref();
        self.store.get(key).and_then(|v| v.as_object())
    }

    pub fn insert_scoped_value(&mut self, scope: &str, key: impl ToString, value: Value) {
        self.store.insert(format!("{}:{}", scope, key.to_string()), value);
    }
    pub fn insert(&mut self, key: impl ToString, value: Value) {
        self.store.insert(key.to_string(), value);
    }

    pub fn iter(&self) -> indexmap::map::Iter<String, Value> {
        self.store.iter()
    }

    pub fn len(&self) -> usize {
        self.store.len()
    }
    pub fn get_mut(&mut self, key: impl AsRef<str>) -> Option<&mut Value> {
        let key = key.as_ref();
        self.store.get_mut(key)
    }
    pub fn contains_key(&self, key: impl AsRef<str>) -> bool {
        let key = key.as_ref();
        self.store.contains_key(key)
    }
}
