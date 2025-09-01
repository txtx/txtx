//! Integration tests for transaction cost calculation
//! 
//! These tests verify that transaction cost calculations:
//! - Accurately predict costs for legacy transactions
//! - Handle EIP-1559 transactions correctly
//! - Match actual costs from receipts
//! - Handle different gas price scenarios

#[cfg(test)]
mod transaction_cost_tests {
    use crate::tests::integration::anvil_harness::AnvilInstance;
    use crate::tests::fixture_builder::{FixtureBuilder, get_anvil_manager};
    use std::path::PathBuf;
    use std::fs;
    use tokio;
    
    #[tokio::test]
    async fn test_legacy_transaction_cost() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_legacy_transaction_cost - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing legacy transaction cost calculation");
        
        // ARRANGE: Load the fixture and create test setup
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/transaction_cost.tx");
        let fixture_content = fs::read_to_string(&fixture_path)
            .expect("Failed to read fixture file");
        
        let mut fixture = FixtureBuilder::new("test_legacy_cost")
            .with_anvil_manager(get_anvil_manager().await.unwrap())
            .with_runbook("main", &fixture_content)
            .with_parameter("chain_id", "31337")
            .with_parameter("gas_price", "20000000000") // 20 gwei
            .with_parameter("gas_limit", "21000")
            .with_parameter("amount", "1000000000000000") // 0.001 ETH
            .with_parameter("max_fee_per_gas", "25000000000")
            .with_parameter("max_priority_fee", "2000000000")
            .build()
            .await
            .expect("Failed to build fixture");
        
        // Add account parameters from Anvil
        let accounts = fixture.anvil_handle.accounts();
        fixture.config.parameters.insert("private_key".to_string(), accounts.alice.secret_string());
        fixture.config.parameters.insert("recipient".to_string(), accounts.bob.address_string());
        fixture.config.parameters.insert("rpc_url".to_string(), fixture.rpc_url.clone());
        
        // ACT: Execute the runbook to calculate costs
        fixture.execute_runbook("main").await
            .expect("Failed to execute runbook");
        
        // ASSERT: Verify the cost calculation
        let outputs = fixture.get_outputs("main")
            .expect("Should have outputs");
        
        let estimated_cost = outputs.get("legacy_estimated_cost")
            .and_then(|v| v.as_string())
            .and_then(|s| s.parse::<u64>().ok())
            .or_else(|| outputs.get("legacy_estimated_cost")
                .and_then(|v| v.as_integer())
                .map(|i| i as u64))
            .expect("Should have estimated cost");
        
        // Cost = gas_limit * gas_price = 21000 * 20000000000
        let expected_cost = 21000u64 * 20000000000u64;
        assert_eq!(estimated_cost, expected_cost, "Legacy cost calculation should be accurate");
        
