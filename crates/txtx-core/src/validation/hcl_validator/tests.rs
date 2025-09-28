//! Tests for HCL validator, focusing on multi-file flow validation

use super::visitor::{BasicHclValidator, validate_with_hcl};
use crate::validation::types::ValidationResult;

#[cfg(test)]
mod flow_validation_tests {
    use super::*;

    #[test]
    fn test_flow_input_undefined_in_all_flows() {
        // Flow input referenced but not defined in ANY flow
        let combined_content = r#"
flow "super1" {
    api_url = "https://api1.com"
}

flow "super2" {
    api_url = "https://api2.com"
}

action "deploy" "evm::deploy_contract" {
    constructor_args = [flow.chain_id]
}
"#;

        let mut result = ValidationResult::new();

        // Validate combined content (simulates multi-file runbook)
        let _refs = validate_with_hcl(combined_content, &mut result, "runbook.tx").unwrap();

        // Should have error at reference site
        assert!(result.has_errors(), "Expected error for undefined flow input");

        let error = result.errors.iter()
            .find(|e| e.message.contains("chain_id"))
            .expect("Should have error mentioning chain_id");

        assert_eq!(error.file, "runbook.tx");

        // Should have related locations pointing to flows
        assert_eq!(error.related_locations.len(), 2,
            "Should show both flows missing the input");

        assert!(error.related_locations.iter()
            .any(|loc| loc.message.contains("super1") && loc.message.contains("chain_id")));
        assert!(error.related_locations.iter()
            .any(|loc| loc.message.contains("super2") && loc.message.contains("chain_id")));
    }

    #[test]
    fn test_flow_input_missing_in_some_flows() {
        // Some flows define the input, others don't
        let combined_content = r#"
flow "super1" {
    chain_id = "1"
}

flow "super2" {
    api_url = "https://api.com"
}

action "deploy" "evm::deploy_contract" {
    constructor_args = [flow.chain_id]
}
"#;

        let mut result = ValidationResult::new();
        let _refs = validate_with_hcl(combined_content, &mut result, "runbook.tx").unwrap();

        assert!(result.has_errors(), "Expected error for partially defined flow input");

        // Should have error at reference site mentioning incomplete definition
        let ref_errors: Vec<_> = result.errors.iter()
            .filter(|e| e.message.contains("chain_id") && e.message.contains("not defined in all flows"))
            .collect();
        assert!(!ref_errors.is_empty(), "Should have error at reference site");

        // Should have error at incomplete flow definition
        let flow_errors: Vec<_> = result.errors.iter()
            .filter(|e| e.message.contains("super2") && e.message.contains("missing input"))
            .collect();
        assert!(!flow_errors.is_empty(), "Should have error at incomplete flow definition");

        // Reference error should point to missing flow
        let ref_error = &ref_errors[0];
        assert!(ref_error.related_locations.iter()
            .any(|loc| loc.message.contains("super2")),
            "Reference error should point to flow missing the input");
    }

    #[test]
    fn test_flow_input_defined_in_all_flows() {
        // All flows properly define the referenced input - should pass
        let combined_content = r#"
flow "super1" {
    chain_id = "1"
}

flow "super2" {
    chain_id = "11155111"
}

action "deploy" "evm::deploy_contract" {
    constructor_args = [flow.chain_id]
}
"#;

        let mut result = ValidationResult::new();
        let _refs = validate_with_hcl(combined_content, &mut result, "runbook.tx").unwrap();

        assert!(!result.has_errors(), "Should not have errors when all flows define the input");
    }

    #[test]
    fn test_flow_input_in_variable() {
        // Flow input referenced in variable definition
        let combined_content = r#"
flow "prod" {
    env_name = "production"
}

variable "deployment_target" {
    value = flow.region
}
"#;

        let mut result = ValidationResult::new();
        let _refs = validate_with_hcl(combined_content, &mut result, "runbook.tx").unwrap();

        assert!(result.has_errors(), "Should have error for undefined flow input in variable");

        let error = result.errors.iter()
            .find(|e| e.message.contains("region"))
            .expect("Should have error mentioning region");

        assert!(error.related_locations.iter()
            .any(|loc| loc.message.contains("region")));
    }

    #[test]
    fn test_flow_input_in_output() {
        // Flow input referenced in output
        let combined_content = r#"
flow "default" {
    chain_id = "1"
}

output "contract_address" {
    value = action.deploy.address
    network = flow.network_name
}
"#;

        let mut result = ValidationResult::new();
        let _refs = validate_with_hcl(combined_content, &mut result, "runbook.tx").unwrap();

        assert!(result.has_errors(), "Should have error for undefined flow input in output");

        let error = result.errors.iter()
            .find(|e| e.message.contains("network_name"))
            .expect("Should have error mentioning network_name");

        assert_eq!(error.related_locations.len(), 1, "Should reference the one flow");
    }

    #[test]
    fn test_multiple_references_to_same_flow_input() {
        // Same flow input referenced multiple times
        let combined_content = r#"
flow "main" {
    api_key = "secret"
}

action "deploy" "evm::deploy_contract" {
    constructor_args = [flow.chain_id]
}

output "api_used" {
    value = input.api_url
    chain_id = flow.chain_id
}
"#;

        let mut result = ValidationResult::new();
        let _refs = validate_with_hcl(combined_content, &mut result, "runbook.tx").unwrap();

        assert!(result.has_errors(), "Should have errors for undefined flow input");

        // Should have errors at both reference sites
        let errors: Vec<_> = result.errors.iter()
            .filter(|e| e.message.contains("chain_id"))
            .collect();

        assert_eq!(errors.len(), 2, "Should have error at both reference sites");
    }

    #[test]
    fn test_no_flows_defined() {
        // Reference to flow.* when no flows exist at all
        let combined_content = r#"
action "deploy" "evm::deploy_contract" {
    constructor_args = [flow.chain_id]
}
"#;

        let mut result = ValidationResult::new();
        let _refs = validate_with_hcl(combined_content, &mut result, "runbook.tx").unwrap();

        // When no flows are defined, we don't generate errors
        // because the flow might be provided at runtime
        // The partition logic handles this: (defining.is_empty(), missing.is_empty()) = (true, true) → no errors
        assert!(!result.has_errors(), "Should not error when no flows are defined (might be runtime flow)");
    }
}
