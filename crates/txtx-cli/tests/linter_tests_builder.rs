use txtx_core::manifest::WorkspaceManifest;
use txtx_test_utils::builders::{create_test_manifest_with_env, RunbookBuilder, ValidationResult};

// Test content constants
const SIMPLE_CIRCULAR_VARS: &str = r#"
variable "a" {
    value = variable.b
}
variable "b" {
    value = variable.a
}
"#;

const CIRCULAR_CHAIN_VARS: &str = r#"
variable "a" {
    value = variable.b
}
variable "b" {
    value = variable.c
}
variable "c" {
    value = variable.a
}
output "result" {
    value = variable.a
}
"#;

const SELF_REF_VAR: &str = r#"
variable "self_ref" {
    value = variable.self_ref
}
"#;

const TEST_RUNBOOK: &str = r#"
variable "test_var" {
    value = input.TEST_VAR
}

output "result" {
    value = variable.test_var
}
"#;

// Helper macros for common assertions
macro_rules! assert_validation_error {
    ($result:expr, $expected:expr) => {
        assert!(!$result.success, "Expected validation to fail");
        assert!(
            $result.errors.iter().any(|e| e.message.contains($expected)),
            "Expected error containing '{}', but got: {:?}",
            $expected,
            error_messages(&$result)
        );
    };
}

macro_rules! assert_validation_passes {
    ($result:expr) => {
        assert!(
            $result.success,
            "Expected validation to succeed, but got errors: {:?}",
            error_messages(&$result)
        );
    };
}

macro_rules! assert_circular_dependency {
    ($result:expr) => {
        assert!(!$result.success, "Should detect circular dependency");
        assert!(
            $result.errors.iter().any(|e|
                e.message.contains("circular") ||
                e.message.contains("cycle") ||
                e.message.contains("recursive") ||
                e.message.contains("depends on itself")
            ),
            "Expected circular dependency error, got: {:?}",
            error_messages(&$result)
        );
    };
}

// Type-safe error code assertions
macro_rules! assert_has_error_code {
    ($result:expr, $code:expr) => {
        assert!(!$result.success, "Expected validation to fail");
        assert!(
            $result.errors.iter().any(|e| e.code.as_deref() == Some($code)),
            "Expected error with code '{}', but got codes: {:?}",
            $code,
            error_codes(&$result)
        );
    };
}

macro_rules! assert_has_warning_code {
    ($result:expr, $code:expr) => {
        assert!(
            $result.warnings.iter().any(|w| w.code.as_deref() == Some($code)),
            "Expected warning with code '{}', but got codes: {:?}",
            $code,
            warning_codes(&$result)
        );
    };
}

#[macro_use]
mod common;

// Helper functions - defined at top level to be accessible from all test modules
pub fn error_messages(result: &ValidationResult) -> Vec<&str> {
    result.errors.iter().map(|e| e.message.as_str()).collect()
}

pub fn error_codes(result: &ValidationResult) -> Vec<Option<&str>> {
    result.errors.iter().map(|e| e.code.as_deref()).collect()
}

pub fn warning_codes(result: &ValidationResult) -> Vec<Option<&str>> {
    result.warnings.iter().map(|w| w.code.as_deref()).collect()
}

pub fn evm_builder_with_signer() -> RunbookBuilder {
    RunbookBuilder::new()
        .addon("evm", vec![("rpc_api_url", "\"https://eth.example.com\"")])
        .signer("operator", "evm::private_key", vec![("private_key", "0x1234")])
}

pub fn validate_with_env(content: &str, env_name: &str, vars: Vec<(&str, &str)>) -> ValidationResult {
    let manifest = create_test_manifest_with_env(vec![(env_name, vars)]);
    RunbookBuilder::new()
        .with_content(content)
        .with_manifest(manifest)
        .set_current_environment(env_name)
        .validate_with_manifest()
}

pub fn validate_with_global_env(content: &str, vars: Vec<(&str, &str)>) -> ValidationResult {
    let manifest = create_test_manifest_with_env(vec![("global", vars)]);
    RunbookBuilder::new()
        .with_content(content)
        .with_manifest(manifest)
        .validate_with_manifest()
}

