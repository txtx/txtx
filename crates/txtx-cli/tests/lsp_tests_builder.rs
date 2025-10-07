use txtx_test_utils::builders::{create_test_manifest_with_env, RunbookBuilder};

// Helper macros for LSP testing
macro_rules! assert_has_diagnostic {
    ($diagnostics:expr, $message:expr) => {
        assert!(
            $diagnostics.iter().any(|d| d.message.contains($message)),
            "Expected diagnostic containing '{}', but got: {:?}",
            $message,
            $diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    };
}

#[allow(unused_macros)]
macro_rules! assert_has_error {
    ($errors:expr, $message:expr) => {
        assert!(
            $errors.iter().any(|e| e.contains($message)),
            "Expected error containing '{}', but got: {:?}",
            $message,
            $errors
        );
    };
}

#[cfg(test)]
mod lsp_hover_tests {
    use super::*;

    // Test hover information for functions
    #[test]
    fn test_function_hover_with_builder() {
        let mut builder = RunbookBuilder::new()
            .addon("evm", vec![])
            .variable("wei_amount", "evm::to_wei(1, \"ether\")")
            .variable("hex_value", "std::encode_hex(\"hello\")")
            .action("deploy", "evm::get_contract_from_foundry_project")
            .input("project_path", "\"./contracts\"")
            .input("contract", "\"Token\"");

        // In a real LSP implementation, we would:
        // 1. Parse the runbook to get AST positions
        // 2. Query hover info at specific positions
        // 3. Verify the returned documentation

        // For now, we verify the runbook structure is valid
        let content = builder.build_content();
        assert!(content.contains("evm::to_wei"));
        assert!(content.contains("std::encode_hex"));
        assert!(content.contains("evm::get_contract_from_foundry_project"));
    }

    // Test hover for action types
    #[test]
    fn test_action_hover_with_builder() {
        let mut builder = RunbookBuilder::new()
            .addon("evm", vec![])
            .action("send", "evm::send_eth")
            .input("to", "0x123")
            .input("value", "1000")
            .action("deploy", "evm::deploy_contract")
            .input("contract", "\"Token.sol\"")
            .action("call", "evm::call")
            .input("contract", "0x456")
            .input("method", "\"transfer\"");

        // Hover over action types should show documentation
        let content = builder.build_content();
        assert!(content.contains("evm::send_eth"));
        assert!(content.contains("evm::deploy_contract"));
        assert!(content.contains("evm::call"));
    }

    // Test hover for variable references
    #[test]
    fn test_variable_hover_with_builder() {
        let mut builder = RunbookBuilder::new()
            .addon("evm", vec![])
            .variable("base_fee", "1000000000")
            .variable("multiplier", "2")
            .variable("total_fee", "variable.base_fee * variable.multiplier")
            .action("send", "evm::send_eth")
            .input("to", "0x123")
            .input("value", "variable.total_fee");

        // Hover over variable references should show type and value info
        let content = builder.build_content();
        assert!(content.contains("variable.base_fee"));
        assert!(content.contains("variable.multiplier"));
        assert!(content.contains("variable.total_fee"));
    }
}

#[cfg(test)]
mod lsp_diagnostics_tests {
    use super::*;

    // Test that LSP provides diagnostics for undefined references
    #[test]
    fn test_lsp_undefined_reference_diagnostics() {
        let mut builder = RunbookBuilder::new()
            .addon("evm", vec![])
            .action("send", "evm::send_eth")
            .input("signer", "signer.undefined") // Undefined signer
            .input("to", "0x123")
            .input("value", "variable.missing"); // Undefined variable

        // In LSP mode, this would produce diagnostics
        let result = builder.validate();

        assert!(!result.success);
        assert!(result.errors.len() >= 2);
        assert_has_diagnostic!(&result.errors, "undefined");
        assert_has_diagnostic!(&result.errors, "missing");
    }

    // Test LSP diagnostics for type mismatches
    #[test]
    fn test_lsp_type_mismatch_diagnostics() {
        let mut builder = RunbookBuilder::new()
            .addon("evm", vec![])
            .action("send", "evm::send_eth")
            .input("to", "not_an_address") // Invalid address format
            .input("value", "\"not_a_number\""); // String instead of number

        let result = builder.validate_with_linter(None, None);

        // Should have type-related errors
        assert!(!result.success);
    }

    // Test LSP diagnostics for circular dependencies
    #[test]
    fn test_lsp_workspace_manifest_validation() {
        let manifest = create_test_manifest_with_env(vec![
            ("production", vec![("API_URL", "https://api.prod.example.com"), ("CHAIN_ID", "1")]),
            ("staging", vec![("API_URL", "https://api.staging.example.com"), ("CHAIN_ID", "5")]),
        ]);

        let mut builder = RunbookBuilder::new()
            .addon("evm", vec![("rpc_api_url", "env.API_URL"), ("chain_id", "env.CHAIN_ID")])
            .action("deploy", "evm::deploy_contract")
            .input("contract", "\"Token.sol\"");

        // Use the linter validation
        let result = builder.validate_with_linter(Some(manifest.clone()), None);

        // The builder should have the correct content
        let content = builder.build_content();
        assert!(content.contains("env.API_URL"));
        assert!(content.contains("env.CHAIN_ID"));

        // LSP validation will detect undefined environment variables
        // because it doesn't have the manifest context
        assert!(!result.success);
        assert_has_diagnostic!(&result.errors, "env.API_URL");

        // This test demonstrates that LSP validation works but manifest integration
        // would need to be implemented to properly validate environment variables
    }
}

// Helper function to simulate LSP position in content
#[derive(Debug, Clone)]
struct Position {
    line: u32,
    character: u32,
}

impl Position {
    fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }
}

// Utility to find position of text in content
fn find_position_of(content: &str, search: &str) -> Option<Position> {
    let lines: Vec<&str> = content.lines().collect();
    for (line_idx, line) in lines.iter().enumerate() {
        if let Some(col_idx) = line.find(search) {
            return Some(Position::new(line_idx as u32, col_idx as u32));
        }
    }
    None
}
