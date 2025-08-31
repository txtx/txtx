//! Integration tests for contract interactions
//! 
//! Tests function calls, event logs, and complex interactions

#[cfg(test)]
mod contract_interaction_tests {
    use crate::tests::integration::anvil_harness::AnvilInstance;
    use crate::tests::test_harness::ProjectTestHarness;
    use std::path::PathBuf;
    
    #[test]
    fn test_multi_contract_calls() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_multi_contract_calls - Anvil not installed");
            return;
        }
        
        println!("📞 Testing multiple contract calls in sequence");
        
        // First deploy a simple storage contract
        let deploy_fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/deployments/storage_contract.tx");
        
        let mut deploy_harness = ProjectTestHarness::from_fixture(&deploy_fixture)
            .with_anvil();
        
        let deploy_result = deploy_harness.execute_runbook()
            .expect("Failed to deploy storage contract");
        
        let contract_address = deploy_result.outputs.get("contract_address")
            .and_then(|v| v.as_string())
            .unwrap_or_default();
        
        assert!(!contract_address.is_empty(), "Should have deployed contract");
        
        // Now test multiple calls
        let multi_call_fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/contracts/multi_call.tx");
        
        let mut call_harness = ProjectTestHarness::from_fixture(&multi_call_fixture)
            .with_anvil()
            .with_input("contract_address", &contract_address)
            .with_input("value_to_set", "100");
        
        let call_result = call_harness.execute_runbook()
            .expect("Failed to execute multi-call");
        
        // Verify we got transaction hashes for state-changing calls
        let setter_tx = call_result.outputs.get("setter_tx")
            .and_then(|v| v.as_string())
            .unwrap_or_default();
        let increment_tx = call_result.outputs.get("increment_tx")
            .and_then(|v| v.as_string())
            .unwrap_or_default();
        
        assert!(!setter_tx.is_empty(), "Should have setter transaction");
        assert!(!increment_tx.is_empty(), "Should have increment transaction");
        
        println!("✅ Multi-contract calls test passed");
        println!("   Contract: {}", &contract_address[..10]);
        println!("   Setter TX: {}", &setter_tx[..10]);
        println!("   Increment TX: {}", &increment_tx[..10]);
        
        deploy_harness.cleanup();
        call_harness.cleanup();
    }
    
    #[test]
    fn test_transaction_receipt_data() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_transaction_receipt_data - Anvil not installed");
            return;
        }
        
        println!("📋 Testing transaction receipt data extraction");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/transactions/transaction_receipt.tx");
        
        let mut harness = ProjectTestHarness::from_fixture(&fixture_path)
            .with_anvil()
            .with_input("recipient", "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8")
            .with_input("amount", "1000000000000000"); // 0.001 ETH
        
        let result = harness.execute_runbook()
            .expect("Failed to execute transaction");
        
        // Verify receipt data was extracted
        let tx_hash = result.outputs.get("tx_hash")
            .and_then(|v| v.as_string())
            .unwrap_or_default();
        let gas_used = result.outputs.get("gas_used")
            .and_then(|v| v.as_string())
            .unwrap_or_default();
        
        assert!(!tx_hash.is_empty(), "Should have transaction hash");
        assert!(!gas_used.is_empty(), "Should have gas used from receipt");
        
        println!("✅ Transaction receipt test passed");
        println!("   TX: {}", &tx_hash[..10]);
        println!("   Gas used: {}", gas_used);
        
        harness.cleanup();
    }
    
    #[test]
    fn test_view_function_calls() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_view_function_calls - Anvil not installed");
            return;
        }
        
        println!("👁️ Testing view function calls (no gas required)");
        
        // Deploy a contract first
        let deploy_fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/deployments/storage_contract.tx");
        
        let mut harness = ProjectTestHarness::from_fixture(&deploy_fixture)
            .with_anvil();
        
        let result = harness.execute_runbook()
            .expect("Failed to deploy contract");
        
        let contract_address = result.outputs.get("contract_address")
            .and_then(|v| v.as_string())
            .unwrap_or_default();
        
        // The storage contract fixture already tests view functions
        let initial_value = result.outputs.get("initial_value")
            .and_then(|v| v.as_string())
            .unwrap_or_default();
        let updated_value = result.outputs.get("updated_value")
            .and_then(|v| v.as_string())
            .unwrap_or_default();
        
        assert!(!initial_value.is_empty(), "Should read initial value");
        assert!(!updated_value.is_empty(), "Should read updated value");
        assert_ne!(initial_value, updated_value, "Values should be different");
        
        println!("✅ View function calls test passed");
        println!("   Initial: {}", initial_value);
        println!("   Updated: {}", updated_value);
        
        harness.cleanup();
    }
    
    #[test]
    fn test_contract_deployment_with_args() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_contract_deployment_with_args - Anvil not installed");
            return;
        }
        
        println!("🏗️ Testing contract deployment with constructor arguments");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/deployments/constructor_args.tx");
        
        let mut harness = ProjectTestHarness::from_fixture(&fixture_path)
            .with_anvil();
        
        let result = harness.execute_runbook()
            .expect("Failed to deploy with constructor args");
        
        let contract_address = result.outputs.get("contract_address")
            .and_then(|v| v.as_string())
            .unwrap_or_default();
        
        assert!(!contract_address.is_empty(), "Should have deployed contract");
        
        // Verify constructor args were properly set
        let owner = result.outputs.get("owner")
            .and_then(|v| v.as_string())
            .unwrap_or_default();
        let initial_supply = result.outputs.get("initial_supply")
            .and_then(|v| v.as_string())
            .unwrap_or_default();
        
        assert!(!owner.is_empty(), "Should have owner from constructor");
        assert!(!initial_supply.is_empty(), "Should have initial supply");
        
        println!("✅ Constructor arguments test passed");
        println!("   Contract: {}", &contract_address[..10]);
        println!("   Owner: {}", &owner[..10]);
        println!("   Supply: {}", initial_supply);
        
        harness.cleanup();
    }
}