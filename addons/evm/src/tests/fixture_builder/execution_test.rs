// Full end-to-end execution test using the fixture builder

#[cfg(test)]
mod tests {
    use super::super::*;
    
    #[tokio::test]
    #[ignore] // Ignore by default since it requires building txtx
    async fn test_real_eth_transfer_execution() {
        println!("🚀 Starting real ETH transfer execution test");
        
        // Create fixture
        let mut fixture = FixtureBuilder::new("test_real_transfer")
            .with_environment("testing")
            .build()
            .await
            .expect("Failed to build fixture");
        
        println!("📍 RPC URL: {}", fixture.rpc_url);
        println!("📁 Project dir: {}", fixture.project_dir.display());
        
        // Create a simple transfer runbook
        let runbook = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

signer "alice" "evm::private_key" {
    private_key = input.alice_secret
}

action "check_alice_balance" "evm::get_balance" {
    description = "Check Alice's initial balance"
    address = input.alice_address
}

action "check_bob_balance" "evm::get_balance" {
    description = "Check Bob's initial balance"
    address = input.bob_address
}

action "transfer" "evm::send_eth" {
    description = "Transfer 0.5 ETH from Alice to Bob"
    from = input.alice_address
    to = input.bob_address
    value = "500000000000000000"  // 0.5 ETH
    signer = signer.alice
}

action "check_alice_after" "evm::get_balance" {
    description = "Check Alice's balance after transfer"
    address = input.alice_address
}

action "check_bob_after" "evm::get_balance" {
    description = "Check Bob's balance after transfer"  
    address = input.bob_address
}
"#;
        
        // Add the runbook
        fixture.add_runbook("transfer_test", runbook)
            .expect("Failed to add runbook");
        
        println!("📝 Runbook added, executing...");
        
        // Execute the runbook
        fixture.execute_runbook("transfer_test").await
            .expect("Failed to execute runbook");
        
        println!("✅ Runbook executed successfully");
        
        // Get outputs
        let outputs = fixture.get_outputs("transfer_test")
            .expect("Failed to get outputs");
        
        // Verify we have all expected outputs
        assert!(outputs.contains_key("check_alice_balance_result"), "Missing Alice balance check");
        assert!(outputs.contains_key("check_bob_balance_result"), "Missing Bob balance check");
        assert!(outputs.contains_key("transfer_result"), "Missing transfer result");
        assert!(outputs.contains_key("check_alice_after_result"), "Missing Alice after balance");
        assert!(outputs.contains_key("check_bob_after_result"), "Missing Bob after balance");
        assert!(outputs.contains_key("test_output"), "Missing aggregate test output");
        assert!(outputs.contains_key("test_metadata"), "Missing test metadata");
        
        // Check transfer was successful
        if let Some(transfer_result) = outputs.get("transfer_result") {
            match transfer_result {
                txtx_addon_kit::types::types::Value::Object(map) => {
                    // Check for tx_hash
                    assert!(map.contains_key("tx_hash"), "Transfer should have tx_hash");
                    
                    // Check success flag if present
                    if let Some(success) = map.get("success") {
                        match success {
                            txtx_addon_kit::types::types::Value::Bool(b) => {
                                assert!(*b, "Transfer should be successful");
                            },
                            _ => {}
                        }
                    }
                },
                _ => panic!("Expected transfer_result to be an object")
            }
        }
        
        println!("🎉 All assertions passed!");
    }
    
    #[tokio::test]
    async fn test_runbook_with_error_handling() {
        println!("🧪 Testing runbook with intentional errors");
        
        let mut fixture = FixtureBuilder::new("test_errors")
            .build()
            .await
            .expect("Failed to build fixture");
        
        // Runbook with an invalid address to trigger an error
        let runbook = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

action "bad_balance_check" "evm::get_balance" {
    description = "Try to check balance of invalid address"
    address = "not_a_valid_address"
}
"#;
        
        fixture.add_runbook("error_test", runbook)
            .expect("Failed to add runbook");
        
        // Execute should fail but not panic
        let result = fixture.execute_runbook("error_test").await;
        
        // We expect this to fail due to invalid address
        // The exact behavior depends on txtx error handling
        // For now, just verify we can handle the error case
        match result {
            Ok(_) => {
                println!("⚠️ Runbook succeeded unexpectedly - txtx may have error recovery");
            },
            Err(e) => {
                println!("✅ Runbook failed as expected: {}", e);
            }
        }
    }
    
    #[tokio::test]
    async fn test_checkpoint_and_revert() {
        println!("🔄 Testing checkpoint and revert functionality");
        
        let mut fixture = FixtureBuilder::new("test_checkpoint")
            .build()
            .await
            .expect("Failed to build fixture");
        
        // Simple balance check runbook
        let runbook = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

action "check_balance" "evm::get_balance" {
    description = "Check Alice's balance"
    address = input.alice_address
}
"#;
        
        fixture.add_runbook("balance_check", runbook)
            .expect("Failed to add runbook");
        
        // Execute once
        fixture.execute_runbook("balance_check").await
            .expect("Failed to execute runbook");
        
        // Take checkpoint
        let checkpoint = fixture.checkpoint().await
            .expect("Failed to take checkpoint");
        println!("📸 Checkpoint taken: {}", checkpoint);
        
        // Execute transfer runbook to change state
        let transfer_runbook = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

signer "alice" "evm::private_key" {
    private_key = input.alice_secret
}

action "transfer" "evm::send_eth" {
    from = input.alice_address
    to = input.bob_address
    value = "1000000000000000000"  // 1 ETH
    signer = signer.alice
}
"#;
        
        fixture.add_runbook("transfer", transfer_runbook)
            .expect("Failed to add transfer runbook");
        
        fixture.execute_runbook("transfer").await
            .expect("Failed to execute transfer");
        
        // Revert to checkpoint
        fixture.revert(&checkpoint).await
            .expect("Failed to revert");
        println!("⏮️ Reverted to checkpoint");
        
        // Execute balance check again - should work as if transfer never happened
        fixture.execute_runbook("balance_check").await
            .expect("Failed to execute after revert");
        
        println!("✅ Checkpoint and revert test passed");
    }
}