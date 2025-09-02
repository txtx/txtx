//! Test that validates ALL runbooks in the fixtures directory

#[cfg(test)]
mod validate_all_runbooks {
    use std::fs;
    use std::path::{Path, PathBuf};
    use crate::tests::fixture_builder::action_schemas::get_action_schema;
    use std::collections::HashMap;
    
    /// Find all .tx files in a directory recursively
    fn find_tx_files(dir: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    files.extend(find_tx_files(&path));
                } else if path.extension().and_then(|s| s.to_str()) == Some("tx") {
                    files.push(path);
                }
            }
        }
        
        files
    }
    
    /// Extract action information from runbook content
    fn extract_actions_from_runbook(content: &str) -> Vec<(String, String, HashMap<String, String>)> {
        let mut actions = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;
        
        while i < lines.len() {
            let line = lines[i].trim();
            
            // Look for action definitions
            if line.starts_with("action ") {
                // Parse: action "name" "namespace::action" {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let name = parts[1].trim_matches('"');
                    let action_type = parts[2].trim_matches('"').trim_matches('{');
                    
                    // Extract fields from the action block
                    let mut fields = HashMap::new();
                    i += 1;
                    
                    while i < lines.len() {
                        let field_line = lines[i].trim();
                        if field_line == "}" {
                            break;
                        }
                        
                        // Parse field: field_name = value
                        if field_line.contains(" = ") {
                            let field_parts: Vec<&str> = field_line.splitn(2, " = ").collect();
                            if field_parts.len() == 2 {
                                let field_name = field_parts[0].trim();
                                let field_value = field_parts[1].trim_end_matches(|c| c == ',' || c == ';');
                                fields.insert(field_name.to_string(), field_value.to_string());
                            }
                        }
                        i += 1;
                    }
                    
                    actions.push((name.to_string(), action_type.to_string(), fields));
                }
            }
            i += 1;
        }
        
        actions
    }
    
    /// Validate a single runbook file
    fn validate_runbook_file(path: &Path) -> Result<(), Vec<String>> {
        let content = fs::read_to_string(path)
            .map_err(|e| vec![format!("Failed to read file: {}", e)])?;
        
        let actions = extract_actions_from_runbook(&content);
        let mut all_errors = Vec::new();
        
        for (name, action_type, fields) in actions {
            // Parse namespace::action
            let parts: Vec<&str> = action_type.split("::").collect();
            if parts.len() != 2 {
                all_errors.push(format!("Action '{}': Invalid type format '{}'", name, action_type));
                continue;
            }
            
            let namespace = parts[0];
            let action = parts[1];
            
            // Get schema and validate
            if let Some(schema) = get_action_schema(namespace, action) {
                // Check required fields
                for field_schema in &schema.fields {
                    if field_schema.required && !fields.contains_key(field_schema.name) {
                        // Special case: signer is often defined separately
                        if field_schema.name != "signer" {
                            all_errors.push(format!(
                                "Action '{}' ({}): Missing required field '{}'",
                                name, action_type, field_schema.name
                            ));
                        }
                    }
                }
                
                // Check for unknown fields (common mistakes)
                for (field_name, _) in &fields {
                    if !schema.fields.iter().any(|f| f.name == field_name) {
                        // Check for common mistakes
                        let suggestion = match field_name.as_str() {
                            "to" if action == "send_eth" => Some("recipient_address"),
                            "from" if action == "send_eth" => Some("(not needed when using signer)"),
                            "value" if action == "send_eth" => Some("amount"),
                            "function_arguments" => Some("function_args"),
                            "contract_address" if action == "deploy_contract" => Some("(output, not input)"),
                            _ => None,
                        };
                        
                        if let Some(correct) = suggestion {
                            all_errors.push(format!(
                                "Action '{}' ({}): Unknown field '{}' - should be '{}'",
                                name, action_type, field_name, correct
                            ));
                        } else if field_name != "description" && field_name != "confirmations" {
                            // Don't warn about common optional fields
                            all_errors.push(format!(
                                "Action '{}' ({}): Unknown field '{}'",
                                name, action_type, field_name
                            ));
                        }
                    }
                }
            }
        }
        
        if all_errors.is_empty() {
            Ok(())
        } else {
            Err(all_errors)
        }
    }
    
    #[test]
    fn validate_all_fixture_runbooks() {
        println!("\n🔍 Validating all runbook fixtures...\n");
        
        // Find all directories that might contain fixtures
        let base_paths = vec![
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures"),
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/tests/fixtures"),
        ];
        
        let mut total_files = 0;
        let mut valid_files = 0;
        let mut files_with_errors = 0;
        let mut all_errors: Vec<(PathBuf, Vec<String>)> = Vec::new();
        
        for base_path in &base_paths {
            if !base_path.exists() {
                continue;
            }
            
            println!("📁 Scanning: {}", base_path.display());
            let tx_files = find_tx_files(base_path);
            
            for tx_file in tx_files {
                total_files += 1;
                let relative_path = tx_file.strip_prefix(env!("CARGO_MANIFEST_DIR"))
                    .unwrap_or(&tx_file);
                
                match validate_runbook_file(&tx_file) {
                    Ok(()) => {
                        valid_files += 1;
                        println!("  ✅ {}", relative_path.display());
                    }
                    Err(errors) => {
                        files_with_errors += 1;
                        println!("  ❌ {}", relative_path.display());
                        for error in &errors {
                            println!("      {}", error);
                        }
                        all_errors.push((tx_file.clone(), errors));
                    }
                }
            }
        }
        
        // Summary
        println!("\n📊 Validation Summary:");
        println!("  Total files scanned: {}", total_files);
        println!("  Valid files: {} ✅", valid_files);
        println!("  Files with errors: {} ❌", files_with_errors);
        
        if !all_errors.is_empty() {
            println!("\n🔧 Common issues found:");
            
            // Group errors by type
            let mut error_counts: HashMap<String, usize> = HashMap::new();
            for (_, errors) in &all_errors {
                for error in errors {
                    if error.contains("Unknown field 'to'") {
                        *error_counts.entry("'to' should be 'recipient_address'".to_string()).or_insert(0) += 1;
                    } else if error.contains("Unknown field 'value'") {
                        *error_counts.entry("'value' should be 'amount'".to_string()).or_insert(0) += 1;
                    } else if error.contains("Unknown field 'from'") {
                        *error_counts.entry("'from' not needed when using signer".to_string()).or_insert(0) += 1;
                    } else if error.contains("Missing required field") {
                        *error_counts.entry("Missing required fields".to_string()).or_insert(0) += 1;
                    }
                }
            }
            
            for (issue, count) in error_counts.iter() {
                println!("  - {}: {} occurrences", issue, count);
            }
        }
        
        // Don't fail the test, just report
        println!("\n💡 This validation helps identify common mistakes in runbook fixtures");
    }
    
    #[test]
    fn validate_our_test_runbooks() {
        println!("\n🔍 Validating our newly created test runbooks...\n");
        
        // Check the specific runbooks we're creating in our tests
        let test_runbooks = vec![
            ("send_eth with wrong fields", r#"
action "send_eth" "evm::send_eth" {
    to = input.bob_address           // WRONG
    value = "100000000000000000"     // WRONG
    from = input.alice_address       // WRONG
    signer = signer.alice
}
"#),
            ("send_eth with correct fields", r#"
action "send_eth" "evm::send_eth" {
    recipient_address = input.bob_address
    amount = "100000000000000000"
    signer = signer.alice
}
"#),
            ("call_contract with wrong fields", r#"
action "call" "evm::call_contract" {
    contract = "0x123..."                    // WRONG: should be contract_address
    abi = "..."                              // WRONG: should be contract_abi
    function = "transfer"                    // WRONG: should be function_name
    function_arguments = ["0x456...", 100]  // WRONG: should be function_args
    signer = signer.alice
}
"#),
            ("call_contract with correct fields", r#"
action "call" "evm::call_contract" {
    contract_address = "0x123..."
    contract_abi = "..."
    function_name = "transfer"
    function_args = ["0x456...", 100]
    signer = signer.alice
}
"#),
        ];
        
        for (description, runbook) in test_runbooks {
            println!("📋 Validating: {}", description);
            let actions = extract_actions_from_runbook(runbook);
            
            for (name, action_type, fields) in actions {
                println!("  Action: {} ({})", name, action_type);
                
                // Parse namespace::action
                let parts: Vec<&str> = action_type.split("::").collect();
                if parts.len() == 2 {
                    let namespace = parts[0];
                    let action = parts[1];
                    
                    if let Some(schema) = get_action_schema(namespace, action) {
                        // Validate
                        let mut errors = Vec::new();
                        
                        // Check for wrong field names
                        for (field_name, _) in &fields {
                            if !schema.fields.iter().any(|f| f.name == field_name) {
                                let suggestion = match (action, field_name.as_str()) {
                                    ("send_eth", "to") => "recipient_address",
                                    ("send_eth", "value") => "amount",
                                    ("send_eth", "from") => "(not needed with signer)",
                                    ("call_contract", "contract") => "contract_address",
                                    ("call_contract", "abi") => "contract_abi",
                                    ("call_contract", "function") => "function_name",
                                    ("call_contract", "function_arguments") => "function_args",
                                    _ => "unknown",
                                };
                                errors.push(format!("    ❌ Field '{}' should be '{}'", field_name, suggestion));
                            }
                        }
                        
                        if errors.is_empty() {
                            println!("    ✅ All fields valid");
                        } else {
                            for error in errors {
                                println!("{}", error);
                            }
                        }
                    } else {
                        println!("    ⚠️  No schema available for validation");
                    }
                }
            }
            println!();
        }
    }
}