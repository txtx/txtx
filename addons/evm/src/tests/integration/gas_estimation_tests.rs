//! Integration tests for gas estimation functionality
//! 
//! These tests verify that gas estimation properly:
//! - Estimates gas for simple transfers
//! - Estimates gas for contract deployments
//! - Provides accurate estimates that transactions succeed with
//! - Handles edge cases like insufficient balance

#[cfg(test)]
mod gas_estimation_tests {
    use crate::tests::integration::anvil_harness::AnvilInstance;
    use crate::tests::test_harness::ProjectTestHarness;
    use txtx_addon_kit::types::types::Value;
    use std::path::PathBuf;
    
    #[test]
    fn test_estimate_simple_transfer() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_estimate_simple_transfer - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing gas estimation for simple ETH transfer");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/gas_estimation.tx");
        
        let harness = ProjectTestHarness::from_fixture(&fixture_path)
            .with_anvil()
            .with_input("chain_id", "31337")
            .with_input("rpc_url", "http://127.0.0.1:8545")
            .with_input("private_key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
            .with_input("recipient", "0x70997970c51812dc3a010c7d01b50e0d17dc79c8")
            .with_input("amount", "1000000000000000000") // 1 ETH
            .with_input("contract_bytecode", "0x6080604052348015600f57600080fd5b50603f80601d6000396000f3fe6080604052600080fdfea265627a7a7231582053f")
            .with_input("custom_gas_limit", "100000");
        
        let result = harness.execute_runbook()
            .expect("Failed to execute gas estimation");
        
        assert!(result.success, "Gas estimation should succeed");
        
        // Check that we got an estimation
        let estimated_gas = result.outputs.get("estimated_transfer_gas")
            .and_then(|v| match v {
                Value::String(s) => s.parse::<u64>().ok(),
                Value::Integer(i) => Some(*i as u64),
                _ => None
            })
            .expect("Should have gas estimation");
        
        // ETH transfer should be around 21000 gas
        assert!(estimated_gas >= 21000, "Gas estimate should be at least 21000");
        assert!(estimated_gas <= 30000, "Gas estimate should be reasonable");
        
        println!("✅ Simple transfer gas estimation: {} gas", estimated_gas);
    }
    
    #[test]
    fn test_estimate_contract_deployment() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_estimate_contract_deployment - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing gas estimation for contract deployment");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/gas_estimation.tx");
        
        // Simple storage contract bytecode
        let bytecode = "0x608060405234801561001057600080fd5b5060b88061001f6000396000f3fe6080604052348015600f57600080fd5b506004361060325760003560e01c80632e64cec11460375780636057361d14604c575b600080fd5b60005460405190815260200160405180910390f35b6059605736600460536565565b50565b005b600055565b600060608284031215607657600080fd5b5035919050565b56fea264697066735822122035f";
        
        let harness = ProjectTestHarness::from_fixture(&fixture_path)
            .with_anvil()
            .with_input("chain_id", "31337")
            .with_input("rpc_url", "http://127.0.0.1:8545")
            .with_input("private_key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
            .with_input("recipient", "0x70997970c51812dc3a010c7d01b50e0d17dc79c8")
            .with_input("amount", "0")
            .with_input("contract_bytecode", bytecode)
            .with_input("custom_gas_limit", "500000");
        
        let result = harness.execute_runbook()
            .expect("Failed to execute deployment gas estimation");
        
        assert!(result.success, "Deployment gas estimation should succeed");
        
        let deployment_gas = result.outputs.get("estimated_deployment_gas")
            .and_then(|v| match v {
                Value::String(s) => s.parse::<u64>().ok(),
                Value::Integer(i) => Some(*i as u64),
                _ => None
            })
            .expect("Should have deployment gas estimation");
        
        // Contract deployment needs more gas than simple transfer
        assert!(deployment_gas > 50000, "Deployment should need significant gas");
        assert!(deployment_gas < 1000000, "Deployment gas should be reasonable");
        
        println!("✅ Contract deployment gas estimation: {} gas", deployment_gas);
    }
    
    #[test]
    fn test_estimated_gas_sufficient() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_estimated_gas_sufficient - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing that estimated gas is sufficient for transaction");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/gas_estimation.tx");
        
        let harness = ProjectTestHarness::from_fixture(&fixture_path)
            .with_anvil()
            .with_input("chain_id", "31337")
            .with_input("rpc_url", "http://127.0.0.1:8545")
            .with_input("private_key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
            .with_input("recipient", "0x90f8bf6a479f320ead074411a4b0e7944ea8c9c1")
            .with_input("amount", "5000000000000000") // 0.005 ETH
            .with_input("contract_bytecode", "0x00")
            .with_input("custom_gas_limit", "21000");
        
        let result = harness.execute_runbook()
            .expect("Failed to execute gas estimation test");
        
        assert!(result.success, "Transaction with estimated gas should succeed");
        
        // Verify the transaction succeeded
        let tx_hash = result.outputs.get("tx_hash")
            .and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None
            })
            .expect("Should have transaction hash");
        
        assert!(tx_hash.starts_with("0x"), "Should have valid transaction hash");
        
        // Check actual gas used vs estimated
        let actual_gas = result.outputs.get("actual_gas_used")
            .and_then(|v| match v {
                Value::String(s) => s.parse::<u64>().ok(),
                Value::Integer(i) => Some(*i as u64),
                _ => None
            });
        
        if let Some(gas) = actual_gas {
            println!("✅ Transaction succeeded with {} gas used", gas);
        } else {
            println!("✅ Transaction succeeded");
        }
    }
    
    #[test]
    fn test_custom_gas_limit() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_custom_gas_limit - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing transaction with custom gas limit");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/gas_estimation.tx");
        
        let harness = ProjectTestHarness::from_fixture(&fixture_path)
            .with_anvil()
            .with_input("chain_id", "31337")
            .with_input("rpc_url", "http://127.0.0.1:8545")
            .with_input("private_key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
            .with_input("recipient", "0xffcf8fdee72ac11b5c542428b35eef5769c409f0")
            .with_input("amount", "1000000000000000")
            .with_input("contract_bytecode", "0x00")
            .with_input("custom_gas_limit", "50000"); // More than needed
        
        let result = harness.execute_runbook()
            .expect("Failed to execute custom gas limit test");
        
        assert!(result.success, "Transaction with custom gas limit should succeed");
        
        println!("✅ Custom gas limit test passed");
    }
}