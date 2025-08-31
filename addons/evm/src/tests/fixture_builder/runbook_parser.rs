// Runbook parser that leverages txtx's parsing to generate test outputs

use std::collections::HashMap;
use txtx_addon_kit::helpers::hcl::RawHclContent;
use txtx_addon_kit::hcl::structure::BlockLabel;

/// Parses a runbook file and extracts actions with their expected outputs
pub struct RunbookParser {
    content: String,
}

impl RunbookParser {
    pub fn new(content: String) -> Self {
        Self { content }
    }

    /// Parse the runbook and extract all actions using txtx-core's HCL parser
    pub fn parse_actions(&self) -> Result<Vec<ActionInfo>, String> {
        let mut actions = Vec::new();
        
        // Parse HCL content using txtx-core's parser (use from_string instead of new)
        let raw_content = RawHclContent::from_string(self.content.clone());
        let blocks = raw_content.into_blocks()
            .map_err(|e| format!("Failed to parse HCL: {:?}", e))?;
        
        // Extract action blocks
        for block in blocks {
            if block.ident.value().as_str() == "action" {
                // Get action name and type from labels
                let name = block.labels.get(0)
                    .and_then(|l| match l {
                        BlockLabel::String(s) => Some(s.to_string()),
                        _ => None,
                    })
                    .ok_or_else(|| "Action missing name label".to_string())?;
                
                let action_type = block.labels.get(1)
                    .and_then(|l| match l {
                        BlockLabel::String(s) => Some(s.to_string()),
                        _ => None,
                    })
                    .ok_or_else(|| format!("Action '{}' missing type label", name))?;
                
                // Extract description from attributes if present
                let description = block.body.attributes()
                    .find(|attr| attr.key.value().as_str() == "description")
                    .and_then(|attr| {
                        // Try to extract string value from expression
                        match &attr.value {
                            txtx_addon_kit::hcl::expr::Expression::String(s) => Some(s.value().to_string()),
                            _ => None,
                        }
                    })
                    .unwrap_or_else(|| format!("Action {}", name));
                
                actions.push(ActionInfo {
                    name,
                    action_type,
                    description,
                    expected_outputs: HashMap::new(),
                });
            }
        }
        
        Ok(actions)
    }

    /// Inject outputs into the runbook content
    pub fn inject_outputs(&self) -> String {
        let actions = match self.parse_actions() {
            Ok(actions) => actions,
            Err(e) => {
                eprintln!("Warning: Failed to parse actions: {}", e);
                return self.content.clone();
            }
        };
        
        if actions.is_empty() {
            return self.content.clone();
        }
        
        let outputs = self.generate_outputs(&actions);
        format!("{}\n\n{}", self.content, outputs)
    }
    
    /// Generate output blocks for each action
    pub fn generate_outputs(&self, actions: &[ActionInfo]) -> String {
        let mut outputs = Vec::new();

        // Generate individual outputs for each action
        for action in actions {
            outputs.push(format!(
                r#"output "{}_result" {{
  value = action.{}.result
}}"#,
                action.name, action.name
            ));
        }

        // Generate aggregate test output
        let test_output_values: Vec<String> = actions
            .iter()
            .map(|a| format!("    {}_result = action.{}.result", a.name, a.name))
            .collect();

        outputs.push(format!(
            r#"output "test_output" {{
  value = {{
{}
  }}
}}"#,
            test_output_values.join("\n")
        ));

        // Generate test metadata
        let metadata_values: Vec<String> = actions
            .iter()
            .map(|a| {
                format!(
                    r#"    {} = {{
      type = "{}"
      description = "{}"
    }}"#,
                    a.name, a.action_type, a.description
                )
            })
            .collect();

        outputs.push(format!(
            r#"output "test_metadata" {{
  value = {{
{}
  }}
}}"#,
            metadata_values.join("\n")
        ));

        outputs.join("\n\n")
    }
}

#[derive(Debug, Clone)]
pub struct ActionInfo {
    pub name: String,
    pub action_type: String,
    pub description: String,
    pub expected_outputs: HashMap<String, String>,
}

impl ActionInfo {
    /// Get the expected output fields for this action type
    pub fn output_fields(&self) -> Vec<&'static str> {
        // Parse action type to get the action name
        let parts: Vec<&str> = self.action_type.split("::").collect();
        let action_name = if parts.len() == 2 {
            parts[1]
        } else {
            &self.action_type
        };
        
        match action_name {
            "deploy_contract" => vec![
                "tx_hash", "contract_address", "logs", "raw_logs", 
                "gas_used", "deployed_bytecode", "success"
            ],
            "call_contract" | "call_contract_function" => vec![
                "tx_hash", "logs", "raw_logs", "gas_used", 
                "return_value", "success", "decoded_output"
            ],
            "send_eth" => vec![
                "tx_hash", "gas_used", "success", "from", "to", "value"
            ],
            "sign_transaction" => vec![
                "signed_transaction", "tx_hash", "from", "to", "value", "gas"
            ],
            "broadcast_transaction" => vec![
                "tx_hash", "success", "gas_used", "logs", "raw_logs"
            ],
            _ => vec!["tx_hash", "logs", "raw_logs", "success"]
        }
    }
}

/// Parse action outputs from JSON results (for validation)
pub fn parse_action_outputs(json: &serde_json::Value) -> HashMap<String, HashMap<String, serde_json::Value>> {
    let mut results = HashMap::new();
    
    if let Some(outputs) = json.as_object() {
        for (key, value) in outputs {
            if key.ends_with("_result") {
                let action_name = key.trim_end_matches("_result");
                if let Some(action_outputs) = value.as_object() {
                    let mut action_map = HashMap::new();
                    for (field, val) in action_outputs {
                        action_map.insert(field.clone(), val.clone());
                    }
                    results.insert(action_name.to_string(), action_map);
                }
            }
        }
    }
    
    results
}