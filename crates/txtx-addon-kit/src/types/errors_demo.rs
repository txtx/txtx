/// Demonstration of error-stack usage in txtx
/// This module shows how to use error-stack for rich error reporting

use error_stack::{Report, ResultExt};
use crate::types::diagnostics::Diagnostic;
use crate::types::errors::{TxtxError, ErrorAttachments, ErrorDocumentation, ErrorLocation, ActionContext, TypeMismatchInfo};
use crate::types::stores::ValueStore;
use crate::types::Did;
use crate::types::types::Value;

/// Example: Parse and validate a configuration value
pub fn parse_config_value(
    inputs: &ValueStore,
    key: &str,
    expected_type: &str,
) -> Result<Value, Report<TxtxError>> {
    // Get value from store
    let value = inputs.inputs.get_value(key)
        .ok_or_else(|| Report::new(TxtxError::MissingInput)
            .attach_printable(format!("Configuration key '{}' not found", key))
            .attach(ErrorDocumentation {
                help: format!("The '{}' field is required and must be a {}", key, expected_type),
                example: Some(format!("{} = \"example_value\"", key)),
                link: None,
            }))?;

    // Validate type
    match (expected_type, &value) {
        ("string", Value::String(_)) => Ok(value.clone()),
        ("integer", Value::Integer(_)) => Ok(value.clone()),
        ("boolean", Value::Bool(_)) => Ok(value.clone()),
        _ => {
            let actual_type = match &value {
                Value::String(_) => "string",
                Value::Integer(_) => "integer",
                Value::Bool(_) => "boolean",
                Value::Array(_) => "array",
                Value::Object(_) => "object",
                _ => "unknown",
            };
            
            Err(Report::new(TxtxError::TypeMismatch)
                .attach(TypeMismatchInfo {
                    field: key.to_string(),
                    expected: expected_type.to_string(),
                    actual: actual_type.to_string(),
                }))
                .attach_printable(format!(
                    "Configuration '{}' has wrong type", key
                ))
                .with_documentation("Check the type of the value in your configuration")
        }
    }
}

/// Example: Process an action with full error context
pub fn process_action_with_context(
    action_name: &str,
    namespace: &str,
    construct_id: &str,
    inputs: &ValueStore,
) -> Result<String, Report<TxtxError>> {
    // Step 1: Validate inputs
    let address = parse_config_value(inputs, "address", "string")
        .change_context(TxtxError::Validation)
        .attach_printable("Failed to validate action inputs")
        .with_action_context(action_name, namespace, construct_id)?;

    // Step 2: Parse address (simulate validation)
    let address_str = address.as_string()
        .ok_or_else(|| Report::new(TxtxError::TypeMismatch))?;
    
    if !address_str.starts_with("0x") || address_str.len() != 42 {
        return Err(Report::new(TxtxError::Validation))
            .attach_printable(format!("Invalid address format: {}", address_str))
            .with_location("config.tx", 10, 5)
            .with_documentation("Addresses must start with '0x' and be 42 characters long")
            .with_example("address = \"0x742d35Cc6634C0532925a3b844Bc9e7595f89590\"")
            .with_action_context(action_name, namespace, construct_id);
    }

    // Step 3: Execute (simulate)
    Ok(format!("Executed {} successfully", action_name))
}

/// Convert legacy Diagnostic to error-stack Report
pub fn migrate_diagnostic_error(diag: Diagnostic) -> Report<TxtxError> {
    diag.into()
}

/// Example of chaining errors with context
pub fn complex_operation_example() -> Result<(), Report<TxtxError>> {
    // Simulate multiple steps that can fail
    step_one()
        .change_context(TxtxError::Execution)
        .attach_printable("Step one failed in complex operation")?;
    
    step_two()
        .change_context(TxtxError::Execution)
        .attach_printable("Step two failed in complex operation")?;
    
    step_three()
        .change_context(TxtxError::Execution)
        .attach_printable("Step three failed in complex operation")?;
    
    Ok(())
}

fn step_one() -> Result<(), Report<TxtxError>> {
    Ok(())
}