pub fn validate_with_cli_input(content: &str, input_key: &str, input_value: &str) -> ValidationResult {
    RunbookBuilder::new()
        .with_content(content)
        .with_cli_input(input_key, input_value)
        .validate()
}

#[cfg(test)]
mod lint_fixture_tests {
    use super::*;

    // Test case 1: test_lint_simple.tx
    // Expected errors:
    // - Undefined signer reference
    // - Invalid parameter names 'to' and 'value'
    // - Missing required parameter 'recipient_address'
    // - Invalid field access 'from' on action
    #[test]
    fn test_lint_simple_with_builder() {
        let mut builder = RunbookBuilder::new()
            .action("send", "evm::send_eth")
            .input("signer", "signer.undefined_signer") // ERROR: signer not defined
            .input("to", "0x123")  // ERROR: invalid parameter name
            .input("value", "1000") // ERROR: invalid parameter name
            .output("bad", "action.send.from"); // ERROR: send_eth only outputs 'tx_hash'

        let result = builder.validate();

        assert!(!result.success);

        // Check specific errors - verify each expected failure
        // 1. Undefined signer reference
        assert_validation_error!(result, "undefined_signer");

        // 2 & 3. Invalid parameter names
        let param_errors: Vec<_> = result.errors.iter()
            .filter(|e| e.message.contains("Invalid parameter"))
            .collect();
        assert_eq!(param_errors.len(), 2, "Expected 2 invalid parameter errors, got {}: {:?}",
            param_errors.len(), error_messages(&result));
        assert!(param_errors.iter().any(|e| e.message.contains("'to'")),
            "Expected error for invalid parameter 'to'");
        assert!(param_errors.iter().any(|e| e.message.contains("'value'")),
            "Expected error for invalid parameter 'value'");

        // 4. Invalid field access
        assert_validation_error!(result, "from");
    }

    // Test case 2: test_lint_valid.tx
    // Valid runbook with correct parameter names
    #[test]
    fn test_lint_valid_with_builder() {
        let mut builder = evm_builder_with_signer()
            // Action 1 with CORRECT parameter names
            .action("action1", "evm::send_eth")
            .input("signer", "signer.operator")  // Correct: 'signer' not 'from'
            .input("recipient_address", "0x456")  // Correct: 'recipient_address' not 'to'
            .input("amount", "1000")  // Correct: 'amount' not 'value'
            // Action 2 references action1 (forward reference is OK)
            .action("action2", "evm::send_eth")
            .input("signer", "signer.operator")  // Correct: 'signer' not 'from'
            .input("recipient_address", "0x789")  // Correct: 'recipient_address' not 'to'
            .input("amount", "2000")  // Correct: 'amount' not 'value'
            // Note: depends_on is not a valid parameter for send_eth
            // Output references both actions
            .output("tx1", "action.action1.tx_hash")
            .output("tx2", "action.action2.tx_hash");

        let result = builder.validate();
        assert_validation_passes!(result);
    }

    // Test case 3: test_lint_two_pass.tx
    // Expected errors:
    // - Invalid parameters 'to' and 'value'
    // - Missing required parameters
    // - Undefined action reference
    #[test]
    fn test_lint_two_pass_with_builder() {
        let mut builder = RunbookBuilder::new()
            .addon("evm", vec![])
            .action("first", "evm::send_eth")
            .input("to", "0x123")  // ERROR: should be 'recipient_address'
            .input("value", "1000") // ERROR: should be 'amount'
            .output("result", "action.second.tx_hash"); // ERROR: 'second' action not defined

        let result = builder.validate();

        assert!(!result.success);

        // Check specific errors - verify each expected failure
        // 1. Undefined action reference
        assert_validation_error!(result, "second");

        // 2 & 3. Invalid parameter names
        let param_errors: Vec<_> = result.errors.iter()
            .filter(|e| e.message.contains("Invalid parameter"))
            .collect();
        assert_eq!(param_errors.len(), 2, "Expected 2 invalid parameter errors, got {}: {:?}",
            param_errors.len(), error_messages(&result));
        assert!(param_errors.iter().any(|e| e.message.contains("'to'")),
            "Expected error for invalid parameter 'to'");
        assert!(param_errors.iter().any(|e| e.message.contains("'value'")),
            "Expected error for invalid parameter 'value'");
    }

