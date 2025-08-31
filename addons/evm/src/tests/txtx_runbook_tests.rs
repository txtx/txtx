//! Tests for error-stack integration with EVM addon
//! 
//! These tests verify that our error types work correctly with error-stack

use txtx_addon_kit::Addon;
use txtx_test_utils::StdAddon;
use crate::EvmNetworkAddon;

pub fn get_addon_by_namespace(namespace: &str) -> Option<Box<dyn Addon>> {
    let available_addons: Vec<Box<dyn Addon>> = vec![
        Box::new(StdAddon::new()),
        Box::new(EvmNetworkAddon::new()),
    ];
    for addon in available_addons.into_iter() {
        if namespace.starts_with(&format!("{}", addon.get_namespace())) {
            return Some(addon);
        }
    }
    None
}

#[cfg(test)]
mod error_stack_integration {
    use crate::errors::{EvmError, TransactionError, ContractError, VerificationError};
    use error_stack::Report;
    
    #[test]
    fn test_transaction_errors_use_error_stack() {
        // Verify our error types work with error-stack
        let error = Report::new(EvmError::Transaction(
            TransactionError::InsufficientFunds {
                required: 1000000000000000000,
                available: 100000000000000000,
            }
        ))
        .attach_printable("Attempted to send 1 ETH")
        .attach_printable("Account balance: 0.1 ETH");
        
        // Check the error can be formatted
        let display = error.to_string();
        assert!(display.contains("Insufficient funds"));
        
        // This is the type of error that would be returned
        // from send_eth when execution fails
    }
    
    #[test]
    fn test_contract_errors_use_error_stack() {
        let error = Report::new(EvmError::Contract(
            ContractError::DeploymentFailed("Out of gas".to_string())
        ))
        .attach_printable("Contract size: 24KB")
        .attach_printable("Gas limit: 3000000");
        
        let display = error.to_string();
        assert!(display.contains("deployment failed"));
        
        // This would be returned from deploy_contract
    }
    
    #[test]
    fn test_verification_errors_use_error_stack() {
        let error = Report::new(EvmError::Verification(
            VerificationError::TransactionNotFound {
                tx_hash: "0xabc123".to_string(),
            }
        ))
        .attach_printable("Checked at block 1000000")
        .attach_printable("RPC: http://localhost:8545");
        
        let display = error.to_string();
        assert!(display.contains("not found"));
        
        // This would be returned from check_confirmations
    }
}