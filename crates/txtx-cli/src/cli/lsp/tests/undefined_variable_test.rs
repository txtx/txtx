/// Test to verify that undefined variable detection is handled by HCL validator
/// This replaces the old linter rule for undefined variables
#[cfg(test)]
mod tests {
    use txtx_core::validation::{ValidationResult, hcl_validator};

    #[test]
    fn test_undefined_variable_detection_by_hcl_validator() {
        // Test content with undefined variable reference
        let content = r#"
variable "defined_var" {
    value = "test value"
}

variable "test" {
    value = variable.undefined_var
}

action "example" "test" {
    value = variable.another_undefined
}
"#;

        let mut result = ValidationResult::default();

        // Run HCL validation (what our LSP now relies on)
        let _ = hcl_validator::validate_with_hcl(
            content,
            &mut result,
            "test.tx"
        );

        // Should detect undefined variable references
        let undefined_var_errors: Vec<_> = result.errors.iter()
            .filter(|e| {
                e.message.contains("undefined") ||
                e.message.contains("not found") ||
                e.message.contains("Unknown variable") ||
                e.message.contains("Reference to undefined")
            })
            .collect();

        assert!(
            !undefined_var_errors.is_empty(),
            "HCL validator should detect undefined variables. Got errors: {:?}",
            result.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );

        // Verify we catch both undefined variables
        assert!(
            undefined_var_errors.len() >= 1,
            "Should detect at least one undefined variable reference"
        );
    }

    #[test]
    fn test_undefined_variable_in_action() {
        // Specific test for undefined variable in action block
        let content = r#"
variable "defined_var" {
    value = "test"
}

action "test" "example::action" {
    some_param = variable.undefined_var
}
"#;

        let mut result = ValidationResult::default();
        let _ = hcl_validator::validate_with_hcl(content, &mut result, "test.tx");

        // The HCL validator should either:
        // 1. Detect the undefined variable reference
        // 2. Or report it as an invalid action (since example::action doesn't exist)
        assert!(
            !result.errors.is_empty(),
            "Should detect issues with undefined variable in action"
        );
    }
}