fn step_two() -> Result<(), Report<TxtxError>> {
    // Simulate a network failure
    Err(Report::new(TxtxError::Network)
        .attach_printable("Connection timeout to RPC endpoint")
        .attach(ErrorLocation {
            file: "network_config.tx".to_string(),
            line: 15,
            column: 10,
        })
        .attach(ErrorDocumentation {
            help: "Check your network configuration and ensure the RPC endpoint is accessible".to_string(),
            example: Some("rpc_url = \"https://mainnet.infura.io/v3/YOUR_PROJECT_ID\"".to_string()),
            link: Some("https://docs.txtx.io/network-setup".to_string()),
        }))
}

fn step_three() -> Result<(), Report<TxtxError>> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config_value_success() {
        let mut inputs = ValueStore::new("test", &Did::zero());
        inputs.inputs.insert("name", Value::String("test".to_string()));
        
        let result = parse_config_value(&inputs, "name", "string");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_string().unwrap(), "test");
    }

    #[test]
    fn test_parse_config_value_missing() {
        let inputs = ValueStore::new("test", &Did::zero());
        
        let result = parse_config_value(&inputs, "missing_key", "string");
        assert!(result.is_err());
        
        let error = result.unwrap_err();
        let error_string = format!("{:?}", error);
        assert!(error_string.contains("Missing required input"));
        assert!(error_string.contains("missing_key"));
    }

    #[test]
    fn test_parse_config_value_wrong_type() {
        let mut inputs = ValueStore::new("test", &Did::zero());
        inputs.inputs.insert("count", Value::String("not_a_number".to_string()));
        
        let result = parse_config_value(&inputs, "count", "integer");
        assert!(result.is_err());
        
        let error = result.unwrap_err();
        let type_info = error.downcast_ref::<TypeMismatchInfo>().unwrap();
        assert_eq!(type_info.field, "count");
        assert_eq!(type_info.expected, "integer");
        assert_eq!(type_info.actual, "string");
    }

    #[test]
    fn test_process_action_with_context() {
        let mut inputs = ValueStore::new("test", &Did::zero());
        inputs.inputs.insert("address", 
            Value::String("0x742d35Cc6634C0532925a3b844Bc9e7595f89590".to_string()));
        
        let result = process_action_with_context(
            "deploy_contract",
            "evm",
            "construct_123",
            &inputs,
        );
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Executed deploy_contract successfully"));
    }

    #[test]
    fn test_process_action_invalid_address() {
        let mut inputs = ValueStore::new("test", &Did::zero());
        inputs.inputs.insert("address", Value::String("invalid".to_string()));
        
        let result = process_action_with_context(
            "deploy_contract",
            "evm",
            "construct_123",
            &inputs,
        );
        assert!(result.is_err());
        
        let error = result.unwrap_err();
        let error_string = format!("{:?}", error);
        assert!(error_string.contains("Invalid address format"));
        
        // Check attachments
        let location = error.downcast_ref::<ErrorLocation>().unwrap();
        assert_eq!(location.file, "config.tx");
        
        let action_ctx = error.downcast_ref::<ActionContext>().unwrap();
        assert_eq!(action_ctx.action_name, "deploy_contract");
        assert_eq!(action_ctx.namespace, "evm");
    }

    #[test]
    fn test_complex_operation_with_network_failure() {
        let result = complex_operation_example();
        assert!(result.is_err());
        
        let error = result.unwrap_err();
        let error_string = format!("{:?}", error);
        
        // Should contain both the network error and the execution context
        assert!(error_string.contains("Connection timeout"));
        assert!(error_string.contains("Step two failed"));
        
        // Check detailed attachments
        let location = error.downcast_ref::<ErrorLocation>().unwrap();
        assert_eq!(location.file, "network_config.tx");
        assert_eq!(location.line, 15);
        
        let docs = error.downcast_ref::<ErrorDocumentation>().unwrap();
        assert!(docs.help.contains("Check your network configuration"));
        assert!(docs.example.is_some());
        assert!(docs.link.is_some());
    }
}