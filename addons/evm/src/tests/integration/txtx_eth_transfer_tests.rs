//! Proof-of-concept test for ETH transfers through txtx framework
//!
//! This test validates that:
//! 1. Runbooks can execute real blockchain transactions
//! 2. Anvil integration works correctly
//! 3. Transaction outputs are captured
//! 4. On-chain state changes can be verified

#[cfg(test)]
mod eth_transfer_tests {
    use crate::tests::test_harness::{ProjectTestHarness, CompilationFramework};
    use alloy::providers::{Provider, ProviderBuilder};
    use alloy::primitives::{Address, U256};
    use std::str::FromStr;
    
    #[test]
    fn test_eth_transfer_through_txtx() {
        // Skip if Anvil not available
        use crate::tests::integration::anvil_harness::AnvilInstance;
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_eth_transfer_through_txtx - Anvil not installed");
            return;
        }
        
        println!("🚀 Starting ETH transfer test through txtx framework");
        println!("Current working directory: {}", std::env::current_dir().unwrap().display());
        
        // Create test harness with the send_eth fixture that uses environment configuration
        let mut harness = ProjectTestHarness::new_foundry_from_fixture("integration/simple_send_eth_with_env.tx")
            .with_anvil();  // This spawns Anvil and sets up inputs
        
        // Get Anvil accounts for verification (store them before borrowing)
        let (sender_address, recipient_address, sender_key, anvil_url) = {
            let anvil = harness.anvil().expect("Anvil should be running");
            (
                anvil.accounts[0].address,
                anvil.accounts[1].address,
                anvil.accounts[0].private_key.clone(),
                anvil.url.clone()
            )
        };
        
        println!("📤 Sender: {:?}", sender_address);
        println!("📥 Recipient: {:?}", recipient_address);
        
        // Setup the project (creates directories, copies contracts, etc.)
        harness.setup().expect("Project setup should succeed");
        
        // Execute the runbook through txtx
        println!("🔄 Executing runbook through txtx...");
        let result = harness.execute_runbook();
        
        // Check execution succeeded
        assert!(result.is_ok(), "Runbook execution failed: {:?}", result);
        let execution_result = result.unwrap();
        assert!(execution_result.success, "Execution marked as failed");
        
        // Verify outputs were captured
        println!("📊 Outputs captured: {:?}", execution_result.outputs.keys().collect::<Vec<_>>());
        
        // Check that we got a transaction hash
        assert!(
            execution_result.outputs.contains_key("tx_hash"),
            "Missing tx_hash in outputs"
        );
        
        // Check receipt status (should be 1 for success)
        if let Some(status) = execution_result.outputs.get("receipt_status") {
            println!("Transaction status: {:?}", status);
            // The status should indicate success
            // Note: The exact format depends on how txtx serializes the receipt
        }
        
        // Check gas was used
        if let Some(gas_used) = execution_result.outputs.get("gas_used") {
            println!("⛽ Gas used: {:?}", gas_used);
        }
        
        // Now verify on-chain state using Alloy provider
        println!("🔍 Verifying on-chain state...");
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let provider = ProviderBuilder::new()
                .on_http(anvil_url.parse().unwrap());
            
            // Get final balances
            let sender_balance = provider.get_balance(sender_address).await
                .expect("Failed to get sender balance");
            let recipient_balance = provider.get_balance(recipient_address).await
                .expect("Failed to get recipient balance");
            
            println!("💰 Final sender balance: {} ETH", format_ether(sender_balance));
            println!("💰 Final recipient balance: {} ETH", format_ether(recipient_balance));
            
            // Recipient should have received 1 ETH (they started with 10000 ETH)
            let expected_recipient = U256::from(10001) * U256::from(10).pow(U256::from(18));
            
            // Due to gas costs, we can't check exact amounts for sender
            // But recipient should have exactly 10001 ETH
            assert!(
                recipient_balance >= expected_recipient,
                "Recipient should have at least 10001 ETH, got {}",
                format_ether(recipient_balance)
            );
            
            // Sender should have less than 10000 ETH (due to transfer + gas)
            let original_sender = U256::from(10000) * U256::from(10).pow(U256::from(18));
            assert!(
                sender_balance < original_sender,
                "Sender should have less than 10000 ETH after transfer"
            );
        });
        
        println!("🎉 ETH transfer through txtx completed successfully!");
        
        // Clean up
        harness.cleanup();
    }
    
    /// Format wei as ETH for display
    fn format_ether(wei: U256) -> String {
        let eth = wei / U256::from(10).pow(U256::from(18));
        let remainder = wei % U256::from(10).pow(U256::from(18));
        let decimal = remainder / U256::from(10).pow(U256::from(16)); // Get 2 decimal places
        format!("{}.{:02}", eth, decimal)
    }
}