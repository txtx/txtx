//! Test assertion utilities for comparing outputs and action results

use txtx_addon_kit::types::types::Value;
use txtx_addon_kit::indexmap::IndexMap;
use std::collections::HashMap;

/// Result of a comparison between two values
#[derive(Debug)]
pub struct ComparisonResult {
    pub matches: bool,
    pub differences: Vec<String>,
}

impl ComparisonResult {
    pub fn success() -> Self {
        ComparisonResult {
            matches: true,
            differences: vec![],
        }
    }
    
    pub fn failure(reason: String) -> Self {
        ComparisonResult {
            matches: false,
            differences: vec![reason],
        }
    }
    
    pub fn assert_matches(&self, message: &str) {
        if !self.matches {
            panic!("{}: {}", message, self.differences.join(", "));
        }
    }
}

/// Extension trait for Value comparison
pub trait ValueComparison {
    /// Compare this value with another, supporting nested paths
    fn compare_with(&self, other: &Value) -> ComparisonResult;
    
    /// Get a value at a path (e.g., "action.send_eth.tx_hash")
    fn get_path(&self, path: &str) -> Option<&Value>;
    
    /// Check if a path exists
    fn has_path(&self, path: &str) -> bool;
    
    /// Compare only specific fields in an object
    fn compare_fields(&self, other: &Value, fields: &[&str]) -> ComparisonResult;
}

impl ValueComparison for Value {
    fn compare_with(&self, other: &Value) -> ComparisonResult {
        match (self, other) {
            (Value::Null, Value::Null) => ComparisonResult::success(),
            (Value::Bool(a), Value::Bool(b)) if a == b => ComparisonResult::success(),
            (Value::Integer(a), Value::Integer(b)) if a == b => ComparisonResult::success(),
            (Value::Float(a), Value::Float(b)) if (a - b).abs() < f64::EPSILON => ComparisonResult::success(),
            (Value::String(a), Value::String(b)) if a == b => ComparisonResult::success(),
            (Value::Buffer(a), Value::Buffer(b)) if a == b => ComparisonResult::success(),
            
            // Array comparison
            (Value::Array(a), Value::Array(b)) => {
                if a.len() != b.len() {
                    return ComparisonResult::failure(
                        format!("Array length mismatch: {} vs {}", a.len(), b.len())
                    );
                }
                
                let mut differences = Vec::new();
                for (i, (item_a, item_b)) in a.iter().zip(b.iter()).enumerate() {
                    let result = item_a.compare_with(item_b);
                    if !result.matches {
                        differences.push(format!("[{}]: {}", i, result.differences.join(", ")));
                    }
                }
                
                if differences.is_empty() {
                    ComparisonResult::success()
                } else {
                    ComparisonResult {
                        matches: false,
                        differences,
                    }
                }
            },
            
            // Object comparison
            (Value::Object(a), Value::Object(b)) => {
                let mut differences = Vec::new();
                
                // Check for missing keys in b
                for key in a.keys() {
                    if !b.contains_key(key) {
                        differences.push(format!("Missing key in expected: '{}'", key));
                    }
                }
                
                // Check for extra keys in b
                for key in b.keys() {
                    if !a.contains_key(key) {
                        differences.push(format!("Unexpected key: '{}'", key));
                    }
                }
                
                // Compare values for matching keys
                for (key, value_a) in a.iter() {
                    if let Some(value_b) = b.get(key) {
                        let result = value_a.compare_with(value_b);
                        if !result.matches {
                            differences.push(format!(".{}: {}", key, result.differences.join(", ")));
                        }
                    }
                }
                
                if differences.is_empty() {
                    ComparisonResult::success()
                } else {
                    ComparisonResult {
                        matches: false,
                        differences,
                    }
                }
            },
            
            // Type mismatch
            _ => ComparisonResult::failure(
                format!("Type mismatch: {:?} vs {:?}", 
                    value_type_name(self), 
                    value_type_name(other))
            ),
        }
    }
    
    fn get_path(&self, path: &str) -> Option<&Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = self;
        
        for part in parts {
            match current {
                Value::Object(map) => {
                    current = map.get(part)?;
                },
                Value::Array(arr) => {
                    // Support array indexing like "items.0"
                    if let Ok(index) = part.parse::<usize>() {
                        current = arr.get(index)?;
                    } else {
                        return None;
                    }
                },
                _ => return None,
            }
        }
        