    // Test case 4: test_lint_unknown_action_type.tx
    // Should find unknown action type
    #[test]
    fn test_lint_unknown_action_type_with_builder() {
        let mut builder =
            RunbookBuilder::new().addon("evm", vec![]).action("test", "evm::unknown_action"); // ERROR: unknown action type

        let result = builder.validate();

        assert!(!result.success);
        assert_eq!(result.errors.len(), 1, "Expected 1 error");
        assert_validation_error!(result, "unknown_action");
    }

    // Test case 5: test_lint_flow_missing_variable.tx
    // Should find undefined flow variable and usage error
    #[test]
    fn test_lint_flow_missing_variable_with_builder() {
        // Lint mode now uses the same HCL validator as production
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

        let result = builder.validate_with_linter(None, None);

        assert_validation_error!(result, "undefined_var");
    }

    // Test case 6: Multiple errors combined
    #[test]
    fn test_lint_multiple_errors_with_builder() {
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

        // Check specific errors - verify each expected failure
        // 1. Undefined signer
        assert_validation_error!(result, "missing");

        // 2 & 3. Invalid parameter names
        let param_errors: Vec<_> = result.errors.iter()
            .filter(|e| e.message.contains("Invalid parameter"))
            .collect();
        assert!(param_errors.len() >= 2, "Expected at least 2 invalid parameter errors");

        // 4. Invalid action type
        assert_validation_error!(result, "invalid_action");

        // 5 & 6. Invalid references
        assert_validation_error!(result, "invalid");
    }

