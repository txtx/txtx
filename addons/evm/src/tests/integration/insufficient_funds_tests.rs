//! Insufficient funds test using txtx with filesystem fixtures

#[cfg(test)]
mod insufficient_funds_tests {
    use crate::tests::test_harness::ProjectTestHarness;
    use crate::tests::integration::anvil_harness::AnvilInstance;
    use crate::errors::{EvmError, TransactionError};
    use std::path::PathBuf;
    
    #[test]
    fn test_insufficient_funds_for_transfer() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_insufficient_funds_for_transfer - Anvil not installed");
            return;
        }
        
        println!("💸 Testing insufficient funds error handling through txtx");
        
        // Use fixture from filesystem
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/errors/insufficient_funds_transfer.tx");
        
        // Create harness with Anvil
        let mut harness = ProjectTestHarness::from_fixture(&fixture_path)
            .with_anvil();
        
        // Setup project
        harness.setup().expect("Failed to setup project");
        
        // Execute runbook - should fail
        let result = harness.execute_runbook();
        
        // Verify it failed
        assert!(result.is_err(), "Transaction should fail due to insufficient funds");
        
        let report = result.unwrap_err();
        println!("Expected error: {:?}", report);
        
        // Check error is about insufficient funds
        let is_insufficient_funds = matches!(
            report.current_context(),
            EvmError::Transaction(TransactionError::InsufficientFunds { .. })
        );
        assert!(
            is_insufficient_funds,
            "Expected TransactionError::InsufficientFunds, got: {:?}",
            report.current_context()
        );
        
        println!("Insufficient funds error correctly detected through txtx");
        
        harness.cleanup();
    }
    
    #[test]
    fn test_insufficient_funds_for_transfer_with_fixture() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("Warning: Skipping test_insufficient_funds_for_transfer_with_fixture - Anvil not installed");
            return;
        }
        
        println!("Testing insufficient funds for transfer using fixture");
        
        // Use the existing fixture for insufficient funds transfer
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/errors/insufficient_funds_transfer.tx");
        
        // Create harness with Anvil
        let mut harness = ProjectTestHarness::from_fixture(&fixture_path)
            .with_anvil();
        
        // Setup project
        harness.setup().expect("Failed to setup project");
        
        // Execute runbook - should fail
        let result = harness.execute_runbook();
        
        // Verify it failed with the right error
        assert!(result.is_err(), "Transaction should fail due to insufficient funds");
        
        let report = result.unwrap_err();
        println!("Expected error: {:?}", report);
        
        // Check that error mentions insufficient funds for transfer
        let is_insufficient_funds = matches!(
            report.current_context(),
            EvmError::Transaction(TransactionError::InsufficientFunds { .. })
        );
        assert!(
            is_insufficient_funds,
            "Expected TransactionError::InsufficientFunds, got: {:?}",
            report.current_context()
        );
        
        println!("Insufficient funds for transfer error correctly detected");
        
        harness.cleanup();
    }
    
    #[test]
    fn test_insufficient_funds_for_gas() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_insufficient_funds_for_gas - Anvil not installed");
            return;
        }
        
        println!("⛽ Testing insufficient funds for gas through txtx");
        
        // Use fixture from filesystem
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/errors/insufficient_gas.tx");
        
        // Create harness with Anvil
        let mut harness = ProjectTestHarness::from_fixture(&fixture_path)
            .with_anvil();
        
        // Setup project
        harness.setup().expect("Failed to setup project");
        
        // Execute runbook - should fail
        let result = harness.execute_runbook();
        
        // Verify it failed
        assert!(result.is_err(), "Transaction should fail due to insufficient funds for gas");
        
        let report = result.unwrap_err();
        println!("Expected error: {:?}", report);
        
        // Check error mentions gas or funds
        let is_gas_or_funds_error = matches!(
            report.current_context(),
            EvmError::Transaction(TransactionError::InsufficientFunds { .. }) |
            EvmError::Transaction(TransactionError::GasEstimationFailed)
        );
        assert!(
            is_gas_or_funds_error,
            "Expected InsufficientFunds or GasEstimationFailed, got: {:?}",
            report.current_context()
        );
        
        println!("Insufficient gas funds error correctly detected through txtx");
        
        harness.cleanup();
    }
}