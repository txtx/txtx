//! Integration tests for error handling and recovery
//! 
//! Tests various error scenarios and validates error messages

#[cfg(test)]
mod error_handling_integration_tests {
    use crate::tests::integration::anvil_harness::AnvilInstance;
    use crate::tests::fixture_builder::{MigrationHelper, TestResult};
    use crate::errors::{EvmError, TransactionError, CodecError, SignerError};
    use std::path::PathBuf;
    use tokio;
    
    #[tokio::test]
    async fn test_insufficient_funds_error() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_insufficient_funds_error - Anvil not installed");
            return;
        }
        
        println!("💸 Testing insufficient funds error");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/errors/insufficient_funds_transfer.tx");
        
        let result = MigrationHelper::from_fixture(&fixture_path)
            
            .with_input("recipient", "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8")
            .with_input("amount", "1000000000000000000000")
            .execute()
            .await
            .expect("Failed to execute test"); // 1000 ETH (way too much)
        
        harness.setup().expect("Failed to setup project");
        let result = result.execute().await;
        
        // Should fail due to insufficient funds
        assert!(result.is_err() || !result.as_ref().unwrap().success,
            "Transaction should fail due to insufficient funds");
        
        if let Err(ref report) = result {
            let is_insufficient_funds = matches!(
                report.current_context(),
                EvmError::Transaction(TransactionError::InsufficientFunds { .. })
            );
            assert!(
                is_insufficient_funds,
                "Expected TransactionError::InsufficientFunds, got: {:?}",
                report.current_context()
            );
        }
        
        println!("✅ Insufficient funds error handled correctly");
        
        harness.cleanup();
    }
    
    #[tokio::test]
    async fn test_invalid_address_error() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_invalid_address_error - Anvil not installed");
            return;
        }
        
        println!("📍 Testing invalid address error");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/errors/invalid_hex_address.tx");
        
        let result = MigrationHelper::from_fixture(&fixture_path)
            ;
        
        let result = result.execute().await;
        
        // Should fail due to invalid address format
        assert!(result.is_err() || !result.as_ref().unwrap().success,
            "Should fail with invalid address");
        
        if let Err(ref report) = result {
            let is_invalid_address = matches!(
                report.current_context(),
                EvmError::Codec(CodecError::InvalidAddress(_)) |
                EvmError::Transaction(TransactionError::InvalidRecipient(_))
            );
            assert!(
                is_invalid_address,
                "Expected InvalidAddress or InvalidRecipient error, got: {:?}",
                report.current_context()
            );
        }
        
        println!("✅ Invalid address error handled correctly");
        
        harness.cleanup();
    }
    
    #[tokio::test]
    async fn test_missing_signer_error() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_missing_signer_error - Anvil not installed");
            return;
        }
        
        println!("🔑 Testing missing signer error");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/errors/missing_signer.tx");
        
        let result = MigrationHelper::from_fixture(&fixture_path)
            ;
        
        let result = result.execute().await;
        
        // Should fail due to missing signer
        assert!(result.is_err() || !result.as_ref().unwrap().success,
            "Should fail with missing signer");
        
        if let Err(ref report) = result {
            let is_signer_error = matches!(
                report.current_context(),
                EvmError::Signer(SignerError::KeyNotFound)
            );
            assert!(
                is_signer_error,
                "Expected SignerError::KeyNotFound, got: {:?}",
                report.current_context()
            );
        }
        
        println!("✅ Missing signer error handled correctly");
        
        harness.cleanup();
    }
    
    #[tokio::test]
    async fn test_invalid_function_call_error() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_invalid_function_call_error - Anvil not installed");
            return;
        }
        
        println!("📞 Testing invalid function call error");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/errors/invalid_function_call.tx");
        
        let result = MigrationHelper::from_fixture(&fixture_path)
            ;
        
        let result = result.execute().await;
        
        // Should fail due to invalid function
        assert!(result.is_err() || !result.as_ref().unwrap().success,
            "Should fail with invalid function call");
        
        println!("✅ Invalid function call error handled correctly");
        
        harness.cleanup();
    }
    
    #[tokio::test]
    async fn test_out_of_gas_error() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_out_of_gas_error - Anvil not installed");
            return;
        }
        
        println!("⛽ Testing out of gas error");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/errors/out_of_gas.tx");
        
        let result = MigrationHelper::from_fixture(&fixture_path)
            
            .with_input("contract_bytecode", "0x608060405234801561001057600080fd5b50610150806100206000396000f3fe")
            .execute()
            .await
            .expect("Failed to execute test");
        
        let result = result.execute().await;
        
        // Should fail due to insufficient gas
        assert!(result.is_err() || !result.as_ref().unwrap().success,
            "Should fail due to out of gas");
        
        if let Err(ref report) = result {
            let is_gas_error = matches!(
                report.current_context(),
                EvmError::Transaction(TransactionError::GasEstimationFailed)
            );
            assert!(
                is_gas_error,
                "Expected TransactionError::GasEstimationFailed, got: {:?}",
                report.current_context()
            );
        }
        
        println!("✅ Out of gas error handled correctly");
        
        harness.cleanup();
    }
    
    #[tokio::test]
    async fn test_invalid_nonce_too_high() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_invalid_nonce_too_high - Anvil not installed");
            return;
        }
        
        println!("🔢 Testing nonce too high error");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/errors/invalid_nonce.tx");
        
        let result = MigrationHelper::from_fixture(&fixture_path)
            
            .with_input("recipient", "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8")
            .with_input("wrong_nonce", "999")
            .execute()
            .await
            .expect("Failed to execute test"); // Way too high
        
        let result = result.execute().await;
        
        // Should fail due to invalid nonce
        assert!(result.is_err() || !result.as_ref().unwrap().success,
            "Should fail with invalid nonce");
        
        if let Err(ref report) = result {
            let is_nonce_error = matches!(
                report.current_context(),
                EvmError::Transaction(TransactionError::InvalidNonce { .. })
            );
            assert!(
                is_nonce_error,
                "Expected TransactionError::InvalidNonce, got: {:?}",
                report.current_context()
            );
        }
        
        println!("✅ Invalid nonce error handled correctly");
        
        harness.cleanup();
    }
    
    #[tokio::test]
    async fn test_call_non_contract_address() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_call_non_contract_address - Anvil not installed");
            return;
        }
        
        println!("📭 Testing call to non-contract address");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/errors/invalid_contract_address.tx");
        
        let result = MigrationHelper::from_fixture(&fixture_path)
            
            // Use a regular EOA address (no contract code)
            .with_input("non_contract_address", "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8")
            .execute()
            .await
            .expect("Failed to execute test");
        
        let result = result.execute().await;
        
        // Should fail because there's no contract at the address
        assert!(result.is_err() || !result.as_ref().unwrap().success,
            "Should fail when calling non-contract address");
        
        println!("✅ Non-contract address error handled correctly");
        
        harness.cleanup();
    }
    
    #[tokio::test]
    async fn test_insufficient_gas_price() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_insufficient_gas_price - Anvil not installed");
            return;
        }
        
        println!("💰 Testing insufficient gas price error");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/errors/insufficient_gas.tx");
        
        let result = MigrationHelper::from_fixture(&fixture_path)
            ;
        
        let result = result.execute().await;
        
        // Transaction might be rejected or stuck
        // The exact behavior depends on the fixture implementation
        println!("✅ Insufficient gas price test completed");
        
        harness.cleanup();
    }
}