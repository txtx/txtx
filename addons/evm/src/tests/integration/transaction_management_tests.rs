//! Integration tests for transaction management
//! 
//! Tests nonce handling, gas estimation, and different transaction types

#[cfg(test)]
mod transaction_management_tests {
    use crate::tests::integration::anvil_harness::AnvilInstance;
    use std::path::PathBuf;
    use tokio;
    
    #[tokio::test]
    async fn test_nonce_management() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_nonce_management - Anvil not installed");
            return;
        }
        
        println!("🔢 Testing nonce management for sequential transactions");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/transactions/nonce_management.tx");
        
        // REMOVED:         let result = MigrationHelper::from_fixture(&fixture_path)
            .with_anvil()
            .with_input("recipient", "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8")
            .execute()
            .await
            .expect("Failed to execute test");
        
        
        
        assert!(result.success, "All transactions should succeed with proper nonces");
        
        // Verify we got three different transaction hashes
        let tx1 = result.outputs.get("tx1_hash").and_then(|v| v.as_string()).unwrap_or_default();
        let tx2 = result.outputs.get("tx2_hash").and_then(|v| v.as_string()).unwrap_or_default();
        let tx3 = result.outputs.get("tx3_hash").and_then(|v| v.as_string()).unwrap_or_default();
        
        assert!(!tx1.is_empty(), "Should have tx1 hash");
        assert!(!tx2.is_empty(), "Should have tx2 hash");
        assert!(!tx3.is_empty(), "Should have tx3 hash");
        assert_ne!(tx1, tx2, "Transaction hashes should be different");
        assert_ne!(tx2, tx3, "Transaction hashes should be different");
        
        println!("✅ Nonce management test passed");
        println!("   TX1: {}", &tx1[..10]);
        println!("   TX2: {}", &tx2[..10]);
        println!("   TX3: {}", &tx3[..10]);
        
        harness.cleanup();
    }
    
    #[tokio::test]
    async fn test_gas_estimation_transfer() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_gas_estimation_transfer - Anvil not installed");
            return;
        }
        
        println!("⛽ Testing gas estimation for ETH transfer");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/transactions/gas_estimation.tx");
        
        // REMOVED:         let result = MigrationHelper::from_fixture(&fixture_path)
            .with_anvil()
            .with_input("test_transfer", "true")
            .with_input("test_deploy", "false")
            .with_input("test_call", "false")
            .with_input("recipient", "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8")
            .execute()
            .await
            .expect("Failed to execute test");
        
        
        
        assert!(result.success, "Transfer should succeed");
        
        // ETH transfer should use approximately 21000 gas
        let gas_used = result.outputs.get("transfer_gas")
            .and_then(|v| v.as_string())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        
        // Allow some variance but should be close to 21000
        assert!(gas_used >= 20000 && gas_used <= 25000, 
            "ETH transfer should use ~21000 gas, got {}", gas_used);
        
        println!("✅ Gas estimation test passed");
        println!("   Transfer gas used: {}", gas_used);
        
        harness.cleanup();
    }
    
    #[tokio::test]
    async fn test_gas_estimation_deployment() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_gas_estimation_deployment - Anvil not installed");
            return;
        }
        
        println!("⛽ Testing gas estimation for contract deployment");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/transactions/gas_estimation.tx");
        
        // REMOVED:         let result = MigrationHelper::from_fixture(&fixture_path)
            .with_anvil()
            .with_input("test_transfer", "false")
            .with_input("test_deploy", "true")
            .with_input("test_call", "false")
            .execute()
            .await
            .expect("Failed to execute test");
        
        
        
        assert!(result.success, "Deployment should succeed");
        
        // Contract deployment should use significantly more gas than transfer
        let gas_used = result.outputs.get("deploy_gas")
            .and_then(|v| v.as_string())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        
        assert!(gas_used > 50000, 
            "Contract deployment should use significant gas, got {}", gas_used);
        
        println!("✅ Deployment gas estimation test passed");
        println!("   Deploy gas used: {}", gas_used);
        
        harness.cleanup();
    }
    
    #[tokio::test]
    async fn test_eip1559_transaction() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_eip1559_transaction - Anvil not installed");
            return;
        }
        
        println!("🔥 Testing EIP-1559 transaction with dynamic fees");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/transactions/eip1559_transaction.tx");
        
        // REMOVED:         let result = MigrationHelper::from_fixture(&fixture_path)
            .with_anvil()
            .with_input("recipient", "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8")
            .with_input("amount", "1000000000000000") // 0.001 ETH
            .with_input("max_fee_per_gas", "20000000000") // 20 gwei
            .with_input("max_priority_fee_per_gas", "1000000000")
            .execute()
            .await
            .expect("Failed to execute test"); // 1 gwei
        
        
        
        assert!(result.success, "EIP-1559 transaction should succeed");
        
        let tx_hash = result.outputs.get("tx_hash")
            .and_then(|v| v.as_string())
            .unwrap_or_default();
        
        let gas_used = result.outputs.get("gas_used")
            .and_then(|v| v.as_string())
            .unwrap_or_default();
        
        assert!(!tx_hash.is_empty(), "Should have transaction hash");
        assert!(!gas_used.is_empty(), "Should have gas used");
        
        println!("✅ EIP-1559 transaction test passed");
        println!("   TX: {}", &tx_hash[..10]);
        println!("   Gas used: {}", gas_used);
        
        harness.cleanup();
    }
    
    #[tokio::test]
    async fn test_legacy_transaction() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_legacy_transaction - Anvil not installed");
            return;
        }
        
        println!("🏛️ Testing legacy transaction format");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/transactions/legacy_transaction.tx");
        
        // REMOVED:         let result = MigrationHelper::from_fixture(&fixture_path)
            .with_anvil()
            .with_input("recipient", "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8")
            .with_input("amount", "1000000000000000")
            .execute()
            .await
            .expect("Failed to execute test"); // 0.001 ETH
        
        
        
        assert!(result.success, "Legacy transaction should succeed");
        
        let tx_hash = result.outputs.get("tx_hash")
            .and_then(|v| v.as_string())
            .unwrap_or_default();
        
        assert!(!tx_hash.is_empty(), "Should have transaction hash");
        
        println!("✅ Legacy transaction test passed");
        println!("   TX: {}", &tx_hash[..10]);
        
        harness.cleanup();
    }
    
    #[tokio::test]
    async fn test_batch_transactions() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_batch_transactions - Anvil not installed");
            return;
        }
        
        println!("📦 Testing batch transaction execution");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/transactions/batch_transactions.tx");
        
        // REMOVED:         let harness = MigrationHelper::from_fixture(&fixture_path)
            .with_anvil();
        
        
        
        assert!(result.success, "Batch transactions should succeed");
        
        // Verify outputs exist for batch processing
        let batch_complete = result.outputs.get("batch_complete")
            .and_then(|v| v.as_string())
            .unwrap_or_default();
        
        assert_eq!(batch_complete, "true", "Batch should complete successfully");
        
        println!("✅ Batch transactions test passed");
        
        harness.cleanup();
    }
}