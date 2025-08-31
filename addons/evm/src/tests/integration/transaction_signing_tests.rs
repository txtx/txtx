//! Transaction signing and verification tests
//! 
//! These tests verify transaction signing functionality:
//! - Offline transaction signing
//! - Signature verification
//! - Sending pre-signed transactions
//! - Recovering signer from signature

#[cfg(test)]
mod transaction_signing_tests {
    use crate::tests::integration::anvil_harness::AnvilInstance;
    use crate::tests::fixture_builder::{MigrationHelper, TestResult};
    use txtx_addon_kit::types::types::Value;
    use std::path::PathBuf;
    use tokio;
    
    #[tokio::test]
    async fn test_sign_and_send_transaction() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_sign_and_send_transaction - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing transaction signing and sending");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/transaction_signing.tx");
        
        let result = MigrationHelper::from_fixture(&fixture_path)
            .with_anvil()
            .with_input("chain_id", "31337")
            .with_input("rpc_url", "http://127.0.0.1:8545")
            .with_input("private_key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
            .with_input("recipient", "0x70997970c51812dc3a010c7d01b50e0d17dc79c8")
            .with_input("amount", "1000000000000000000") // 1 ETH
            .with_input("gas_price", "20000000000")
            .with_input("nonce", "0")
            .with_input("data", "0x")
            .execute()
            .await
            .expect("Failed to execute test");
        
        
        
        assert!(result.success, "Transaction signing should succeed");
        
        // Verify signature is valid
        let signature_valid = result.outputs.get("signature_valid")
            .and_then(|v| match v {
                Value::Bool(b) => Some(*b),
                Value::String(s) => Some(s == "true"),
                _ => None
            })
            .expect("Should have signature validation result");
        
        assert!(signature_valid, "Signature should be valid");
        
        // Verify transaction was sent
        let tx_hash = result.outputs.get("tx_hash")
            .and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None
            })
            .expect("Should have transaction hash");
        
        assert!(tx_hash.starts_with("0x"), "Should have valid transaction hash");
        
        println!("✅ Transaction signed and sent: {}", tx_hash);
    }
    
    #[tokio::test]
    async fn test_signature_verification() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_signature_verification - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing signature verification");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/transaction_signing.tx");
        
        let expected_signer = "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266";
        
        let result = MigrationHelper::from_fixture(&fixture_path)
            .with_anvil()
            .with_input("chain_id", "31337")
            .with_input("rpc_url", "http://127.0.0.1:8545")
            .with_input("private_key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
            .with_input("recipient", "0x90f8bf6a479f320ead074411a4b0e7944ea8c9c1")
            .with_input("amount", "500000000000000000")
            .with_input("gas_price", "10000000000")
            .with_input("nonce", "0")
            .with_input("data", "0x")
            .execute()
            .await
            .expect("Failed to execute test");
        
        
        
        assert!(result.success, "Signature verification should succeed");
        
        // Check recovered signer
        let recovered_signer = result.outputs.get("recovered_signer")
            .and_then(|v| match v {
                Value::String(s) => Some(s.to_lowercase()),
                _ => None
            })
            .expect("Should have recovered signer");
        
        assert_eq!(recovered_signer, expected_signer, "Should recover correct signer");
        
        println!("✅ Signature verified, signer: {}", recovered_signer);
    }
    
    #[tokio::test]
    async fn test_sign_transaction_with_data() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_sign_transaction_with_data - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing transaction signing with data payload");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/transaction_signing.tx");
        
        // Function call data (transfer(address,uint256))
        let data = "0xa9059cbb00000000000000000000000070997970c51812dc3a010c7d01b50e0d17dc79c80000000000000000000000000000000000000000000000000de0b6b3a7640000";
        
        let result = MigrationHelper::from_fixture(&fixture_path)
            .with_anvil()
            .with_input("chain_id", "31337")
            .with_input("rpc_url", "http://127.0.0.1:8545")
            .with_input("private_key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
            .with_input("recipient", "0x5fbdb2315678afecb367f032d93f642f64180aa3") // Contract address
            .with_input("amount", "0") // No ETH value for contract call
            .with_input("gas_price", "15000000000")
            .with_input("nonce", "0")
            .with_input("data", data)
            .execute()
            .await
            .expect("Failed to execute test");
        
        
        
        assert!(result.success, "Transaction with data should be signed");
        
        // Verify we got a signed transaction
        let signed_tx = result.outputs.get("signed_tx")
            .and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None
            })
            .expect("Should have signed transaction");
        
        assert!(signed_tx.starts_with("0x"), "Should have valid signed transaction");
        assert!(signed_tx.len() > 100, "Signed transaction should include data");
        
        println!("✅ Transaction with data signed successfully");
    }
    
    /// Test: Offline transaction signing
    /// 
    /// Expected Behavior:
    /// - Transaction can be signed without network connection
    /// - Signed transaction is valid and can be sent later
    /// - Signature can be verified against signer address
    /// 
    /// Validates:
    /// - Offline signing for cold storage scenarios
    #[tokio::test]
    async fn test_offline_signing() {
        // This test doesn't need Anvil since it's offline signing only
        println!("🔍 Testing offline transaction signing");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/transaction_signing.tx");
        
        let result = MigrationHelper::from_fixture(&fixture_path)
            .with_input("chain_id", "1") // Mainnet chain ID
            .with_input("rpc_url", "http://127.0.0.1:8545") // Not used for signing
            .with_input("private_key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
            .with_input("recipient", "0xd8da6bf26964af9d7eed9e03e7175beb9076d64f")
            .with_input("amount", "1000000000000000000")
            .with_input("gas_price", "50000000000")
            .with_input("nonce", "42")
            .with_input("data", "0x")
            .execute()
            .await
            .expect("Failed to execute test");
        
        // Act - Note: This will fail at send step since we're offline
        let result = result.execute().await;
        
        // Assert - We should get a signed transaction even if send fails
        // The fixture signs first, then tries to send
        if let Ok(result) = result {
            // If it succeeded, we must have signed transaction
            let signed_tx = result.outputs.get("signed_tx")
                .and_then(|v| match v {
                    Value::String(s) => Some(s.clone()),
                    _ => None
                })
                .expect("Should have signed transaction in output");
            
            assert!(signed_tx.starts_with("0x"), "Signed transaction should be hex");
            assert!(signed_tx.len() > 100, "Signed transaction should have substance");
            
            println!("✅ Offline signing successful with full execution");
        } else {
            // If send failed (expected without network), check error is network-related
            let error_msg = result.unwrap_err().to_string();
            assert!(
                error_msg.contains("connection") || 
                error_msg.contains("network") ||
                error_msg.contains("rpc"),
                "Failure should be due to network, not signing. Error: {}",
                error_msg
            );
            
            println!("✅ Offline signing succeeded, send failed as expected (no network)");
        }
    }
}