        println!("✅ Legacy transaction cost: {} wei", estimated_cost);
    }
    
    #[tokio::test]
    async fn test_eip1559_transaction_cost() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_eip1559_transaction_cost - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing EIP-1559 transaction cost calculation");
        
        // ARRANGE: Load fixture and create test setup
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/transaction_cost.tx");
        let fixture_content = fs::read_to_string(&fixture_path)
            .expect("Failed to read fixture file");
        
        let mut fixture = FixtureBuilder::new("test_eip1559_cost")
            .with_anvil_manager(get_anvil_manager().await.unwrap())
            .with_runbook("main", &fixture_content)
            .with_parameter("chain_id", "31337")
            .with_parameter("gas_price", "15000000000")
            .with_parameter("gas_limit", "21000")
            .with_parameter("amount", "2000000000000000") // 0.002 ETH
            .with_parameter("max_fee_per_gas", "30000000000") // 30 gwei
            .with_parameter("max_priority_fee", "3000000000") // 3 gwei
            .build()
            .await
            .expect("Failed to build fixture");
        
        // Add account parameters
        let accounts = fixture.anvil_handle.accounts();
        fixture.config.parameters.insert("private_key".to_string(), accounts.alice.secret_string());
        fixture.config.parameters.insert("recipient".to_string(), accounts.bob.address_string());
        fixture.config.parameters.insert("rpc_url".to_string(), fixture.rpc_url.clone());
        
        // ACT: Execute runbook
        fixture.execute_runbook("main").await
            .expect("Failed to execute runbook");
        
        // ASSERT: Verify EIP-1559 cost calculation
        let outputs = fixture.get_outputs("main")
            .expect("Should have outputs");
        
        let estimated_cost = outputs.get("eip1559_estimated_cost")
            .and_then(|v| v.as_string())
            .and_then(|s| s.parse::<u64>().ok())
            .or_else(|| outputs.get("eip1559_estimated_cost")
                .and_then(|v| v.as_integer())
                .map(|i| i as u64))
            .expect("Should have EIP-1559 estimated cost");
        
        // Maximum cost = gas_limit * max_fee_per_gas
        let max_cost = 21000u64 * 30000000000u64;
        assert_eq!(estimated_cost, max_cost, "EIP-1559 max cost should be calculated");
        
        println!("✅ EIP-1559 max transaction cost: {} wei", estimated_cost);
    }
    
    #[tokio::test]
    async fn test_high_gas_price_cost() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_high_gas_price_cost - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing transaction cost with high gas price");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/transaction_cost.tx");
        
        // REMOVED:         let result = MigrationHelper::from_fixture(&fixture_path)
            .with_anvil()
            .with_input("chain_id", "31337")
            .with_input("rpc_url", "http://127.0.0.1:8545")
            .with_input("private_key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
            .with_input("recipient", "0x22d491bde2303f2f43325b2108d26f1eaba1e32b")
            .with_input("amount", "100000000000000") // 0.0001 ETH
            .with_input("gas_price", "100000000000") // 100 gwei (high)
            .with_input("gas_limit", "21000")
            .with_input("max_fee_per_gas", "150000000000")
            .with_input("max_priority_fee", "10000000000")
            .execute()
            .await
            .expect("Failed to execute test");
        
        
        
        assert!(result.success, "High gas price calculation should succeed");
        
        let high_cost = result.outputs.get("legacy_estimated_cost")
            .and_then(|v| match v {
                Value::String(s) => s.parse::<u64>().ok(),
                Value::Integer(i) => Some(*i as u64),
                _ => None
            })
            .expect("Should have high gas cost");
        
        // High cost = 21000 * 100 gwei
        assert_eq!(high_cost, 2100000000000000u64, "High gas price cost should be accurate");
        
        println!("✅ High gas price cost: {} wei (0.0021 ETH)", high_cost);
    }
    
    #[tokio::test]
    async fn test_zero_gas_price() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_zero_gas_price - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing transaction cost with zero gas price");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/transaction_cost.tx");
        
        // REMOVED:         let result = MigrationHelper::from_fixture(&fixture_path)
            .with_anvil()
            .with_input("chain_id", "31337")
            .with_input("rpc_url", "http://127.0.0.1:8545")
            .with_input("private_key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
            .with_input("recipient", "0xe11ba2b4d45eaed5996cd0823791e0c93114882d")
            .with_input("amount", "1000000000000")
            .with_input("gas_price", "0") // Free gas (test networks)
            .with_input("gas_limit", "21000")
            .with_input("max_fee_per_gas", "0")
            .with_input("max_priority_fee", "0")
            .execute()
            .await
            .expect("Failed to execute test");
        
        
        
        assert!(result.success, "Zero gas price calculation should succeed");
        
        let zero_cost = result.outputs.get("legacy_estimated_cost")
            .and_then(|v| match v {
                Value::String(s) => s.parse::<u64>().ok(),
                Value::Integer(i) => Some(*i as u64),
                _ => None
            })
            .expect("Should have zero cost");
        
        assert_eq!(zero_cost, 0, "Zero gas price should result in zero cost");
        
        println!("✅ Zero gas price cost: {} wei (free)", zero_cost);
    }
}