#[cfg(test)]
mod tests {
    use txtx_test_utils::{assert_success, assert_validation_error, RunbookBuilder};

    #[test]
    fn test_problematic_transfer() {
        let content = include_str!(
            "../../../../../../addons/evm/fixtures/doctor_demo/runbooks/problematic_transfer.tx"
        );
        let result = RunbookBuilder::new().with_content(content).validate();

        // This fixture has 4 errors: from, to, value, gas_used
        assert_eq!(result.errors.len(), 4, "Expected 4 errors in problematic_transfer.tx");

        // Check specific errors
        let error_messages: Vec<_> = result.errors.iter().map(|e| &e.message).collect();

        // Check that we have errors about invalid field access
        assert!(error_messages.iter().any(|m| m.contains("Field 'result' does not exist")));
        assert!(error_messages.iter().any(|m| m.contains("Field 'value' does not exist")));

        // All errors should mention the available outputs
        assert!(error_messages.iter().all(|m| m.contains("Available outputs: tx_hash")));
    }

    #[test]
    fn test_correct_transfer() {
        let content = include_str!(
            "../../../../../../addons/evm/fixtures/doctor_demo/runbooks/correct_transfer.tx"
        );
        let result = RunbookBuilder::new().with_content(content).validate();

        assert_success!(result);
    }

    #[test]
    fn test_undefined_action() {
        // Take a valid fixture and break it by referencing undefined action
        let valid = include_str!(
            "../../../../../../addons/evm/fixtures/doctor_demo/runbooks/correct_transfer.tx"
        );
        let broken = valid.replace("action.transfer.tx_hash", "action.nonexistent.tx_hash");

        let result = RunbookBuilder::new().with_content(&broken).validate();

        assert_eq!(
            result.errors.len(),
            2,
            "Expected 2 errors (one for each reference to nonexistent action)"
        );
        assert_validation_error!(result, "undefined action");
    }

    #[test]
    fn test_send_eth_invalid_field_access() {
        let result = RunbookBuilder::new()
            .addon("evm", vec![("network_id", "1")])
            .action("send", "evm::send_eth")
            .input("from", "0x123")
            .input("to", "0x456")
            .input("value", "1000")
            .output("bad", "action.send.from")
            .validate();

        assert_validation_error!(result, "Field 'from' does not exist");
        assert_validation_error!(result, "send_eth");
        assert_validation_error!(result, "Available outputs: tx_hash");
    }

    #[test]
    fn test_invalid_action_fields() {
        // This test validates that BOTH invalid fields AND missing required parameters are detected
        // Table-driven test for common invalid field access patterns
        let test_cases = vec![
            ("evm::send_eth", "from", "Field 'from' does not exist"),
            ("evm::send_eth", "to", "Field 'to' does not exist"),
            ("evm::send_eth", "gas", "Field 'gas' does not exist"),
            ("evm::send_eth", "gas_used", "Field 'gas_used' does not exist"),
        ];

        for (action_type, field, expected_error) in test_cases {
            let result = RunbookBuilder::new()
                .addon("evm", vec![("network_id", "1")])
                .action("test", action_type)
                .input("value", "1000")
                .output("bad", &format!("action.test.{}", field))
                .validate();

            // We expect multiple errors: one for invalid field access + errors for missing required params
            assert!(result.errors.len() > 1, "Testing field '{}' on {}: expected multiple validation errors", field, action_type);
            assert_validation_error!(result, expected_error);
            // Also check for missing required parameters
            assert_validation_error!(result, "Missing required parameter");
        }
    }

    #[test]
    fn test_nested_invalid_field_access() {
        let result = RunbookBuilder::new()
            .addon("evm", vec![("network_id", "1")])
            .action("send", "evm::send_eth")
            .input("value", "1000")
            .variable("bad", "action.send.from")
            .output("also_bad", "input.bad")
            .validate();

        // HCL validator catches the first error but not the cascading error
        // This is actually good behavior - it avoids noise from cascading errors
        assert!(result.errors.len() >= 1);
        assert_validation_error!(result, "Field 'from' does not exist");
    }

