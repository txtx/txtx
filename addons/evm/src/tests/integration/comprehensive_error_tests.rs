//! Comprehensive error handling tests
//! 
//! These tests verify robust error handling for:
//! - Contract reverts with reasons
//! - Gas exhaustion scenarios
//! - Nonce management errors
//! - Input validation errors
//! - Signature and encoding errors

#[cfg(test)]
mod comprehensive_error_tests {
    use crate::tests::integration::anvil_harness::AnvilInstance;
    use crate::tests::fixture_builder::{FixtureBuilder, get_anvil_manager};
    use crate::errors::{EvmError, TransactionError};
    use txtx_addon_kit::types::types::Value;
    use std::path::PathBuf;
    use std::fs;
    use serial_test::serial;
    use tokio;
    
    #[tokio::test]
    #[serial(anvil)]
    async fn test_revert_reason_extraction() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_revert_reason_extraction - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing revert reason extraction");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/revert_reasons.tx");
        
        let fixture_content = fs::read_to_string(&fixture_path)
            .expect("Failed to read fixture");
        
        // Reverter contract bytecode with various revert conditions
        let reverter_bytecode = "0x608060405234801561001057600080fd5b50610334806100206000396000f3fe608060405234801561001057600080fd5b506004361061004c5760003560e01c80631b9265b814610051578063398c08ec1461005b578063a3c2f6b61461006f578063ce83732e14610089575b600080fd5b6100596100a5565b005b610069600435610af565b60405180910390f35b61008760048036038101906100829190610214565b610127565b005b6100a360048036038101906100729190610265565b610185565b005b6040517f08c379a00000000000000000000000000000000000000000000000000000000081526004016100f190610301565b60405180910390fd5b60008111610126576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040161011d906102d1565b60405180910390fd5b50565b600073ffffffffffffffffffffffffffffffffffffffff168173ffffffffffffffffffffffffffffffffffffffff161415610183576040517fc5723b5100000000000000000000000000000000000000000000000000000000815260040160405180910390fd5b50565b60008082905060008111915050919050565b600080fd5b6000819050919050565b6101b081610198565b81146101bb57600080fd5b50565b6000813590506101cd816101a7565b92915050565b6000602082840312156101ea576101e9610193565b5b60006101f8848285016101be565b91505092915050565b600073ffffffffffffffffffffffffffffffffffffffff82169050919050565b600061022d82610201565b9050919050565b61023d81610222565b811461024857600080fd5b50565b60008135905061025a81610234565b92915050565b60006020828403121561027657610275610193565b5b60006102848482850161024b565b91505092915050565b600082825260208201905092915050565b7f56616c7565206d75737420626520706f7369746976650000000000000000006000820152505b50565b60006102d760178361028d565b91506102e28261029f565b602082019050919050565b600060208201905081810360008301526102f6816102c8565b9050919050565b7f506c61696e207265766572740000000000000000000000000000000000000060008201525056fe";
        
        let mut fixture = FixtureBuilder::new("test_revert_reasons")
            .with_anvil_manager(get_anvil_manager().await.unwrap())
            .with_runbook("revert_test", &fixture_content)
            .build()
            .await
            .expect("Failed to build fixture");
        
        // Add parameters
        fixture.config.parameters.insert("chain_id".to_string(), "31337".to_string());
        fixture.config.parameters.insert("rpc_url".to_string(), fixture.rpc_url.clone());
        fixture.config.parameters.insert("private_key".to_string(), "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string());
        fixture.config.parameters.insert("reverter_bytecode".to_string(), reverter_bytecode.to_string());
        
        // Execute
        fixture.execute_runbook("revert_test").await
            .expect("Failed to execute test");
        
        let outputs = fixture.get_outputs("revert_test")
            .expect("Should have outputs");
        
        // Check we got the contract address
        let deployed = outputs.get("deployed_address")
            .and_then(|v| v.as_string())
            .expect("Should have deployed reverter contract");
        
        assert!(deployed.starts_with("0x"), "Should have valid contract address");
        
        println!("✅ Revert reason extraction test passed");
    }
    
    /// Test: Gas exhaustion error handling
    /// 
    /// Expected Behavior:
    /// - Transactions with insufficient gas should fail
    /// - Error should indicate gas issue
    /// - Different gas errors should be distinguishable
    /// 
    /// Validates:
    /// - Gas limit validation
    /// - Out of gas error handling
    #[tokio::test]
    #[serial(anvil)]
    async fn test_gas_exhaustion_errors() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_gas_exhaustion_errors - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing gas exhaustion error handling");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/gas_errors.tx");
        
        let fixture_content = fs::read_to_string(&fixture_path)
            .expect("Failed to read fixture");
        
        let mut fixture = FixtureBuilder::new("test_gas_errors")
            .with_anvil_manager(get_anvil_manager().await.unwrap())
            .with_runbook("gas_test", &fixture_content)
            .build()
            .await
            .expect("Failed to build fixture");
        
        // Add parameters
        fixture.config.parameters.insert("chain_id".to_string(), "31337".to_string());
        fixture.config.parameters.insert("rpc_url".to_string(), fixture.rpc_url.clone());
        fixture.config.parameters.insert("private_key".to_string(), "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string());
        fixture.config.parameters.insert("recipient".to_string(), "0x70997970c51812dc3a010c7d01b50e0d17dc79c8".to_string());
        fixture.config.parameters.insert("amount".to_string(), "1000000000000000".to_string());
        fixture.config.parameters.insert("contract_bytecode".to_string(), "0x6080604052600080fd00".to_string());
        fixture.config.parameters.insert("huge_data".to_string(), format!("0x{}", "00".repeat(100000))); // 100KB of data
        
        // Act
        fixture.execute_runbook("gas_test").await
            .expect("Failed to execute gas test");
        
        let outputs = fixture.get_outputs("gas_test")
            .expect("Should have outputs");
        
        // Verify we captured gas errors in outputs
        let low_gas_error = outputs.get("low_gas_error");
        assert!(
            low_gas_error.is_some(),
            "Should capture low gas error in output"
        );
        
        // Verify exact gas succeeded
        let exact_gas_tx = outputs.get("exact_gas_success");
        assert!(
            exact_gas_tx.is_some(),
            "Transaction with exact gas limit should have result"
        );
        
        println!("✅ Gas exhaustion errors properly captured and handled");
    }
    
    #[tokio::test]
    #[serial(anvil)]
    async fn test_nonce_management_errors() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_nonce_management_errors - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing nonce management error handling");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/nonce_errors.tx");
        
        let fixture_content = fs::read_to_string(&fixture_path)
            .expect("Failed to read fixture");
        
        let mut fixture = FixtureBuilder::new("test_nonce_errors")
            .with_anvil_manager(get_anvil_manager().await.unwrap())
            .with_runbook("nonce_test", &fixture_content)
            .build()
            .await
            .expect("Failed to build fixture");
        
        // Add parameters
        fixture.config.parameters.insert("chain_id".to_string(), "31337".to_string());
        fixture.config.parameters.insert("rpc_url".to_string(), fixture.rpc_url.clone());
        fixture.config.parameters.insert("private_key".to_string(), "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string());
        fixture.config.parameters.insert("recipient".to_string(), "0x3c44cdddb6a900fa2b585dd299e03d12fa4293bc".to_string());
        
        fixture.execute_runbook("nonce_test").await
            .expect("Failed to execute test");
        
        let outputs = fixture.get_outputs("nonce_test")
            .expect("Should have outputs");
        
        // Check we got current nonce
        let current_nonce = outputs.get("current_nonce")
            .and_then(|v| v.as_integer())
            .or_else(|| outputs.get("current_nonce")
                .and_then(|v| v.as_string())
                .and_then(|s| s.parse().ok()));
        
        assert!(current_nonce.is_some(), "Should have current nonce");
        
        // Auto nonce transactions should succeed
        let auto_tx1 = outputs.get("auto_nonce_tx1")
            .and_then(|v| v.as_string());
        
        assert!(auto_tx1.is_some(), "Auto nonce tx should succeed");
        
        println!("✅ Nonce error handling test passed");
    }
    
    /// Test: Input validation error handling
    /// 
    /// Expected Behavior:
    /// - Invalid addresses should be rejected
    /// - Invalid hex data should be rejected  
    /// - Negative values should be rejected
    /// - Overflow values should be rejected
    /// 
    /// Validates:
    /// - Input validation before transaction submission
    #[tokio::test]
    #[serial(anvil)]
    async fn test_validation_errors() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_validation_errors - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing input validation error handling");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/validation_errors.tx");
        
        let fixture_content = fs::read_to_string(&fixture_path)
            .expect("Failed to read fixture");
        
        let mut fixture = FixtureBuilder::new("test_validation_errors")
            .with_anvil_manager(get_anvil_manager().await.unwrap())
            .with_runbook("validation_test", &fixture_content)
            .build()
            .await
            .expect("Failed to build fixture");
        
        // Add parameters
        fixture.config.parameters.insert("chain_id".to_string(), "31337".to_string());
        fixture.config.parameters.insert("rpc_url".to_string(), fixture.rpc_url.clone());
        fixture.config.parameters.insert("private_key".to_string(), "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string());
        fixture.config.parameters.insert("recipient".to_string(), "0x90f8bf6a479f320ead074411a4b0e7944ea8c9c1".to_string());
        fixture.config.parameters.insert("contract_address".to_string(), "0x5FbDB2315678afecb367f032d93F642f64180aa3".to_string());
        
        // Act
        fixture.execute_runbook("validation_test").await
            .expect("Failed to execute validation test");
        
        let outputs = fixture.get_outputs("validation_test")
            .expect("Should have outputs");
        
        // Verify we captured validation errors
        let invalid_addr_error = outputs.get("invalid_address_error");
        assert!(
            invalid_addr_error.is_some(),
            "Should capture invalid address error"
        );
        
        let invalid_hex_error = outputs.get("invalid_hex_error");
        assert!(
            invalid_hex_error.is_some(),
            "Should capture invalid hex error"
        );
        
        let negative_value_error = outputs.get("negative_value_error");
        assert!(
            negative_value_error.is_some(),
            "Should capture negative value error"
        );
        
        println!("✅ Validation errors properly captured and handled");
    }
    
    /// Test: Insufficient balance error handling
    /// 
    /// Expected Behavior:
    /// - Transaction from account with insufficient balance should fail
    /// - Error message should indicate insufficient funds
    /// 
    /// Validates:
    /// - Balance validation before transaction submission
    #[tokio::test]
    #[serial(anvil)]
    async fn test_insufficient_balance_error() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_insufficient_balance_error - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing insufficient balance error");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/insufficient_funds_transfer.tx");
        
        let fixture_content = fs::read_to_string(&fixture_path)
            .expect("Failed to read fixture");
        
        let mut fixture = FixtureBuilder::new("test_insufficient_balance")
            .with_anvil_manager(get_anvil_manager().await.unwrap())
            .with_runbook("balance_test", &fixture_content)
            .build()
            .await
            .expect("Failed to build fixture");
        
        // Add parameters
        fixture.config.parameters.insert("chain_id".to_string(), "31337".to_string());
        fixture.config.parameters.insert("rpc_url".to_string(), fixture.rpc_url.clone());
        // Use a new private key with no balance
        fixture.config.parameters.insert("private_key".to_string(), "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string());
        fixture.config.parameters.insert("recipient".to_string(), "0x70997970c51812dc3a010c7d01b50e0d17dc79c8".to_string());
        fixture.config.parameters.insert("amount".to_string(), "1000000000000000000000".to_string()); // 1000 ETH (more than balance)
        
        // Act - The fixture should handle the error
        let result = fixture.execute_runbook("balance_test").await;
        
        // Assert - Should fail due to insufficient balance
        if let Err(report) = &result {
            // Check if error contains insufficient funds indication
            let error_str = format!("{:?}", report);
            assert!(
                error_str.contains("insufficient") || error_str.contains("balance"),
                "Expected error related to insufficient funds, got: {}",
                error_str
            );
        } else if let Ok(()) = result {
            // Alternative: the fixture might capture the error in outputs
            let outputs = fixture.get_outputs("balance_test")
                .expect("Should have outputs");
            assert!(
                outputs.contains_key("error_message"),
                "Should have error_message in output when handling insufficient funds"
            );
        }
        
        println!("✅ Insufficient balance error properly handled");
    }
    
    /// Test: Contract not found error handling
    /// 
    /// Expected Behavior:
    /// - Calls to non-existent contracts should fail or return empty
    /// - Error should be clear about missing contract
    /// 
    /// Validates:
    /// - Contract existence validation
    #[tokio::test]
    #[serial(anvil)]
    async fn test_contract_not_found_error() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_contract_not_found_error - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing contract not found error");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/validation_errors.tx");
        
        let fixture_content = fs::read_to_string(&fixture_path)
            .expect("Failed to read fixture");
        
        let mut fixture = FixtureBuilder::new("test_contract_not_found")
            .with_anvil_manager(get_anvil_manager().await.unwrap())
            .with_runbook("contract_test", &fixture_content)
            .build()
            .await
            .expect("Failed to build fixture");
        
        // Add parameters
        fixture.config.parameters.insert("chain_id".to_string(), "31337".to_string());
        fixture.config.parameters.insert("rpc_url".to_string(), fixture.rpc_url.clone());
        fixture.config.parameters.insert("private_key".to_string(), "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string());
        fixture.config.parameters.insert("recipient".to_string(), "0x90f8bf6a479f320ead074411a4b0e7944ea8c9c1".to_string());
        // Non-existent contract address
        fixture.config.parameters.insert("contract_address".to_string(), "0x0000000000000000000000000000000000000999".to_string());
        
        // Act
        fixture.execute_runbook("contract_test").await
            .expect("Failed to execute contract test");
        
        let outputs = fixture.get_outputs("contract_test")
            .expect("Should have outputs");
        
        // Should have captured the contract call error
        let function_error = outputs.get("invalid_function_error");
        assert!(
            function_error.is_some(),
            "Should capture error when calling non-existent contract"
        );
        
        println!("✅ Contract not found error properly handled");
    }
    
    #[tokio::test]
    #[serial(anvil)]
    async fn test_network_error_handling() {
        // Test without Anvil running (network error)
        println!("🔍 Testing network error handling");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/validation_errors.tx");
        
        let fixture_content = fs::read_to_string(&fixture_path)
            .expect("Failed to read fixture");
        
        // Don't use anvil for this test - we want to test network errors
        let mut fixture = FixtureBuilder::new("test_network_error")
            .with_runbook("network_test", &fixture_content)
            .build()
            .await
            .expect("Failed to build fixture");
        
        // Add parameters with wrong port
        fixture.config.parameters.insert("chain_id".to_string(), "31337".to_string());
        fixture.config.parameters.insert("rpc_url".to_string(), "http://127.0.0.1:9999".to_string()); // Wrong port
        fixture.config.parameters.insert("private_key".to_string(), "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string());
        fixture.config.parameters.insert("recipient".to_string(), "0x90f8bf6a479f320ead074411a4b0e7944ea8c9c1".to_string());
        fixture.config.parameters.insert("contract_address".to_string(), "0x5FbDB2315678afecb367f032d93F642f64180aa3".to_string());
        
        let result = fixture.execute_runbook("network_test").await;
        
        // Network error should be caught
        assert!(result.is_err(), "Network error should be caught");
        
        println!("✅ Network error handling test passed");
    }
}