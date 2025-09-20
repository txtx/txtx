#[cfg(test)]
mod tests {
    use crate::cli::lsp::diagnostics_enhanced::validate_runbook_with_doctor_rules;
    use lsp_types::{DiagnosticSeverity, Url};

    #[test]
    fn test_doctor_rules_integration() {
        let uri = Url::parse("file:///test.tx").unwrap();

        // Test content with various issues that doctor should catch
        let content = r#"
addon "evm" {
  chain_id = 1
  rpc_url = "https://eth.public-rpc.com"
}

// Unknown action type
action "bad" "evm::unknown_action" {
  chain_id = 1
}

// Undefined inputs
action "deploy" "evm::deploy_contract" {
  chain_id = addon.evm.chain_id
  contract = input.undefined_contract
  deployer = input.undefined_deployer
}

// Sensitive data in output
output "private_key" {
  value = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
}
"#;

        // Run validation without manifest (should still catch some issues)
        let diagnostics = validate_runbook_with_doctor_rules(&uri, content, None, None, &[]);

        // Print diagnostics for debugging
        println!("Found {} diagnostics:", diagnostics.len());
        for (i, diag) in diagnostics.iter().enumerate() {
            println!(
                "{}. {} - {}",
                i + 1,
                match diag.severity {
                    Some(DiagnosticSeverity::ERROR) => "ERROR",
                    Some(DiagnosticSeverity::WARNING) => "WARNING",
                    _ => "INFO",
                },
                diag.message
            );
        }

        // We should have at least one diagnostic for the unknown action
        assert!(!diagnostics.is_empty(), "Expected at least one diagnostic");

        // Check for specific issues
        let has_unknown_action = diagnostics
            .iter()
            .any(|d| d.message.contains("unknown_action") || d.message.contains("Unknown action"));
        assert!(has_unknown_action, "Should detect unknown action type");
    }

    #[test]
    fn test_doctor_rules_with_manifest() {
        use crate::cli::lsp::workspace::{Manifest, RunbookRef};
        use std::collections::HashMap;

        let uri = Url::parse("file:///test.tx").unwrap();

        // Create a minimal manifest with correct structure
        let runbooks = vec![RunbookRef {
            name: "test".to_string(),
            location: "test.tx".to_string(),
            absolute_uri: Some(uri.clone()),
        }];

        let manifest = Manifest {
            uri: Url::parse("file:///test/txtx.yml").unwrap(),
            runbooks,
            environments: HashMap::new(),
        };

        let content = r#"
addon "evm" {
  chain_id = 1
  rpc_url = "https://eth.public-rpc.com"
}

// Using undefined inputs
action "deploy" "evm::deploy_contract" {
  chain_id = addon.evm.chain_id
  contract = input.contract_bytecode  // Not defined in manifest
  deployer = input.deployer_address   // Not defined in manifest
}
"#;

        // Run validation with manifest
        let diagnostics =
            validate_runbook_with_doctor_rules(&uri, content, Some(&manifest), None, &[]);

        println!("\nWith manifest - Found {} diagnostics:", diagnostics.len());
        for (i, diag) in diagnostics.iter().enumerate() {
            println!(
                "{}. {} (line {}) - {}",
                i + 1,
                match diag.severity {
                    Some(DiagnosticSeverity::ERROR) => "ERROR",
                    Some(DiagnosticSeverity::WARNING) => "WARNING",
                    _ => "INFO",
                },
                diag.range.start.line,
                diag.message
            );
        }

        // We should detect undefined inputs when manifest is provided
        let has_undefined_input = diagnostics.iter().any(|d| {
            d.message.contains("undefined")
                || d.message.contains("Undefined")
                || d.message.contains("not defined")
        });
        assert!(has_undefined_input, "Should detect undefined inputs with manifest context");
    }
}
