//! Tests for HCL diagnostic integration

#[cfg(test)]
mod tests {
    use crate::cli::lsp::diagnostics_hcl_integrated::validate_runbook_with_hcl;
    use lsp_types::Url;

    #[test]
    fn test_hcl_syntax_error_detection() {
        let uri = Url::parse("file:///test.tx").unwrap();

        // Test with invalid HCL syntax
        let content = r#"
addon "evm" {
    chain_id = 1
    # Missing closing brace
"#;

        let diagnostics = validate_runbook_with_hcl(&uri, content);
        assert!(!diagnostics.is_empty(), "Should detect syntax error");

        let first_diag = &diagnostics[0];
        assert!(first_diag.message.contains("parse error") || first_diag.message.contains("HCL"));
        assert_eq!(first_diag.source.as_deref(), Some("hcl-parser"));
    }

    #[test]
    fn test_valid_hcl_with_semantic_errors() {
        let uri = Url::parse("file:///test.tx").unwrap();

        // Valid HCL but with semantic errors
        let content = r#"
action "deploy" "unknown::action" {
    signer = "undefined_signer"
}
"#;

        let diagnostics = validate_runbook_with_hcl(&uri, content);
        // Should have errors for unknown namespace and undefined signer
        assert!(diagnostics.len() >= 1, "Should detect semantic errors");
    }

    #[test]
    fn test_clean_runbook() {
        let uri = Url::parse("file:///test.tx").unwrap();

        // Valid runbook with no errors
        let content = r#"
addon "evm" "ethereum" {
    chain_id = 1
}

variable "contract_address" {
    value = "0x123"
}

output "result" {
    value = variable.contract_address
}
"#;

        let diagnostics = validate_runbook_with_hcl(&uri, content);
        // Should have no errors for a clean runbook
        assert!(
            diagnostics.is_empty()
                || diagnostics
                    .iter()
                    .all(|d| d.severity != Some(lsp_types::DiagnosticSeverity::ERROR)),
            "Should have no errors for valid runbook"
        );
    }

    #[test]
    fn test_hcl_error_position_extraction() {
        use crate::cli::lsp::diagnostics_hcl_integrated::extract_position_from_error;

        // Test various error message formats
        assert_eq!(extract_position_from_error("Error on line 5, column 10"), (5, 10));
        assert_eq!(extract_position_from_error("Syntax error at 3:7"), (3, 7));
        assert_eq!(extract_position_from_error("Parse failed on line 2"), (2, 1));
        assert_eq!(extract_position_from_error("Unknown error"), (1, 1));
    }

    #[test]
    fn test_circular_dependency_detection_in_variables() {
        let uri = Url::parse("file:///test.tx").unwrap();

        // Variables with circular dependency
        let content = r#"
variable "a" {
    value = variable.b
}

variable "b" {
    value = variable.a
}

output "result" {
    value = "test"
}
"#;

        let diagnostics = validate_runbook_with_hcl(&uri, content);

        // Should have circular dependency errors
        let circular_errors: Vec<_> = diagnostics.iter()
            .filter(|d| d.message.contains("circular dependency"))
            .collect();

        assert_eq!(circular_errors.len(), 2, "Should detect 2 circular dependency errors");

        // Verify errors are at different positions
        let positions: Vec<_> = circular_errors.iter()
            .map(|d| (d.range.start.line, d.range.start.character))
            .collect();

        assert_ne!(positions[0], positions[1], "Errors should be at different positions");

        // Check that the error message contains the full cycle
        // Note: The cycle could be represented starting from either node:
        // "a -> b -> a" if starting from 'a', or "b -> a -> b" if starting from 'b'
        // Both are valid representations of the same circular dependency
        assert!(circular_errors[0].message.contains("a -> b -> a") ||
                circular_errors[0].message.contains("b -> a -> b"),
                "Should show complete cycle in error message (either a -> b -> a or b -> a -> b)");
    }

    #[test]
    fn test_three_way_circular_dependency() {
        let uri = Url::parse("file:///test.tx").unwrap();

        let content = r#"
variable "x" {
    value = variable.y
}

variable "y" {
    value = variable.z
}

variable "z" {
    value = variable.x
}
"#;

        let diagnostics = validate_runbook_with_hcl(&uri, content);

        let circular_errors: Vec<_> = diagnostics.iter()
            .filter(|d| d.message.contains("circular dependency"))
            .collect();

        assert_eq!(circular_errors.len(), 2, "Should detect 2 errors for 3-way cycle");

        // Verify the cycle path contains all three variables
        // The cycle can be detected starting from any point, so accept any valid representation
        let valid_cycles = [
            "x -> y -> z -> x",
            "y -> z -> x -> y",
            "z -> x -> y -> z",
        ];

        let contains_valid_cycle = valid_cycles.iter()
            .any(|cycle| circular_errors[0].message.contains(cycle));

        assert!(contains_valid_cycle,
                "Should show complete 3-way cycle, got: {}", circular_errors[0].message);
    }

    #[test]
    fn test_action_circular_dependency() {
        let uri = Url::parse("file:///test.tx").unwrap();

        let content = r#"
action "deploy" "test::action" {
    input = action.setup.output
}

action "setup" "test::action" {
    input = action.deploy.output
}
"#;

        let diagnostics = validate_runbook_with_hcl(&uri, content);

        let circular_errors: Vec<_> = diagnostics.iter()
            .filter(|d| d.message.contains("circular dependency in action"))
            .collect();

        assert_eq!(circular_errors.len(), 2, "Should detect action circular dependency");

        assert!(circular_errors[0].message.contains("deploy -> setup -> deploy") ||
                circular_errors[0].message.contains("setup -> deploy -> setup"),
                "Should show action cycle path");
    }

    #[test]
    fn test_post_condition_self_reference_not_circular() {
        let uri = Url::parse("file:///test.tx").unwrap();

        // Post-conditions execute AFTER the action completes,
        // so self-references are NOT circular dependencies
        let content = r#"
action "fetch_data" "std::send_http_request" {
    url = "https://api.example.com/data"
    method = "GET"

    post_condition {
        assertion = std::assert_eq(200, action.fetch_data.status_code)
        behavior = "halt"
    }
}

output "data" {
    value = action.fetch_data.response_body
}
"#;

        let diagnostics = validate_runbook_with_hcl(&uri, content);

        let circular_errors: Vec<_> = diagnostics.iter()
            .filter(|d| d.message.contains("circular dependency"))
            .collect();

        assert_eq!(circular_errors.len(), 0,
                   "Should NOT detect circular dependency for action self-reference in post_condition");
    }

    #[test]
    fn test_pre_condition_creates_valid_dependency() {
        let uri = Url::parse("file:///test.tx").unwrap();

        // Pre-conditions execute BEFORE the action runs,
        // so they create real dependencies (not circular in this case)
        let content = r#"
action "setup" "std::send_http_request" {
    url = "https://api.example.com/setup"
    method = "POST"
}

action "main_task" "std::send_http_request" {
    url = "https://api.example.com/task"
    method = "GET"

    pre_condition {
        assertion = std::assert_eq(200, action.setup.status_code)
        behavior = "halt"
    }
}
"#;

        let diagnostics = validate_runbook_with_hcl(&uri, content);

        let circular_errors: Vec<_> = diagnostics.iter()
            .filter(|d| d.message.contains("circular dependency"))
            .collect();

        assert_eq!(circular_errors.len(), 0,
                   "Should NOT detect circular dependency for valid pre_condition dependency");
    }

    #[test]
    fn test_multiple_post_conditions_with_self_reference() {
        let uri = Url::parse("file:///test.tx").unwrap();

        // Multiple post_conditions all referencing the same action
        let content = r#"
action "process" "std::send_http_request" {
    url = "https://api.example.com/process"
    method = "POST"
    body = { data = "test" }

    post_condition {
        assertion = std::assert_eq(200, action.process.status_code)
        behavior = "halt"
    }

    post_condition {
        assertion = std::assert_not_null(action.process.response_body.id)
        behavior = "log"
    }

    post_condition {
        retries = 3
        assertion = std::assert_true(action.process.response_body.success)
        behavior = "halt"
    }
}
"#;

        let diagnostics = validate_runbook_with_hcl(&uri, content);

        let circular_errors: Vec<_> = diagnostics.iter()
            .filter(|d| d.message.contains("circular dependency"))
            .collect();

        assert_eq!(circular_errors.len(), 0,
                   "Should NOT detect circular dependency for multiple self-references in post_conditions");
    }

    #[test]
    fn test_no_false_positive_for_valid_dependencies() {
        let uri = Url::parse("file:///test.tx").unwrap();

        // Valid dependency chain without cycles
        let content = r#"
variable "base" {
    value = "initial"
}

variable "derived1" {
    value = "${variable.base}_suffix1"
}

variable "derived2" {
    value = "${variable.derived1}_suffix2"
}

output "final" {
    value = variable.derived2
}
"#;

        let diagnostics = validate_runbook_with_hcl(&uri, content);

        let has_circular = diagnostics.iter()
            .any(|d| d.message.contains("circular"));

        assert!(!has_circular, "Should not detect circular dependency for valid chain");
    }

    #[test]
    fn test_block_type_parameters_recognized() {
        let uri = Url::parse("file:///test.tx").unwrap();

        // Action with block-type parameter (like svm::process_instructions)
        let content = r#"
addon "svm" {
    rpc_api_url = "https://api.devnet.solana.com"
    network_id = "devnet"
}

signer "test_signer" "ed25519" {
    seed = "0x1234"
}

action "process" "svm::process_instructions" {
    signers = [signer.test_signer]
    rpc_api_url = "https://api.devnet.solana.com"

    // This is a block-type parameter, not an attribute
    instruction {
        program_idl = "test_program"
        instruction_name = "initialize"
        sender {
            public_key = signer.test_signer.public_key
        }
    }
}

output "result" {
    value = action.process.signature
}
"#;

        let diagnostics = validate_runbook_with_hcl(&uri, content);

        // Check that we DON'T get "Missing parameter 'instruction'" error
        let missing_instruction_error = diagnostics.iter()
            .any(|d| d.message.contains("Missing parameter 'instruction'"));

        assert!(!missing_instruction_error,
                "Should NOT report 'instruction' as missing when provided as a block");

        // Should also not have the rpc_api_url missing error since it's provided
        let missing_rpc_error = diagnostics.iter()
            .any(|d| d.message.contains("Missing parameter 'rpc_api_url'"));

        assert!(!missing_rpc_error,
                "Should NOT report 'rpc_api_url' as missing when provided");
    }

    #[test]
    fn test_block_type_parameter_missing_error() {
        let uri = Url::parse("file:///test.tx").unwrap();

        // Action missing the required block-type parameter
        let content = r#"
addon "svm" {
    rpc_api_url = "https://api.devnet.solana.com"
    network_id = "devnet"
}

signer "test_signer" "ed25519" {
    seed = "0x1234"
}

action "process" "svm::process_instructions" {
    signers = [signer.test_signer]
    rpc_api_url = "https://api.devnet.solana.com"
    // Missing the required 'instruction' block
}

output "result" {
    value = action.process.signature
}
"#;

        let diagnostics = validate_runbook_with_hcl(&uri, content);

        // Should get "Missing parameter 'instruction'" error when it's actually missing
        let missing_instruction_error = diagnostics.iter()
            .any(|d| d.message.contains("Missing parameter 'instruction'"));

        assert!(missing_instruction_error,
                "Should report 'instruction' as missing when not provided");
    }

    #[test]
    fn test_post_condition_and_pre_condition_allowed_on_actions() {
        let uri = Url::parse("file:///test.tx").unwrap();
        let content = r#"
action "http_request" "std::send_http_request" {
    url = "https://example.com"
    method = "GET"

    pre_condition {
        condition = "1 == 1"
        message = "Pre-condition check"
    }

    post_condition {
        condition = "output.status_code == 200"
        message = "Request should be successful"
    }
}

action "write_test" "std::write_file" {
    path = "/tmp/test.txt"
    content = "test content"

    pre_condition {
        condition = "true"
        message = "Always true"
    }

    post_condition {
        condition = "output.success"
        message = "Write should succeed"
    }
}
"#;

        let diagnostics = validate_runbook_with_hcl(&uri, content);

        // Should NOT report post_condition or pre_condition as invalid parameters
        let has_invalid_post_condition = diagnostics.iter()
            .any(|d| d.message.contains("Invalid parameter 'post_condition'"));
        let has_invalid_pre_condition = diagnostics.iter()
            .any(|d| d.message.contains("Invalid parameter 'pre_condition'"));

        assert!(!has_invalid_post_condition,
                "post_condition should be allowed on all actions, but got: {:?}",
                diagnostics);
        assert!(!has_invalid_pre_condition,
                "pre_condition should be allowed on all actions, but got: {:?}",
                diagnostics);
    }
}
