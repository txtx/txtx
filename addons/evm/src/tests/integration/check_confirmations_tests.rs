//! Integration tests for check_confirmations action
//! 
//! These tests verify that the check_confirmations action properly:
//! - Waits for transaction inclusion in a block
//! - Waits for the specified number of confirmations
//! - Extracts contract addresses and logs from receipts
//!
//! All tests use a single comprehensive fixture with different input parameters

#[cfg(test)]
mod check_confirmations_integration_tests {
    use crate::tests::integration::anvil_harness::AnvilInstance;
    use txtx_addon_kit::types::types::Value;
    use std::path::PathBuf;
    use tokio;
    
    #[tokio::test]
    async fn test_check_confirmations_basic() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_check_confirmations_basic - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing check_confirmations with basic ETH transfer");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/check_confirmations_transfer.tx");
        
        // REMOVED:         let result = MigrationHelper::from_fixture(&fixture_path)
            .with_input("recipient_address", "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8")
            .with_input("amount", "1000000000000000") // 0.001 ETH
            .with_input("confirmations", "3")
            .execute()
            .await
            .expect("Failed to execute test");
        
        assert!(result.success, "Check confirmations should succeed");
        
        // Verify we got the tx_hash output
        let tx_hash = result.outputs.get("tx_hash")
            .expect("Should have tx_hash output");
        let tx_hash_str = match tx_hash {
            Value::String(s) => s.clone(),
            _ => panic!("tx_hash should be a string")
        };
        
        println!("✅ Basic confirmation test passed");
        println!("   Transaction: {}", tx_hash_str);
        
        harness.cleanup();
    }
    
    #[tokio::test]
    async fn test_check_confirmations_with_contract_deployment() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_check_confirmations_with_contract_deployment - Anvil not installed");
            return;
        }
        
        println!("🚀 Testing check_confirmations with contract deployment");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/check_confirmations_deployment.tx");
        
        // REMOVED:         let result = MigrationHelper::from_fixture(&fixture_path)
            
            .with_input("bytecode", "0x602a60005260206000f3") // Returns 42
            .with_input("confirmations", "2")
            .execute()
            .await
            .expect("Failed to execute test");
        
        
        
        assert!(result.success, "Deployment and confirmation should succeed");
        
        // Verify both actions returned the same contract address
        let deployed_addr = result.outputs.get("deployed_address")
            .and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None
            })
            .unwrap_or_default();
        let confirmed_addr = result.outputs.get("confirmed_address")
            .and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None
            })
            .unwrap_or_default();
        
        assert!(!deployed_addr.is_empty(), "Should have deployed address");
        assert_eq!(deployed_addr, confirmed_addr, 
            "check_confirmations should return the same contract address");
        
        println!("✅ Deployment confirmation test passed");
        println!("   Contract deployed at: {}", deployed_addr);
        
        harness.cleanup();
    }
    
    #[tokio::test]
    async fn test_check_confirmations_with_different_counts() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_check_confirmations_with_different_counts - Anvil not installed");
            return;
        }
        
        println!("🔢 Testing check_confirmations with different confirmation counts");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/check_confirmations_transfer.tx");
        
        // Test 1: Quick confirmation (1 block)
        println!("   Testing with 1 confirmation...");
        // REMOVED:         let result = MigrationHelper::from_fixture(&fixture_path)
            
            .with_input("recipient_address", "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8")
            .with_input("amount", "1000000000000000")
            .with_input("confirmations", "1")
            .execute()
            .await
            .expect("Failed to execute test");
        
        
        assert!(result.success, "1 confirmation should succeed");
        harness.cleanup();
        
        // Test 2: More secure confirmation (5 blocks)
        println!("   Testing with 5 confirmations...");
        // REMOVED:         let result = MigrationHelper::from_fixture(&fixture_path)
            
            .with_input("recipient_address", "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8")
            .with_input("amount", "1000000000000000")
            .with_input("confirmations", "5")
            .execute()
            .await
            .expect("Failed to execute test");
        
        
        assert!(result.success, "5 confirmations should succeed");
        
        println!("✅ Different confirmation counts test passed");
        
        harness.cleanup();
    }
}