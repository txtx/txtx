// Action schema definitions for better test validation
// This could be auto-generated from the action definitions

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct FieldSchema {
    pub name: &'static str,
    pub field_type: &'static str,
    pub required: bool,
    pub description: &'static str,
}

#[derive(Debug, Clone)]
pub struct ActionSchema {
    pub namespace: &'static str,
    pub action: &'static str,
    pub fields: Vec<FieldSchema>,
}

impl ActionSchema {
    pub fn validate_fields(&self, provided: &HashMap<String, String>) -> Result<(), String> {
        let mut errors = Vec::new();
        
        // Check required fields
        for field in &self.fields {
            if field.required && !provided.contains_key(field.name) {
                errors.push(format!("Missing required field: '{}'", field.name));
            }
        }
        
        // Check unknown fields
        for (key, _) in provided {
            if !self.fields.iter().any(|f| f.name == key) {
                // Try to find similar field names for suggestions
                let suggestion = self.find_similar_field(key);
                if let Some(similar) = suggestion {
                    errors.push(format!("Unknown field: '{}' (did you mean '{}'?)", key, similar));
                } else {
                    errors.push(format!("Unknown field: '{}'", key));
                }
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "Invalid configuration for action '{}' ({}::{}):\n  {}\n\nRequired fields:\n{}\n\nOptional fields:\n{}",
                provided.get("__name__").unwrap_or(&String::new()),
                self.namespace,
                self.action,
                errors.join("\n  "),
                self.format_required_fields(),
                self.format_optional_fields()
            ))
        }
    }
    
    fn find_similar_field(&self, name: &str) -> Option<&'static str> {
        // Simple similarity check - could be improved with edit distance
        let lower = name.to_lowercase();
        
        // Check for exact match ignoring case
        for field in &self.fields {
            if field.name.to_lowercase() == lower {
                return Some(field.name);
            }
        }
        
        // Check for common mistakes
        match name {
            "to" => self.fields.iter().find(|f| f.name == "recipient_address").map(|f| f.name),
            "from" => self.fields.iter().find(|f| f.name == "sender_address").map(|f| f.name),
            "value" => self.fields.iter().find(|f| f.name == "amount").map(|f| f.name),
            _ => None,
        }
    }
    
    fn format_required_fields(&self) -> String {
        self.fields
            .iter()
            .filter(|f| f.required)
            .map(|f| format!("  - {}: {} - {}", f.name, f.field_type, f.description))
            .collect::<Vec<_>>()
            .join("\n")
    }
    
    fn format_optional_fields(&self) -> String {
        self.fields
            .iter()
            .filter(|f| !f.required)
            .map(|f| format!("  - {}: {} - {}", f.name, f.field_type, f.description))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

// Schema definitions for common EVM actions
pub fn get_action_schema(namespace: &str, action: &str) -> Option<ActionSchema> {
    match (namespace, action) {
        ("evm", "send_eth") => Some(ActionSchema {
            namespace: "evm",
            action: "send_eth",
            fields: vec![
                FieldSchema {
                    name: "recipient_address",
                    field_type: "string",
                    required: true,
                    description: "The address to send ETH to",
                },
                FieldSchema {
                    name: "amount",
                    field_type: "string",
                    required: true,
                    description: "Amount of ETH to send in wei",
                },
                FieldSchema {
                    name: "signer",
                    field_type: "signer",
                    required: true,
                    description: "The signer to use for the transaction",
                },
                FieldSchema {
                    name: "confirmations",
                    field_type: "number",
                    required: false,
                    description: "Number of confirmations to wait (default: 1)",
                },
                FieldSchema {
                    name: "gas_limit",
                    field_type: "string",
                    required: false,
                    description: "Gas limit for the transaction",
                },
            ],
        }),
        ("evm", "deploy_contract") => Some(ActionSchema {
            namespace: "evm",
            action: "deploy_contract",
            fields: vec![
                FieldSchema {
                    name: "contract",
                    field_type: "object",
                    required: true,
                    description: "Contract bytecode and ABI",
                },
                FieldSchema {
                    name: "signer",
                    field_type: "signer",
                    required: true,
                    description: "The signer to deploy the contract",
                },
                FieldSchema {
                    name: "constructor_args",
                    field_type: "array",
                    required: false,
                    description: "Constructor arguments",
                },
                FieldSchema {
                    name: "confirmations",
                    field_type: "number",
                    required: false,
                    description: "Number of confirmations to wait",
                },
            ],
        }),
        ("evm", "call_contract") => Some(ActionSchema {
            namespace: "evm",
            action: "call_contract",
            fields: vec![
                FieldSchema {
                    name: "contract_address",
                    field_type: "string",
                    required: true,
                    description: "Address of the contract to call",
                },
                FieldSchema {
                    name: "contract_abi",
                    field_type: "string",
                    required: true,
                    description: "ABI of the contract",
                },
                FieldSchema {
                    name: "function_name",
                    field_type: "string",
                    required: true,
                    description: "Name of the function to call",
                },
                FieldSchema {
                    name: "function_args",
                    field_type: "array",
                    required: false,
                    description: "Arguments to pass to the function",
                },
                FieldSchema {
                    name: "signer",
                    field_type: "signer",
                    required: true,
                    description: "The signer for the transaction",
                },
                FieldSchema {
                    name: "amount",
                    field_type: "string",
                    required: false,
                    description: "Amount of ETH to send with the call",
                },
                FieldSchema {
                    name: "confirmations",
                    field_type: "number",
                    required: false,
                    description: "Number of confirmations to wait",
                },
            ],
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_send_eth() {
        let schema = get_action_schema("evm", "send_eth").unwrap();
        
        // Valid configuration
        let mut fields = HashMap::new();
        fields.insert("recipient_address".to_string(), "0x123...".to_string());
        fields.insert("amount".to_string(), "1000".to_string());
        fields.insert("signer".to_string(), "signer.alice".to_string());
        
        assert!(schema.validate_fields(&fields).is_ok());
        
        // Missing required field
        let mut fields = HashMap::new();
        fields.insert("amount".to_string(), "1000".to_string());
        fields.insert("signer".to_string(), "signer.alice".to_string());
        
        let err = schema.validate_fields(&fields).unwrap_err();
        assert!(err.contains("Missing required field: 'recipient_address'"));
        
        // Wrong field name
        let mut fields = HashMap::new();
        fields.insert("to".to_string(), "0x123...".to_string());
        fields.insert("amount".to_string(), "1000".to_string());
        fields.insert("signer".to_string(), "signer.alice".to_string());
        
        let err = schema.validate_fields(&fields).unwrap_err();
        assert!(err.contains("Unknown field: 'to' (did you mean 'recipient_address'?)"));
        assert!(err.contains("Missing required field: 'recipient_address'"));
    }
}