use txtx_core::manifest::WorkspaceManifest;
use txtx_test_utils::builders::{create_test_manifest_with_env, RunbookBuilder};

// Helper macros for common assertions
macro_rules! assert_validation_error {
    ($result:expr, $expected:expr) => {
        assert!(!$result.success, "Expected validation to fail");
        assert!(
            $result.errors.iter().any(|e| e.message.contains($expected)),
            "Expected error containing '{}', but got: {:?}",
            $expected,
            $result.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    };
}

macro_rules! assert_validation_passes {
    ($result:expr) => {
        assert!(
            $result.success,
            "Expected validation to succeed, but got errors: {:?}",
            $result.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    };
}

#[cfg(test)]
mod doctor_fixture_tests {
    use super::*;

    // Test case 1: test_doctor_simple.tx
    // Expected errors: 2
    // 1. Undefined signer: signer.undefined_signer
    // 2. Invalid field access: action.send.from (send_eth has no 'from' output)
    #[test]
    fn test_doctor_simple_with_builder() {
        let mut builder = RunbookBuilder::new()
            .action("send", "evm::send_eth")
            .input("signer", "signer.undefined_signer") // ERROR: signer not defined
            .input("to", "0x123")
            .input("value", "1000")
            .output("bad", "action.send.from"); // ERROR: send_eth only outputs 'tx_hash'

        let result = builder.validate();

        // Should have 2 errors
        assert!(!result.success);
        assert_eq!(
            result.errors.len(),
            2,
            "Expected 2 errors, got: {:?}",
            result.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );

        // Check specific errors
        assert_validation_error!(result, "undefined_signer");
        assert_validation_error!(result, "from");
    }

    // Test case 2: test_doctor_valid.tx
    // Test file with no errors
    #[test]
    fn test_doctor_valid_with_builder() {
        let mut builder = RunbookBuilder::new()
            .addon("evm", vec![("rpc_api_url", "\"https://eth.example.com\"")])
            // Define a signer
            .signer("operator", "evm::private_key", vec![("private_key", "0x1234")])
            // Action 1 references the signer
            .action("action1", "evm::send_eth")
            .input("from", "signer.operator.address")
            .input("to", "0x456")
            .input("value", "1000")
            // Action 2 references action1 (forward reference is OK)
            .action("action2", "evm::send_eth")
            .input("from", "signer.operator.address")
            .input("to", "0x789")
            .input("value", "2000")
            .input("depends_on", "[action.action1.tx_hash]")
            // Output references both actions
            .output("tx1", "action.action1.tx_hash")
            .output("tx2", "action.action2.tx_hash");

        let result = builder.validate();
        assert_validation_passes!(result);
    }

    // Test case 3: test_doctor_two_pass.tx
    // Should find undefined action reference
    #[test]
    fn test_doctor_two_pass_with_builder() {
        let mut builder = RunbookBuilder::new()
            .addon("evm", vec![])
            .action("first", "evm::send_eth")
            .input("to", "0x123")
            .input("value", "1000")
            .output("result", "action.second.tx_hash"); // ERROR: 'second' action not defined

        let result = builder.validate();

        assert!(!result.success);
        assert_eq!(result.errors.len(), 1, "Expected 1 error");
        assert_validation_error!(result, "second");
    }

    // Test case 4: test_doctor_unknown_action_type.tx
    // Should find unknown action type
    #[test]
    fn test_doctor_unknown_action_type_with_builder() {
        let mut builder =
            RunbookBuilder::new().addon("evm", vec![]).action("test", "evm::unknown_action"); // ERROR: unknown action type

        let result = builder.validate();

        assert!(!result.success);
        assert_eq!(result.errors.len(), 1, "Expected 1 error");
        assert_validation_error!(result, "unknown_action");
    }

    // Test case 5: test_doctor_flow_missing_variable.tx
    // Should find undefined flow variable and usage error
    #[test]
    fn test_doctor_flow_missing_variable_with_builder() {
        // Doctor mode now uses the same HCL validator as production
        let mut builder = RunbookBuilder::new()
            .with_content(r#"
                addon "evm" {}
                
                flow "deploy" {
                    some_var = "test"
                }
                
                signer "test_signer" "evm::secret_key" {
                    secret_key = "0x1234567890123456789012345678901234567890123456789012345678901234"
                }
                
                action "send" "evm::send_eth" {
                    signer = signer.test_signer
                    to = flow.undefined_var  // ERROR: undefined flow variable
                    value = "1000"
                }
            "#);

        let result = builder.validate_with_doctor(None, None);

        assert_validation_error!(result, "undefined_var");
    }

    // Test case 6: Multiple errors combined
    #[test]
    fn test_doctor_multiple_errors_with_builder() {
        let mut builder = RunbookBuilder::new()
            .addon("evm", vec![])
            // Multiple errors in one runbook
            .action("send1", "evm::send_eth")
            .input("signer", "signer.missing") // ERROR: undefined signer
            .input("to", "0x123")
            .input("value", "1000")
            .action("send2", "evm::invalid_action") // ERROR: invalid action type
            .input("param", "value")
            .output("bad1", "action.send1.invalid") // ERROR: invalid field
            .output("bad2", "action.missing.tx_hash"); // ERROR: undefined action

        let result = builder.validate();

        assert!(!result.success);
        assert!(
            result.errors.len() >= 4,
            "Expected at least 4 errors, got: {:?}",
            result.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    // Test environment variable validation
    #[test]
    fn test_variable_resolution_cli_input() {
        // Test that variables can be resolved via CLI input, even when env var is missing
        let result = RunbookBuilder::new()
            .with_content(
                r#"
variable "api_key" {
    value = env.API_KEY
}
output "key" {
    value = variable.api_key
}
"#,
            )
            .with_environment("test", vec![]) // Empty environment - API_KEY not provided
            .set_current_environment("test")
            .with_cli_input("API_KEY", "cli-provided-key")
            .validate();

        // Should pass - variable is resolved via CLI input
        assert_validation_passes!(result);
    }

    #[test]
    fn test_variable_resolution_env_var() {
        // Test that variables can be resolved via environment variables
        let result = RunbookBuilder::new()
            .with_content(
                r#"
variable "api_key" {
    value = env.API_KEY
}
output "key" {
    value = variable.api_key
}
"#,
            )
            .with_environment("test", vec![("API_KEY", "env-provided-key")])
            .set_current_environment("test")
            .validate();

        // Should pass - variable is resolved via environment
        assert_validation_passes!(result);
    }

    #[test]
    fn test_variable_resolution_fails_when_unresolved() {
        // This test now works! Variables that reference environment variables
        // are validated for resolution thanks to our implementation.

        let result = RunbookBuilder::new()
            .with_content(
                r#"
variable "api_key" {
    value = env.API_KEY
}
output "key" {
    value = variable.api_key
}
"#,
            )
            .with_environment("test", vec![]) // Empty environment - API_KEY not provided
            .set_current_environment("test")
            // No CLI input provided either
            .validate();

        // This now correctly fails!
        assert!(!result.success);
        assert_validation_error!(result, "API_KEY");
    }

    #[test]
    fn test_doctor_env_validation_with_builder() {
        // Test that variable resolution works with environment variables
        // Part 1: Variables with env references should fail validation when env var is missing
        let result = RunbookBuilder::new()
            .with_content(
                r#"
variable "api_key" {
    value = env.API_KEY
}

output "key" {
    value = variable.api_key
}
"#,
            )
            .with_environment(
                "production",
                vec![
                    ("OTHER_VAR", "value"), // API_KEY is missing!
                ],
            )
            .set_current_environment("production")
            .validate();

        // Should fail - API_KEY is missing
        assert!(!result.success);
        assert_validation_error!(result, "API_KEY");

        // Part 2: Variable can be resolved when env var is present
        let result2 = RunbookBuilder::new()
            .with_content(
                r#"
variable "api_key" {
    value = env.API_KEY
}

output "key" {
    value = variable.api_key
}
"#,
            )
            .with_environment("production", vec![("API_KEY", "prod-key-123")])
            .set_current_environment("production")
            .validate();

        assert_validation_passes!(result2);
    }

    // Test CLI input validation
    #[test]
    fn test_doctor_cli_input_validation_with_builder() {
        // Test that CLI inputs take precedence over environment variables
        let result = RunbookBuilder::new()
            .with_content(
                r#"
variable "api_url" {
    value = env.API_URL
}
variable "api_key" {
    value = env.API_KEY
}
output "url" {
    value = variable.api_url
}
output "key" {
    value = variable.api_key
}
"#,
            )
            .with_environment(
                "staging",
                vec![("API_URL", "https://staging.api.com"), ("API_KEY", "staging-key")],
            )
            .set_current_environment("staging")
            .with_cli_input("API_URL", "https://override.api.com")
            .validate();

        // Should pass - api_url from CLI, api_key from environment
        assert_validation_passes!(result);

        // Test missing required variable
        // This demonstrates the current limitation - validation passes even when
        // variables with env references can't be resolved
        let result2 = RunbookBuilder::new()
            .with_content(
                r#"
variable "required_key" {
    value = env.REQUIRED_KEY
}
output "key" {
    value = variable.required_key
}
"#,
            )
            .with_environment(
                "production",
                vec![
                    // REQUIRED_KEY not provided in environment
                ],
            )
            .set_current_environment("production")
            // And no CLI input provided
            .validate();

        // Should fail - REQUIRED_KEY is not provided
        assert!(!result2.success);
        assert_validation_error!(result2, "REQUIRED_KEY");
    }

    // Test forward references are allowed
    #[test]
    fn test_doctor_forward_references_with_builder() {
        let mut builder = RunbookBuilder::new()
            .addon("evm", vec![])
            .signer("deployer", "evm::private_key", vec![("private_key", "0x123")])
            // Action 1 references action2 (forward reference)
            .action("action1", "evm::send_eth")
            .input("from", "signer.deployer.address")
            .input("to", "action.action2.contract_address") // Forward ref
            .input("value", "1000")
            // Action 2 defined after action1
            .action("action2", "evm::deploy_contract")
            .input("contract", "\"Token.sol\"")
            .input("signer", "signer.deployer");

        let result = builder.validate();
        assert_validation_passes!(result);
    }

    // Test nested field access validation
    #[test]
    #[ignore = "Requires doctor validation to check nested field access - not yet implemented"]
    fn test_doctor_nested_field_access_with_builder() {
        // This test would require doctor validation mode which checks if
        // action outputs actually have the fields being accessed.
        // For example: action.deploy.contract_address is valid only if
        // the deploy action type actually outputs a contract_address field.
        // This validation is not yet available in manifest validation.

        // When implemented, this test would look like:
        /*
        let mut builder = RunbookBuilder::new()
            .addon("evm", vec![])
            .action("send", "evm::send_eth")
                .input("to", "0x123")
                .input("value", "1000")
            .output("invalid", "action.send.contract_address");  // send_eth doesn't have contract_address!

        let result = builder.validate_with_doctor();  // Need doctor mode
        assert_validation_error!(result, "contract_address");
        */
    }
}

#[cfg(test)]
mod doctor_hcl_vs_doctor_comparison {

    // This test demonstrates the difference between HCL-only and manifest validation
    #[test]
    fn test_validation_mode_differences() {
        use txtx_test_utils::builders::*;

        let content = r#"
variable "api_key" {
    value = env.API_KEY
}
output "key" {
    value = variable.api_key
}
"#;

        // Test 1: HCL-only validation (no environment set)
        let result1 = RunbookBuilder::new().with_content(content).validate(); // No environment set - uses HCL-only validation

        // HCL validation passes - it only checks syntax
        assert_validation_passes!(result1);

        // Test 2: Manifest validation without variable resolution
        // This demonstrates the current limitation - variables with env references pass validation
        let result2 = RunbookBuilder::new()
            .with_content(content)
            .with_environment(
                "production",
                vec![
                    // API_KEY is missing from environment
                ],
            )
            .set_current_environment("production") // This enables manifest validation
            .validate();

        // Should fail - API_KEY is not provided
        assert!(!result2.success);
        assert_validation_error!(result2, "API_KEY");

        // Test 3: Manifest validation with variable resolved via environment
        let result3 = RunbookBuilder::new()
            .with_content(content)
            .with_environment("production", vec![("API_KEY", "prod-key-123")])
            .set_current_environment("production")
            .validate();

        // Now it passes - variable is resolved
        assert_validation_passes!(result3);

        // Test 4: Manifest validation with variable resolved via CLI
        let result4 = RunbookBuilder::new()
            .with_content(content)
            .with_environment(
                "production",
                vec![
                    // API_KEY missing from environment
                ],
            )
            .set_current_environment("production")
            .with_cli_input("API_KEY", "cli-override")
            .validate();

        // Passes - variable resolved via CLI input
        assert_validation_passes!(result4);
    }
}

#[cfg(test)]
mod doctor_multi_file_tests {
    use super::*;

    // Test multi-file runbook validation
    #[test]
    fn test_doctor_multi_file_with_builder() {
        // Main runbook file
        let mut builder = RunbookBuilder::new()
            .with_content(
                r#"
                import "./flows.tx"
                
                addon "evm" {
                    rpc_api_url = "https://eth.example.com"
                }
                
                action "main" "evm::send_eth" {
                    to = "0x123"
                    value = "1000"
                }
            "#,
            )
            // Add imported file
            .with_file(
                "./flows.tx",
                r#"
                flow "deployment" {
                    variable "token_name" {
                        value = "MyToken"
                    }
                    
                    action "deploy" "evm::deploy_contract" {
                        contract = "Token.sol"
                        constructor_args = [flow.token_name]
                    }
                }
            "#,
            );

        // Doctor validation should handle multi-file imports
        let result = builder.validate();

        // This test would need actual multi-file support in the builder
        // For now, we're demonstrating the pattern
        println!(
            "Multi-file validation result: {}",
            if result.success { "✓ Success" } else { "✗ Failed" }
        );
    }
}

#[cfg(test)]
mod variable_resolution_truth_table {
    use super::*;

    // Test all 18 combinations of:
    // - Manifest: exists/doesn't exist (2 states)
    // - Global environment: none/defines var/doesn't define var (3 states)
    // - Specific environment: none/defines var/doesn't define var (3 states)
    //
    // Truth table:
    // Case | Manifest | Global Env      | Specific Env    | CLI Input | Expected Result
    // -----|----------|-----------------|-----------------|-----------|----------------
    //  1   | No       | None            | None            | No        | Pass (HCL-only)
    //  2   | No       | None            | None            | Yes       | Pass (HCL-only)
    //  3   | No       | Defines VAR     | None            | No        | Pass (HCL-only)
    //  4   | No       | Defines VAR     | None            | Yes       | Pass (HCL-only)
    //  5   | No       | Missing VAR     | None            | No        | Pass (HCL-only)
    //  6   | No       | Missing VAR     | None            | Yes       | Pass (HCL-only)
    //  7   | Yes      | None            | None            | No        | Pass*
    //  8   | Yes      | None            | None            | Yes       | Pass
    //  9   | Yes      | Defines VAR     | None            | No        | Pass
    // 10   | Yes      | Defines VAR     | None            | Yes       | Pass
    // 11   | Yes      | Missing VAR     | None            | No        | Pass*
    // 12   | Yes      | Missing VAR     | None            | Yes       | Pass
    // 13   | Yes      | None            | Defines VAR     | No        | Pass
    // 14   | Yes      | None            | Defines VAR     | Yes       | Pass
    // 15   | Yes      | None            | Missing VAR     | No        | Pass*
    // 16   | Yes      | None            | Missing VAR     | Yes       | Pass
    // 17   | Yes      | Missing VAR     | Defines VAR     | No        | Pass
    // 18   | Yes      | Missing VAR     | Missing VAR     | No        | Pass*
    //
    // * = Should fail when variable resolution validation is implemented

    const TEST_RUNBOOK: &str = r#"
variable "test_var" {
    value = env.TEST_VAR
}

output "result" {
    value = variable.test_var
}
"#;

    // Case 1: No manifest, no environments, no CLI input
    #[test]
    fn case_01_no_manifest_no_env_no_cli() {
        let result = RunbookBuilder::new().with_content(TEST_RUNBOOK).validate();

        // HCL-only validation passes
        assert_validation_passes!(result);
    }

    // Case 2: No manifest, no environments, with CLI input
    #[test]
    fn case_02_no_manifest_no_env_with_cli() {
        let result = RunbookBuilder::new()
            .with_content(TEST_RUNBOOK)
            .with_cli_input("TEST_VAR", "cli-value")
            .validate();

        // HCL-only validation passes
        assert_validation_passes!(result);
    }

    // Case 3: No manifest, global env defines var, no CLI input
    #[test]
    fn case_03_no_manifest_global_defines_no_cli() {
        // Cannot test this case - without manifest we can't set global env
        // This would require setting actual OS environment variables
    }

    // Case 7: Manifest exists, no environments, no CLI input
    #[test]
    fn case_07_manifest_no_env_no_cli() {
        let manifest = WorkspaceManifest::new("test".to_string());

        let result = RunbookBuilder::new()
            .with_content(TEST_RUNBOOK)
            .with_manifest(manifest)
            .validate_with_manifest();

        // Should fail - variable can't be resolved
        assert!(!result.success);
        assert_validation_error!(result, "TEST_VAR");
    }

    // Case 8: Manifest exists, no environments, with CLI input
    #[test]
    fn case_08_manifest_no_env_with_cli() {
        let manifest = WorkspaceManifest::new("test".to_string());

        let result = RunbookBuilder::new()
            .with_content(TEST_RUNBOOK)
            .with_manifest(manifest)
            .with_cli_input("TEST_VAR", "cli-value")
            .validate_with_manifest();

        // Should pass - resolved via CLI
        assert_validation_passes!(result);
    }

    // Case 9: Manifest with global env that defines var, no specific env, no CLI
    #[test]
    fn case_09_manifest_global_defines_no_specific_no_cli() {
        let manifest =
            create_test_manifest_with_env(vec![("global", vec![("TEST_VAR", "global-value")])]);

        let result = RunbookBuilder::new()
            .with_content(TEST_RUNBOOK)
            .with_manifest(manifest)
            .validate_with_manifest();

        // Should pass - resolved via global env
        assert_validation_passes!(result);
    }

    // Case 10: Manifest with global env that defines var, no specific env, with CLI
    #[test]
    fn case_10_manifest_global_defines_no_specific_with_cli() {
        let manifest =
            create_test_manifest_with_env(vec![("global", vec![("TEST_VAR", "global-value")])]);

        let result = RunbookBuilder::new()
            .with_content(TEST_RUNBOOK)
            .with_manifest(manifest)
            .with_cli_input("TEST_VAR", "cli-override")
            .validate_with_manifest();

        // Should pass - CLI overrides global env
        assert_validation_passes!(result);
    }

    // Case 11: Manifest with global env missing var, no specific env, no CLI
    #[test]
    fn case_11_manifest_global_missing_no_specific_no_cli() {
        let manifest =
            create_test_manifest_with_env(vec![("global", vec![("OTHER_VAR", "other-value")])]);

        let result = RunbookBuilder::new()
            .with_content(TEST_RUNBOOK)
            .with_manifest(manifest)
            .validate_with_manifest();

        // Currently passes but should fail - TEST_VAR not defined
        assert!(!result.success);
        assert_validation_error!(result, "TEST_VAR");
    }

    // Case 12: Manifest with global env missing var, no specific env, with CLI
    #[test]
    fn case_12_manifest_global_missing_no_specific_with_cli() {
        let manifest =
            create_test_manifest_with_env(vec![("global", vec![("OTHER_VAR", "other-value")])]);

        let result = RunbookBuilder::new()
            .with_content(TEST_RUNBOOK)
            .with_manifest(manifest)
            .with_cli_input("TEST_VAR", "cli-value")
            .validate_with_manifest();

        // Should pass - resolved via CLI
        assert_validation_passes!(result);
    }

    // Case 13: Manifest with specific env that defines var, no CLI
    #[test]
    fn case_13_manifest_no_global_specific_defines_no_cli() {
        let manifest =
            create_test_manifest_with_env(vec![("production", vec![("TEST_VAR", "prod-value")])]);

        let result = RunbookBuilder::new()
            .with_content(TEST_RUNBOOK)
            .with_manifest(manifest)
            .set_current_environment("production")
            .validate_with_manifest();

        // Should pass - resolved via specific env
        assert_validation_passes!(result);
    }

    // Case 14: Manifest with specific env that defines var, with CLI
    #[test]
    fn case_14_manifest_no_global_specific_defines_with_cli() {
        let manifest =
            create_test_manifest_with_env(vec![("production", vec![("TEST_VAR", "prod-value")])]);

        let result = RunbookBuilder::new()
            .with_content(TEST_RUNBOOK)
            .with_manifest(manifest)
            .set_current_environment("production")
            .with_cli_input("TEST_VAR", "cli-override")
            .validate_with_manifest();

        // Should pass - CLI overrides env
        assert_validation_passes!(result);
    }

    // Case 15: Manifest with specific env missing var, no CLI
    #[test]
    fn case_15_manifest_no_global_specific_missing_no_cli() {
        let manifest =
            create_test_manifest_with_env(vec![("production", vec![("OTHER_VAR", "other-value")])]);

        let result = RunbookBuilder::new()
            .with_content(TEST_RUNBOOK)
            .with_manifest(manifest)
            .set_current_environment("production")
            .validate_with_manifest();

        // Currently passes but should fail - TEST_VAR not defined
        assert!(!result.success);
        assert_validation_error!(result, "TEST_VAR");
    }

    // Case 16: Manifest with specific env missing var, with CLI
    #[test]
    fn case_16_manifest_no_global_specific_missing_with_cli() {
        let manifest =
            create_test_manifest_with_env(vec![("production", vec![("OTHER_VAR", "other-value")])]);

        let result = RunbookBuilder::new()
            .with_content(TEST_RUNBOOK)
            .with_manifest(manifest)
            .set_current_environment("production")
            .with_cli_input("TEST_VAR", "cli-value")
            .validate_with_manifest();

        // Should pass - resolved via CLI
        assert_validation_passes!(result);
    }

    // Case 17: Manifest with global missing but specific defines var
    #[test]
    fn case_17_manifest_global_missing_specific_defines_no_cli() {
        let manifest = create_test_manifest_with_env(vec![
            ("global", vec![("OTHER_VAR", "other-value")]),
            ("production", vec![("TEST_VAR", "prod-value")]),
        ]);

        let result = RunbookBuilder::new()
            .with_content(TEST_RUNBOOK)
            .with_manifest(manifest)
            .set_current_environment("production")
            .validate_with_manifest();

        // Should pass - specific env overrides global
        assert_validation_passes!(result);
    }

    // Case 18: Manifest with both envs missing var, no CLI
    #[test]
    fn case_18_manifest_both_missing_no_cli() {
        let manifest = create_test_manifest_with_env(vec![
            ("global", vec![("OTHER_VAR", "other-value")]),
            ("production", vec![("ANOTHER_VAR", "another-value")]),
        ]);

        let result = RunbookBuilder::new()
            .with_content(TEST_RUNBOOK)
            .with_manifest(manifest)
            .set_current_environment("production")
            .validate_with_manifest();

        // Currently passes but should fail - TEST_VAR not defined anywhere
        assert!(!result.success);
        assert_validation_error!(result, "TEST_VAR");
    }

    // Additional edge case tests

    #[test]
    fn test_env_precedence_specific_overrides_global() {
        // Test that specific environment overrides global
        let manifest = create_test_manifest_with_env(vec![
            ("global", vec![("TEST_VAR", "global-value")]),
            ("production", vec![("TEST_VAR", "prod-override")]),
        ]);

        let result = RunbookBuilder::new()
            .with_content(TEST_RUNBOOK)
            .with_manifest(manifest)
            .set_current_environment("production")
            .validate_with_manifest();

        // Should use production value
        assert_validation_passes!(result);
    }

    #[test]
    fn test_cli_precedence_overrides_all() {
        // Test that CLI input has highest precedence
        let manifest = create_test_manifest_with_env(vec![
            ("global", vec![("TEST_VAR", "global-value")]),
            ("production", vec![("TEST_VAR", "prod-value")]),
        ]);

        let result = RunbookBuilder::new()
            .with_content(TEST_RUNBOOK)
            .with_manifest(manifest)
            .set_current_environment("production")
            .with_cli_input("TEST_VAR", "cli-wins")
            .validate_with_manifest();

        // CLI should win
        assert_validation_passes!(result);
    }

    #[test]
    fn test_multiple_env_references() {
        // Test runbook with multiple environment variable references
        let content = r#"
variable "api_key" {
    value = env.API_KEY
}
variable "api_url" {
    value = env.API_URL
}
variable "timeout" {
    value = env.TIMEOUT
}

output "key" {
    value = variable.api_key
}
output "url" {
    value = variable.api_url
}
output "timeout" {
    value = variable.timeout
}
"#;

        // Case 1: All vars defined in environment
        let manifest1 = create_test_manifest_with_env(vec![(
            "test",
            vec![("API_KEY", "test-key"), ("API_URL", "https://test.api.com"), ("TIMEOUT", "30")],
        )]);

        let result1 = RunbookBuilder::new()
            .with_content(content)
            .with_manifest(manifest1)
            .set_current_environment("test")
            .validate_with_manifest();

        assert_validation_passes!(result1);

        // Case 2: Mix of env and CLI inputs
        let manifest2 = create_test_manifest_with_env(vec![(
            "test",
            vec![
                ("API_KEY", "test-key"),
                // API_URL missing
                ("TIMEOUT", "30"),
            ],
        )]);

        let result2 = RunbookBuilder::new()
            .with_content(content)
            .with_manifest(manifest2)
            .set_current_environment("test")
            .with_cli_input("API_URL", "https://cli.api.com")
            .validate_with_manifest();

        assert_validation_passes!(result2);

        // Case 3: Some vars missing - should fail
        let manifest3 = create_test_manifest_with_env(vec![(
            "test",
            vec![
                ("API_KEY", "test-key"),
                // API_URL and TIMEOUT missing
            ],
        )]);

        let result3 = RunbookBuilder::new()
            .with_content(content)
            .with_manifest(manifest3)
            .set_current_environment("test")
            .validate_with_manifest();

        // Should fail - API_URL and TIMEOUT are missing
        assert!(!result3.success);
        assert_validation_error!(result3, "API_URL");
    }
}