    #[test]
    fn test_unknown_namespace() {
        let result =
            RunbookBuilder::new().with_content(r#"action "test" "unknown::action" {}"#).validate();

        assert_validation_error!(result, "Unknown addon namespace 'unknown'");
    }

    #[test]
    fn test_unknown_action_type() {
        let result = RunbookBuilder::new()
            .addon("evm", vec![("network_id", "1")])
            .action("test", "evm::unknown_action")
            .validate();

        assert_validation_error!(result, "Unknown action type 'evm::unknown_action'");
    }

    #[test]
    fn test_invalid_action_type_format() {
        let test_cases = vec![
            ("no_namespace", "must be in format 'namespace::action'"),
            ("too::many::colons", "Unknown addon namespace 'too'"), // Different error - unknown namespace
            ("", "must be in format 'namespace::action'"),
            ("::", "Unknown addon namespace ''"), // Empty namespace error
            ("namespace:", "must be in format 'namespace::action'"),
            (":action", "must be in format 'namespace::action'"),
        ];

        for (invalid_type, expected_error) in test_cases {
            let result = RunbookBuilder::new()
                .with_content(&format!(r#"action "test" "{}" {{}}"#, invalid_type))
                .validate();

            if !result.success && !result.errors[0].message.contains(expected_error) {
                println!(
                    "DEBUG: Testing '{}', expected '{}', got '{}'",
                    invalid_type, expected_error, result.errors[0].message
                );
            }
            assert_validation_error!(result, expected_error);
        }
    }

    #[test]
    fn test_multiple_errors_in_one_runbook() {
        let result = RunbookBuilder::new()
            .with_content(
                r#"
                addon "evm" { network_id = 1 }
                
                action "a1" "unknown::action" {}
                action "a2" "evm::unknown_action" {}
                action "a3" "evm::send_eth" { value = "100" }
                
                output "o1" { value = action.a3.from }
                output "o2" { value = action.undefined.field }
                output "o3" { value = input.missing }
            "#,
            )
            .validate();

        // Should have multiple distinct errors
        // The HCL validator catches 4 errors: unknown namespace, unknown action, field access, undefined action
        assert!(
            result.errors.len() >= 4,
            "Expected at least 4 errors, got {}",
            result.errors.len()
        );

        // Check we have different types of errors
        let error_messages = result.errors.iter().map(|e| &e.message).collect::<Vec<_>>();

        assert!(error_messages.iter().any(|m| m.contains("Unknown addon namespace")));
        assert!(error_messages.iter().any(|m| m.contains("Unknown action type")));
        assert!(error_messages.iter().any(|m| m.contains("Field 'from' does not exist")));
        assert!(error_messages.iter().any(|m| m.contains("undefined action")));
    }

    #[test]
    fn test_cascading_errors_suppressed() {
        let result = RunbookBuilder::new()
            .with_content(
                r#"
                action "broken" "unknown::action" {}
                
                output "o1" { value = action.broken.field1 }
                output "o2" { value = action.broken.field2 }
                output "o3" { value = action.broken.field3 }
            "#,
            )
            .validate();

        // Should only have the namespace error, not cascading field access errors
        assert_eq!(result.errors.len(), 1);
        assert_validation_error!(result, "Unknown addon namespace");
    }

    #[test]
    fn test_missing_input_in_environment() {
        // This test validates that undefined input references are caught
        let result = RunbookBuilder::new()
            .with_content(
                r#"
                output "test" {
                    value = input.MISSING_VAR
                }
            "#,
            )
            .with_environment("prod", vec![("OTHER_VAR", "value")])
            .set_current_environment("prod") // Enable manifest validation
            .validate();

        // HCL validation should catch undefined input reference
        assert_validation_error!(result, "MISSING_VAR");
    }

    #[test]
    fn test_environment_global_inheritance() {
        let result = RunbookBuilder::new()
            .with_content(
                r#"
                variable "key1" { value = env.KEY1 }
                variable "key2" { value = env.KEY2 }
                
                output "k1" { value = input.key1 }
                output "k2" { value = input.key2 }
            "#,
            )
            .with_environment("global", vec![("KEY1", "global1")])
            .with_environment("test", vec![("KEY2", "test2")])
            .validate();

        // Should pass - test env should inherit from global
        assert_success!(result);
    }

    #[test]
    fn test_cli_inputs_override_environment() {
        let result = RunbookBuilder::new()
            .with_content(
                r#"
                variable "key" { value = env.KEY }
                output "result" { value = input.key }
            "#,
            )
            .with_environment("test", vec![("KEY", "env_value")])
            .with_cli_input("key", "cli_value")
            .validate();

        assert_success!(result);
    }

    #[test]
    fn test_cli_precedence_note() {
        let result = RunbookBuilder::new()
            .with_content(
                r#"
                variable "key" { value = env.MISSING }
                output "result" { value = input.key }
            "#,
            )
            .with_cli_input("key", "cli_value")
            .validate();

        // Should succeed because CLI input overrides the missing env var
        assert_success!(result);
    }

    #[test]
    fn test_invalid_action_parameter() {
        // Test the exact scenario from the user's example
        let result = RunbookBuilder::new()
            .with_content(
                r#"
                addon "evm" {
                  network_id = 1
                  rpc_api_url = "https://eth.llamarpc.com"
                }

                signer "alice" "evm::web_wallet" {
                  expected_address = input.alice
                }

                action "fund" "evm::send_eth" {
                  from = signer.alice
                  confirmations = 1
                }

                output "fund_output" {
                  value = {
                    bad = action.fund.from
                  }
                }
            "#,
            )
            .validate();

        // Should have errors for invalid 'from' parameter and invalid field access
        assert!(result.errors.len() >= 2, "Expected at least 2 errors");
        
        // Check for invalid parameter error
        assert_validation_error!(result, "Invalid parameter 'from'");
        assert_validation_error!(result, "Available parameters:");
        assert_validation_error!(result, "signer");
        
        // Check for invalid field access error
        assert_validation_error!(result, "Field 'from' does not exist");
    }

    #[test]
    fn test_missing_required_action_parameter() {
        let result = RunbookBuilder::new()
            .with_content(
                r#"
                addon "evm" {
                  network_id = 1
                }

                action "send" "evm::send_eth" {
                  # Missing required 'signer' parameter
                  recipient_address = "0x123"
                  amount = "1000"
                }
            "#,
            )
            .validate();

        assert_validation_error!(result, "Missing required parameter 'signer'");
    }

    #[test]
    fn test_isolated_missing_required_parameters() {
        // This test ONLY checks for missing required parameters
        // by providing a valid action with all correct parameter names but missing some required ones
        let result = RunbookBuilder::new()
            .with_content(
                r#"
                addon "evm" {
                  network_id = 1
                  rpc_api_url = "https://eth.llamarpc.com"
                }

                signer "alice" "evm::web_wallet" {
                  expected_address = input.alice
                }

                action "send" "evm::send_eth" {
                  # Has signer (required) but missing recipient_address (also required)
                  # Note: amount is optional, not required
                  signer = signer.alice
                }
            "#,
            )
            .with_cli_input("alice", "0x456")  // Provide alice input to avoid that error
            .validate();

        // Should have exactly 1 error for the missing required parameter (recipient_address)
        assert_eq!(result.errors.len(), 1, "Expected exactly 1 missing required parameter error");
        assert_validation_error!(result, "Missing required parameter 'recipient_address'");
    }

    #[test]
    fn test_isolated_invalid_parameter_names() {
        // This test ONLY checks for invalid parameter names
        // by providing all required parameters plus some invalid ones
        let result = RunbookBuilder::new()
            .with_content(
                r#"
                addon "evm" {
                  network_id = 1
                  rpc_api_url = "https://eth.llamarpc.com"
                }

                signer "alice" "evm::web_wallet" {
                  expected_address = input.alice
                }

                action "send" "evm::send_eth" {
                  # All required parameters are present
                  signer = signer.alice
                  recipient_address = "0x123"
                  amount = 1000

                  # Invalid parameters - these should trigger validation errors
                  from = signer.alice
                  to = "0x456"
                  gas = 21000
                }
            "#,
            )
            .with_cli_input("alice", "0x789")  // Provide alice input to avoid that error
            .validate();

        // Should have exactly 3 errors for the 3 invalid parameters
        assert_eq!(result.errors.len(), 3, "Expected exactly 3 invalid parameter errors");
        assert_validation_error!(result, "Invalid parameter 'from'");
        assert_validation_error!(result, "Invalid parameter 'to'");
        assert_validation_error!(result, "Invalid parameter 'gas'");
    }

    #[test]
    fn test_error_position_handling() {
        // Test that missing position information (0, 0) is handled correctly
        // This would occur if span information is missing from the HCL parser
        let result = RunbookBuilder::new()
            .with_content(
                r#"
                addon "evm" {
                  network_id = 1
                  rpc_api_url = "https://eth.llamarpc.com"
                }

                action "test" "evm::send_eth" {
                  # Missing required parameter should generate error
                  signer = "0x123"
                }
            "#,
            )
            .validate();

        // Should have error for missing required parameter
        assert!(!result.errors.is_empty(), "Should have validation errors");

        // Check that error position is handled correctly
        // Diagnostics use span with line_start/column_start
        for error in &result.errors {
            if let Some(span) = &error.span {
                // Line numbers should never be 0 when span is present
                // A span with line_start = 0 means position is unknown
                // and ideally shouldn't be present (span should be None)
                assert!(
                    span.line_start > 0,
                    "Line number should be > 0 when span is present, got {}",
                    span.line_start
                );
            }
            // When position is truly unknown, span should be None
            // rather than Some(span) with 0 values
        }
    }

    #[test]
    fn test_action_with_all_valid_parameters() {
        // This test verifies that when all required parameters are present
        // and no invalid parameters are used, there are no validation errors
        let result = RunbookBuilder::new()
            .with_content(
                r#"
                addon "evm" {
                  network_id = 1
                  rpc_api_url = "https://eth.llamarpc.com"
                }

                signer "alice" "evm::web_wallet" {
                  expected_address = input.alice
                }

                action "send" "evm::send_eth" {
                  signer = signer.alice
                  recipient_address = "0x123"
                  amount = 1000
                  confirmations = 1
                }

                output "result" {
                  value = action.send.tx_hash
                }
            "#,
            )
            .with_cli_input("alice", "0xabc")  // Provide alice input
            .validate();

        // Should have no validation errors
        assert_success!(result);
    }
}