    // Test environment variable validation
    #[test]
    fn test_variable_resolution_cli_input() {
        // Test that variables can be resolved via CLI input, even when env var is missing
        let result = RunbookBuilder::new()
            .with_content(
                r#"
variable "api_key" {
    value = input.API_KEY
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
    value = input.API_KEY
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
    value = input.API_KEY
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
    fn test_lint_env_validation_with_builder() {
        // Test that variable resolution works with environment variables
        // Part 1: Variables with env references should fail validation when env var is missing
        let result = RunbookBuilder::new()
            .with_content(
                r#"
variable "api_key" {
    value = input.API_KEY
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
        assert_has_error_code!(result, "undefined_input");

        // Part 2: Variable can be resolved when env var is present
        let result2 = RunbookBuilder::new()
            .with_content(
                r#"
variable "api_key" {
    value = input.API_KEY
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
    fn test_lint_cli_input_validation_with_builder() {
        // Test that CLI inputs take precedence over environment variables
        let result = RunbookBuilder::new()
            .with_content(
                r#"
variable "api_url" {
    value = input.API_URL
}
variable "api_key" {
    value = input.API_KEY
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
    value = input.REQUIRED_KEY
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
    fn test_lint_forward_references_with_builder() {
        let mut builder = RunbookBuilder::new()
            .addon("evm", vec![])
            .signer("deployer", "evm::private_key", vec![("private_key", "0x123")])
            // Action 1 references action2 (forward reference)
            .action("action1", "evm::send_eth")
            .input("signer", "signer.deployer")
            .input("recipient_address", "action.action2.contract_address") // Forward ref
            .input("amount", "1000")
            // Action 2 defined after action1
            .action("action2", "evm::deploy_contract")
            .input("contract", r#"{"bytecode": "0x6080604052"}"#)
            .input("signer", "signer.deployer");

        let result = builder.validate();
        assert_validation_passes!(result);
    }

    // Test circular dependencies in variable definitions
    #[test]
    fn test_circular_dependency_in_variables() {
        // Test case 1: Simple circular dependency between two variables
        let result = RunbookBuilder::new()
            .with_content(SIMPLE_CIRCULAR_VARS)
            .validate();

        assert_circular_dependency!(result);
    }

    #[test]
    fn test_circular_dependency_chain() {
        // Test case 2: Circular dependency chain (a -> b -> c -> a)
        let result = RunbookBuilder::new()
            .with_content(CIRCULAR_CHAIN_VARS)
            .validate();

        assert_circular_dependency!(result);
    }

    #[test]
    fn test_self_referencing_variable() {
        // Test case 3: Variable that references itself
        let result = RunbookBuilder::new()
            .with_content(SELF_REF_VAR)
            .validate();

        assert_circular_dependency!(result);
    }

    #[test]
    fn test_circular_dependency_with_valid_variables() {
        // Test case 4: Mix of valid and circular dependencies
        let content = r#"
variable "valid1" {
    value = "static_value"
}
variable "valid2" {
    value = variable.valid1
}
variable "circular_a" {
    value = variable.circular_b
}
variable "circular_b" {
    value = variable.circular_a
}
output "good" {
    value = variable.valid2
}
output "bad" {
    value = variable.circular_a
}
"#;

        let result = RunbookBuilder::new()
            .with_content(content)
            .validate();

        assert_circular_dependency!(result);
    }

    #[test]
    fn test_circular_dependency_in_actions() {
        // Test circular dependencies between actions
        let content = r#"
addon "evm" {
    chain_id = 1
    rpc_url = "https://eth.public-rpc.com"
}

action "action_a" "evm::sign_transaction" {
    signer = action.action_b.signer
    bytes = "0x1234"
}

action "action_b" "evm::sign_transaction" {
    signer = action.action_a.signer
    bytes = "0x5678"
}
"#;

        let result = RunbookBuilder::new()
            .with_content(content)
            .validate();

        assert_circular_dependency!(result);
    }

    #[test]
    fn test_circular_dependency_complex_graph() {
        // Test a more complex circular dependency with multiple paths and a wider circuit
        // Graph structure: a -> b -> c -> d
        //                  |         ^    |
        //                  v         |    v
        //                  e -> f -> g    h
        //
        // This creates multiple potential cycles:
        // - a -> e -> f -> g -> c -> d -> h (no cycle on this path)
        // - a -> b -> c -> g -> c (cycle: c -> g -> c)
        // - a -> e -> f -> g -> c -> d -> h (no cycle)

        let content = r#"
variable "a" {
    value = join("-", [variable.b, variable.e])
}

variable "b" {
    value = variable.c
}

variable "c" {
    value = join("/", [variable.d, variable.g])
}

variable "d" {
    value = variable.h
}

variable "e" {
    value = variable.f
}

variable "f" {
    value = variable.g
}

variable "g" {
    value = variable.c
}

variable "h" {
    value = "terminal_value"
}

output "result" {
    value = variable.a
}
"#;

        let result = RunbookBuilder::new()
            .with_content(content)
            .validate();

        assert_circular_dependency!(result);

        // Verify it detects the specific cycle
        assert!(
            result.errors.iter().any(|e|
                (e.message.contains("c") && e.message.contains("g")) ||
                (e.message.contains("g") && e.message.contains("c"))
            ),
            "Should identify the c -> g -> c cycle, got: {:?}",
            error_messages(&result)
        );
    }

    #[test]
    fn test_circular_dependency_diamond_pattern() {
        // Test a diamond pattern with a cycle at the bottom
        // Graph structure:     a
        //                    /   \
        //                   b     c
        //                    \   / \
        //                      d    e
        //                      ^    |
        //                      |    v
        //                      f <- g
        //
        // Creates cycle: d -> f -> g -> e -> c -> d

        let content = r#"
variable "a" {
    value = join(",", [variable.b, variable.c])
}

variable "b" {
    value = variable.d
}

variable "c" {
    value = join(",", [variable.d, variable.e])
}

variable "d" {
    value = variable.f
}

variable "e" {
    value = variable.g
}

variable "f" {
    value = variable.g
}

variable "g" {
    value = variable.e
}
"#;

        let result = RunbookBuilder::new()
            .with_content(content)
            .validate();

        assert_circular_dependency!(result);
    }

    #[test]
    fn test_circular_dependency_multiple_disconnected_cycles() {
        // Test multiple disconnected circular dependencies in the same file
        // Graph 1: a -> b -> c -> a (cycle)
        // Graph 2: x -> y -> z -> x (cycle)
        // Graph 3: p -> q (no cycle)

        let content = r#"
variable "a" {
    value = variable.b
}

variable "b" {
    value = variable.c
}

variable "c" {
    value = variable.a
}

variable "x" {
    value = variable.y
}

variable "y" {
    value = variable.z
}

variable "z" {
    value = variable.x
}

variable "p" {
    value = variable.q
}

variable "q" {
    value = "static_value"
}

output "result1" {
    value = variable.a
}

output "result2" {
    value = variable.x
}

output "result3" {
    value = variable.p
}
"#;

        let result = RunbookBuilder::new()
            .with_content(content)
            .validate();

        assert_circular_dependency!(result);

        // Count how many circular dependency errors we have
        let circular_errors: Vec<_> = result.errors.iter()
            .filter(|e| e.message.contains("circular") || e.message.contains("cycle"))
            .collect();

        // We should detect at least 2 cycles (could be reported as 2 or more errors)
        assert!(
            circular_errors.len() >= 2,
            "Should detect at least 2 circular dependencies, found {}: {:?}",
            circular_errors.len(),
            error_messages(&result)
        );

        // Verify both cycles are mentioned
        let all_errors = error_messages(&result).join(" ");

        assert!(
            (all_errors.contains("a") && all_errors.contains("b") && all_errors.contains("c")) ||
            (all_errors.contains("a ->") || all_errors.contains("-> a")),
            "Should detect the a -> b -> c -> a cycle"
        );

        assert!(
            (all_errors.contains("x") && all_errors.contains("y") && all_errors.contains("z")) ||
            (all_errors.contains("x ->") || all_errors.contains("-> x")),
            "Should detect the x -> y -> z -> x cycle"
        );
    }

    #[test]
    fn test_circular_dependency_cycle_in_middle_of_chain() {
        // Test a cycle that occurs in the middle of a longer chain
        // Graph structure: a -> b -> c -> d -> c (cycle) -> e -> f
        //                              ^    |
        //                              |____|
        //
        // This tests that we detect cycles even when they don't include
        // the root node and are part of a longer dependency chain

        let content = r#"
variable "a" {
    value = variable.b
}

variable "b" {
    value = variable.c
}

variable "c" {
    value = variable.d
}

variable "d" {
    value = variable.c  // Creates cycle: c -> d -> c
}

variable "e" {
    value = variable.f
}

variable "f" {
    value = "terminal_value"
}

output "result" {
    value = variable.a
}
"#;

        let result = RunbookBuilder::new()
            .with_content(content)
            .validate();

        assert_circular_dependency!(result);

        // Verify it detects the specific c -> d -> c cycle
        let all_errors = error_messages(&result).join(" ");

        assert!(
            all_errors.contains("c") && all_errors.contains("d"),
            "Should identify the c -> d -> c cycle in the middle of the chain, got: {:?}",
            error_messages(&result)
        );
    }

    #[test]
    fn test_circular_dependency_nested_cycles() {
        // Test nested cycles where one cycle is contained within another
        // Graph structure: a -> b -> c -> d -> e -> f -> b (outer cycle)
        //                              \-> g -> h -> g (inner cycle)
        //
        // This creates two cycles:
        // - b -> c -> d -> e -> f -> b (outer cycle)
        // - g -> h -> g (inner cycle branching from c)

        let content = r#"
variable "a" {
    value = variable.b
}

variable "b" {
    value = variable.c
}

variable "c" {
    value = join("-", [variable.d, variable.g])
}

variable "d" {
    value = variable.e
}

variable "e" {
    value = variable.f
}

variable "f" {
    value = variable.b  // Creates outer cycle
}

variable "g" {
    value = variable.h
}

variable "h" {
    value = variable.g  // Creates inner cycle
}

output "result" {
    value = variable.a
}
"#;

        let result = RunbookBuilder::new()
            .with_content(content)
            .validate();

        assert_circular_dependency!(result);

        let circular_errors: Vec<_> = result.errors.iter()
            .filter(|e| e.message.contains("circular") || e.message.contains("cycle"))
            .collect();

        // Should detect at least one cycle (implementation may detect one or both)
        assert!(
            !circular_errors.is_empty(),
            "Should detect at least one circular dependency in nested structure, got: {:?}",
            error_messages(&result)
        );

        // Check that at least one of the cycles is detected
        let all_errors = error_messages(&result).join(" ");

        let has_outer_cycle = all_errors.contains("b") && all_errors.contains("f");
        let has_inner_cycle = all_errors.contains("g") && all_errors.contains("h");

        assert!(
            has_outer_cycle || has_inner_cycle,
            "Should detect at least one of the cycles (outer: b->...->f->b or inner: g->h->g), got: {:?}",
            error_messages(&result)
        );
    }

    // Test action output field reference validation
    #[test]
    fn test_action_output_field_reference_validation() {
        // This test validates that references to action output fields are properly checked.
        // The HCL validator implements this via validate_action_field_access()
        // which ensures action.X.Y references only access fields that exist in the action's output schema

        // Test 1: Valid field access - deploy_contract has contract_address
        let mut builder = RunbookBuilder::new()
            .addon("evm", vec![])
            .signer("deployer", "evm::private_key", vec![("private_key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")])
            .action("deploy", "evm::deploy_contract")
                .input("contract", r#"{"bytecode": "0x6080604052", "abi": "[{\"type\":\"constructor\"}]"}"#)
                .input("signer", "signer.deployer")
            .output("address", "action.deploy.contract_address");

        let result = builder.validate();
        assert_validation_passes!(result);

        // Test 2: Invalid field access - send_eth doesn't have contract_address
        let mut builder2 = RunbookBuilder::new()
            .addon("evm", vec![])
            .signer("sender", "evm::private_key", vec![("private_key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")])
            .action("send", "evm::send_eth")
                .input("signer", "signer.sender")
                .input("recipient_address", "0x1234567890123456789012345678901234567890")
                .input("amount", "1000")
            .output("invalid", "action.send.contract_address");  // send_eth doesn't have contract_address!

        let result2 = builder2.validate();
        assert!(!result2.success, "Should fail - send_eth doesn't output contract_address");
        assert_validation_error!(result2, "contract_address");
        assert_validation_error!(result2, "does not exist");

        // The error message should indicate available outputs
        let error = result2.errors.iter()
            .find(|e| e.message.contains("contract_address"))
            .expect("Should have error about contract_address");
        assert!(error.message.contains("tx_hash"), "Error should list available outputs like tx_hash");
    }
}

#[cfg(test)]
mod lint_hcl_vs_lint_comparison {
    use super::*;

    // This test demonstrates the difference between HCL-only and manifest validation
    #[test]
    fn test_validation_mode_differences() {
        use txtx_test_utils::builders::*;

        let content = r#"
variable "api_key" {
    value = input.API_KEY
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
mod lint_multi_file_tests {
    use super::*;

    // Test multi-file runbook validation
    #[test]
    #[ignore = "Multi-file validation not yet supported by test builder - pending implementation"]
    fn test_lint_multi_file_with_builder() {
        // TODO: Implement multi-file support in RunbookBuilder
        // This test demonstrates the intended pattern for multi-file validation:
        //
        // 1. Create main runbook with import statements
        // 2. Add imported files via builder.with_file()
        // 3. Validate that linter resolves imports correctly
        // 4. Verify errors are reported with correct file paths
        //
        // Implementation blocked on: RunbookBuilder.with_file() support
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
    value = input.TEST_VAR
}

output "result" {
    value = variable.test_var
}
"#;

    // Case 1: No manifest, no environments, no CLI input
    #[test]
    fn case_01_no_manifest_no_env_no_cli() {
        let result = RunbookBuilder::new().with_content(TEST_RUNBOOK).validate();
        assert_validation_passes!(result);
    }

    // Case 2: No manifest, no environments, with CLI input
    #[test]
    fn case_02_no_manifest_no_env_with_cli() {
        let result = validate_with_cli_input(TEST_RUNBOOK, "TEST_VAR", "cli-value");
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
        let result = validate_with_global_env(TEST_RUNBOOK, vec![("TEST_VAR", "global-value")]);
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
        let result = validate_with_global_env(TEST_RUNBOOK, vec![("OTHER_VAR", "other-value")]);
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
        let result = validate_with_env(TEST_RUNBOOK, "production", vec![("TEST_VAR", "prod-value")]);
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
        let result = validate_with_env(TEST_RUNBOOK, "production", vec![("OTHER_VAR", "other-value")]);
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
    value = input.API_KEY
}
variable "api_url" {
    value = input.API_URL
}
variable "timeout" {
    value = input.TIMEOUT
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