        Some(current)
    }
    
    fn has_path(&self, path: &str) -> bool {
        self.get_path(path).is_some()
    }
    
    fn compare_fields(&self, other: &Value, fields: &[&str]) -> ComparisonResult {
        let mut differences = Vec::new();
        
        // Both must be objects
        let (self_obj, other_obj) = match (self, other) {
            (Value::Object(a), Value::Object(b)) => (a, b),
            _ => return ComparisonResult::failure(
                format!("Both values must be objects for field comparison")
            ),
        };
        
        // Compare only specified fields
        for field in fields {
            match (self_obj.get(*field), other_obj.get(*field)) {
                (Some(a), Some(b)) => {
                    let result = a.compare_with(b);
                    if !result.matches {
                        differences.push(format!(".{}: {}", field, result.differences.join(", ")));
                    }
                },
                (Some(_), None) => {
                    differences.push(format!("Field '{}' missing in expected", field));
                },
                (None, Some(_)) => {
                    differences.push(format!("Field '{}' missing in actual", field));
                },
                (None, None) => {
                    differences.push(format!("Field '{}' missing in both", field));
                },
            }
        }
        
        if differences.is_empty() {
            ComparisonResult::success()
        } else {
            ComparisonResult {
                matches: false,
                differences,
            }
        }
    }
}

fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Integer(_) => "integer",
        Value::Float(_) => "float",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
        Value::Buffer(_) => "buffer",
        Value::Addon(_) => "addon",
    }
}

/// Builder for creating expected Value objects for comparison
pub struct ExpectedValueBuilder {
    value: Value,
}

impl ExpectedValueBuilder {
    pub fn new() -> Self {
        ExpectedValueBuilder {
            value: Value::Object(IndexMap::new()),
        }
    }
    
    pub fn with_field(mut self, key: &str, value: Value) -> Self {
        if let Value::Object(ref mut map) = self.value {
            map.insert(key.to_string(), value);
        }
        self
    }
    
    pub fn with_string(self, key: &str, value: &str) -> Self {
        self.with_field(key, Value::String(value.to_string()))
    }
    
    pub fn with_integer(self, key: &str, value: i128) -> Self {
        self.with_field(key, Value::Integer(value))
    }
    
    pub fn with_bool(self, key: &str, value: bool) -> Self {
        self.with_field(key, Value::Bool(value))
    }
    
    pub fn with_object(self, key: &str, builder: ExpectedValueBuilder) -> Self {
        self.with_field(key, builder.build())
    }
    
    pub fn build(self) -> Value {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simple_comparison() {
        let a = Value::String("hello".to_string());
        let b = Value::String("hello".to_string());
        let result = a.compare_with(&b);
        assert!(result.matches);
    }
    
    #[test]
    fn test_object_comparison() {
        let mut map_a = IndexMap::new();
        map_a.insert("tx_hash".to_string(), Value::String("0x123".to_string()));
        map_a.insert("success".to_string(), Value::Bool(true));
        let a = Value::Object(map_a);
        
        let mut map_b = IndexMap::new();
        map_b.insert("tx_hash".to_string(), Value::String("0x123".to_string()));
        map_b.insert("success".to_string(), Value::Bool(true));
        let b = Value::Object(map_b);
        
        let result = a.compare_with(&b);
        assert!(result.matches);
    }
    
    #[test]
    fn test_path_access() {
        let mut inner = IndexMap::new();
        inner.insert("tx_hash".to_string(), Value::String("0x456".to_string()));
        
        let mut outer = IndexMap::new();
        outer.insert("send_eth".to_string(), Value::Object(inner));
        
        let mut root = IndexMap::new();
        root.insert("action".to_string(), Value::Object(outer));
        
        let value = Value::Object(root);
        
        let tx_hash = value.get_path("action.send_eth.tx_hash");
        assert!(tx_hash.is_some());
        assert_eq!(tx_hash.unwrap(), &Value::String("0x456".to_string()));
    }
    
    #[test]
    fn test_field_comparison() {
        let mut map_a = IndexMap::new();
        map_a.insert("tx_hash".to_string(), Value::String("0x123".to_string()));
        map_a.insert("success".to_string(), Value::Bool(true));
        map_a.insert("gas_used".to_string(), Value::Integer(21000));
        let a = Value::Object(map_a);
        
        let mut map_b = IndexMap::new();
        map_b.insert("tx_hash".to_string(), Value::String("0x123".to_string()));
        map_b.insert("success".to_string(), Value::Bool(true));
        map_b.insert("gas_used".to_string(), Value::Integer(25000)); // Different
        let b = Value::Object(map_b);
        
        // Compare only tx_hash and success
        let result = a.compare_fields(&b, &["tx_hash", "success"]);
        assert!(result.matches);
        
        // Compare all fields including gas_used
        let result = a.compare_fields(&b, &["tx_hash", "success", "gas_used"]);
        assert!(!result.matches);
    }
}