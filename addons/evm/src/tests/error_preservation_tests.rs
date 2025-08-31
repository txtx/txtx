//! Tests for error-stack preservation in Diagnostic conversion

use error_stack::Report;
use txtx_addon_kit::types::diagnostics::Diagnostic;
use crate::errors::{EvmError, TransactionError, EvmErrorReport};

#[test]
fn test_error_preservation() {
    // Create a Report<EvmError> with some context
    let error = Report::new(EvmError::Transaction(TransactionError::InsufficientFunds {
        required: 100,
        available: 50,
    }))
    .attach_printable("Transaction failed due to insufficient funds")
    .attach_printable("Please ensure your account has enough balance");
    
    // Convert to EvmErrorReport and then to Diagnostic
    let wrapper = EvmErrorReport::from(error);
    let diagnostic = Diagnostic::from(wrapper);
    
    // Verify the diagnostic has the expected properties
    assert!(diagnostic.message.contains("Insufficient funds"));
    assert!(diagnostic.documentation.is_some());
    assert!(diagnostic.source_error.is_some());
    
    // Verify we can downcast back to Report<EvmError>
    if let Some(source) = diagnostic.source_error {
        let recovered = source.downcast_ref::<Report<EvmError>>();
        assert!(recovered.is_some(), "Should be able to downcast to Report<EvmError>");
        
        if let Some(report) = recovered {
            // Verify the report still has the correct error type
            let current = report.current_context();
            match current {
                EvmError::Transaction(TransactionError::InsufficientFunds { required, available }) => {
                    assert_eq!(*required, 100);
                    assert_eq!(*available, 50);
                }
                _ => panic!("Unexpected error type"),
            }
        }
    }
}

#[test]
fn test_error_chain_preservation() {
    // Create a complex error chain
    let error = Report::new(EvmError::Transaction(TransactionError::GasEstimationFailed))
    .attach_printable("Failed to estimate gas for transaction")
    .attach_printable("Contract execution would exceed block gas limit");
    
    // Convert through the pipeline
    let wrapper = EvmErrorReport::from(error);
    let diagnostic = Diagnostic::from(wrapper);
    
    // Verify documentation contains the full error chain
    assert!(diagnostic.documentation.is_some());
    let docs = diagnostic.documentation.as_ref().unwrap();
    assert!(docs.contains("Full error context"));
    assert!(docs.contains("Failed to estimate gas") || docs.contains("gas estimation"));
}

#[test]
fn test_multiple_error_types() {
    use crate::errors::{RpcError, ContractError, CodecError};
    use alloy::primitives::Address;
    use std::str::FromStr;
    
    // Test with different error types
    let test_address = Address::from_str("0x0000000000000000000000000000000000000000").unwrap();
    let errors = vec![
        EvmError::Rpc(RpcError::ConnectionFailed("http://localhost:8545".to_string())),
        EvmError::Contract(ContractError::NotDeployed(test_address)),
        EvmError::Codec(CodecError::InvalidHex("not hex".to_string())),
    ];
    
    for error in errors {
        let report = Report::new(error.clone());
        let wrapper = EvmErrorReport::from(report);
        let diagnostic = Diagnostic::from(wrapper);
        
        // All should preserve their source errors
        assert!(diagnostic.source_error.is_some());
        
        // All should be recoverable
        if let Some(source) = diagnostic.source_error {
            let recovered = source.downcast_ref::<Report<EvmError>>();
            assert!(recovered.is_some());
            
            if let Some(report) = recovered {
                // Verify we get back the same error variant
                let current = report.current_context();
                match (current, &error) {
                    (EvmError::Rpc(_), EvmError::Rpc(_)) => (),
                    (EvmError::Contract(_), EvmError::Contract(_)) => (),
                    (EvmError::Codec(_), EvmError::Codec(_)) => (),
                    _ => panic!("Error type mismatch"),
                }
            }
        }
    }
}