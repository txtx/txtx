//! Transaction types tests (Legacy, EIP-2930, EIP-1559)
//! 
//! These tests verify different transaction types:
//! - Legacy transactions (Type 0)
//! - EIP-2930 access list transactions (Type 1)
//! - EIP-1559 dynamic fee transactions (Type 2)
//! - Gas optimization with access lists

#[cfg(test)]
mod transaction_types_tests {
    use crate::tests::integration::anvil_harness::AnvilInstance;
    use crate::tests::test_harness::ProjectTestHarness;
    use txtx_addon_kit::types::types::Value;
    use std::path::PathBuf;
    
    #[test]
    fn test_legacy_transaction() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_legacy_transaction - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing legacy transaction (Type 0)");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/transaction_types.tx");
        
        let access_list = r#"[]"#;
        
        let harness = ProjectTestHarness::from_fixture(&fixture_path)
            .with_anvil()
            .with_input("chain_id", "31337")
            .with_input("rpc_url", "http://127.0.0.1:8545")
            .with_input("private_key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
            .with_input("recipient", "0x70997970c51812dc3a010c7d01b50e0d17dc79c8")
            .with_input("amount", "1000000000000000000")
            .with_input("gas_price", "20000000000")
            .with_input("max_fee_per_gas", "30000000000")
            .with_input("max_priority_fee", "2000000000")
            .with_input("access_list", access_list);
        
        let result = harness.execute_runbook()
            .expect("Failed to execute legacy transaction test");
        
        assert!(result.success, "Legacy transaction should succeed");
        
        let tx_hash = result.outputs.get("legacy_tx_hash")
            .and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None
            })
            .expect("Should have legacy transaction hash");
        
        assert!(tx_hash.starts_with("0x"), "Should have valid transaction hash");
        
        println!("✅ Legacy transaction sent: {}", tx_hash);
    }
    
    #[test]
    fn test_eip2930_access_list_transaction() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_eip2930_access_list_transaction - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing EIP-2930 access list transaction (Type 1)");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/transaction_types.tx");
        
        // Access list with contract address and storage keys
        let access_list = r#"[
            {
                "address": "0x5FbDB2315678afecb367f032d93F642f64180aa3",
                "storageKeys": ["0x0000000000000000000000000000000000000000000000000000000000000000"]
            }
        ]"#;
        
        let harness = ProjectTestHarness::from_fixture(&fixture_path)
            .with_anvil()
            .with_input("chain_id", "31337")
            .with_input("rpc_url", "http://127.0.0.1:8545")
            .with_input("private_key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
            .with_input("recipient", "0x3c44cdddb6a900fa2b585dd299e03d12fa4293bc")
            .with_input("amount", "2000000000000000000")
            .with_input("gas_price", "25000000000")
            .with_input("max_fee_per_gas", "30000000000")
            .with_input("max_priority_fee", "2000000000")
            .with_input("access_list", access_list);
        
        let result = harness.execute_runbook()
            .expect("Failed to execute EIP-2930 transaction test");
        
        assert!(result.success, "EIP-2930 transaction should succeed");
        
        let tx_hash = result.outputs.get("eip2930_tx_hash")
            .and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None
            })
            .expect("Should have EIP-2930 transaction hash");
        
        assert!(tx_hash.starts_with("0x"), "Should have valid transaction hash");
        
        println!("✅ EIP-2930 transaction sent: {}", tx_hash);
    }
    
    #[test]
    fn test_eip1559_dynamic_fee_transaction() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_eip1559_dynamic_fee_transaction - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing EIP-1559 dynamic fee transaction (Type 2)");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/transaction_types.tx");
        
        let harness = ProjectTestHarness::from_fixture(&fixture_path)
            .with_anvil()
            .with_input("chain_id", "31337")
            .with_input("rpc_url", "http://127.0.0.1:8545")
            .with_input("private_key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
            .with_input("recipient", "0x90f8bf6a479f320ead074411a4b0e7944ea8c9c1")
            .with_input("amount", "3000000000000000000")
            .with_input("gas_price", "20000000000")
            .with_input("max_fee_per_gas", "40000000000")
            .with_input("max_priority_fee", "3000000000")
            .with_input("access_list", "[]");
        
        let result = harness.execute_runbook()
            .expect("Failed to execute EIP-1559 transaction test");
        
        assert!(result.success, "EIP-1559 transaction should succeed");
        
        let tx_hash = result.outputs.get("eip1559_tx_hash")
            .and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None
            })
            .expect("Should have EIP-1559 transaction hash");
        
        assert!(tx_hash.starts_with("0x"), "Should have valid transaction hash");
        
        println!("✅ EIP-1559 transaction sent: {}", tx_hash);
    }
    
    /// Test: Gas optimization with access lists
    /// 
    /// TODO: This test requires a deployed contract at a specific address
    /// 
    /// Should test:
    /// - Access lists reduce gas costs for storage operations
    /// - Gas savings are measurable
    /// - Access list generation is accurate
    #[test]
    #[ignore = "Requires contract deployment - fixture assumes existing contract"]
    fn test_access_list_gas_optimization() {
        // TODO: Deploy storage contract first
        // TODO: Compare gas with and without access list
        // TODO: Verify gas savings percentage
        
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_access_list_gas_optimization - Anvil not installed");
            return;
        }
        
        panic!("Test requires contract deployment before access list testing");
    }
}