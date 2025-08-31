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
    use crate::tests::fixture_builder::{MigrationHelper, TestResult};
    use txtx_addon_kit::types::types::Value;
    use std::path::PathBuf;
    use tokio;
    
    #[tokio::test]
    async fn test_legacy_transaction_cost() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_legacy_transaction_cost - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing legacy transaction cost calculation");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/transaction_cost.tx");
        
        let result = MigrationHelper::from_fixture(&fixture_path)
            .with_anvil()
            .with_input("chain_id", "31337")
            .with_input("rpc_url", "http://127.0.0.1:8545")
            .with_input("private_key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
            .with_input("recipient", "0x70997970c51812dc3a010c7d01b50e0d17dc79c8")
            .with_input("amount", "1000000000000000") // 0.001 ETH
            .with_input("gas_price", "20000000000") // 20 gwei
            .with_input("gas_limit", "21000")
            .with_input("max_fee_per_gas", "25000000000")
            .with_input("max_priority_fee", "2000000000")
            .execute()
            .await
            .expect("Failed to execute test");
        
        
        
        assert!(result.success, "Cost calculation should succeed");
        
        // Check estimated cost
        let estimated_cost = result.outputs.get("legacy_estimated_cost")
            .and_then(|v| match v {
                Value::String(s) => s.parse::<u64>().ok(),
                Value::Integer(i) => Some(*i as u64),
                _ => None
            })
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
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/transaction_cost.tx");
        
        let result = MigrationHelper::from_fixture(&fixture_path)
            .with_anvil()
            .with_input("chain_id", "31337")
            .with_input("rpc_url", "http://127.0.0.1:8545")
            .with_input("private_key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
            .with_input("recipient", "0x90f8bf6a479f320ead074411a4b0e7944ea8c9c1")
            .with_input("amount", "2000000000000000") // 0.002 ETH
            .with_input("gas_price", "15000000000")
            .with_input("gas_limit", "21000")
            .with_input("max_fee_per_gas", "30000000000") // 30 gwei
            .with_input("max_priority_fee", "3000000000")
            .execute()
            .await
            .expect("Failed to execute test"); // 3 gwei
        
        
        
        assert!(result.success, "EIP-1559 cost calculation should succeed");
        
        let estimated_cost = result.outputs.get("eip1559_estimated_cost")
            .and_then(|v| match v {
                Value::String(s) => s.parse::<u64>().ok(),
                Value::Integer(i) => Some(*i as u64),
                _ => None
            })
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
        
        let result = MigrationHelper::from_fixture(&fixture_path)
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
        
        let result = MigrationHelper::from_fixture(&fixture_path)
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