// Example test demonstrating the complete fixture system

#[cfg(test)]
mod tests {
    use super::super::*;
    use txtx_addon_kit::types::types::Value;
    
    #[tokio::test]
    async fn test_complete_fixture_example() {
        // Create a runbook that sends ETH from alice to bob
        let runbook_content = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
    confirmations = 0
}

variable "amount" {
    value = "1000000000000000000"  # 1 ETH
}

signer "alice" "evm::secret_key" {
    secret_key = input.alice_secret
}

action "send_eth" "evm::send_eth" {
    from = input.alice_address
    to = input.bob_address
    amount = variable.amount
    signer = signer.alice
}

action "check_balance" "evm::get_balance" {
    address = input.bob_address
}
"#;
        
        // Build the fixture
        let mut fixture = FixtureBuilder::new("test_eth_transfer")
            .with_runbook("transfer", runbook_content)
            .with_parameter("chain_id", "31337")
            .build()
            .await
            .expect("Failed to build fixture");
        
        eprintln!("📋 Test fixture created at: {}", fixture.project_dir.display());
        
        // Check that the runbook was created with auto-generated outputs
        let runbook_path = fixture.project_dir.join("runbooks/transfer.tx");
        assert!(runbook_path.exists(), "Runbook should exist");
        
        let runbook_content = std::fs::read_to_string(&runbook_path).unwrap();
        
        // Verify outputs were injected
        assert!(runbook_content.contains("send_eth_result"), "Should have send_eth result");
        assert!(runbook_content.contains("check_balance_result"), "Should have check_balance result");
        assert!(runbook_content.contains("test_output"), "Should have aggregate test output");
        
        // Check that accounts are available
        let alice = fixture.anvil_handle.accounts().alice.clone();
        let bob = fixture.anvil_handle.accounts().bob.clone();
        
        eprintln!("👤 Alice address: {}", alice.address_string());
        eprintln!("👤 Bob address: {}", bob.address_string());
        
        // Take a checkpoint before any transactions
        let checkpoint = fixture.checkpoint().await.expect("Should take checkpoint");
        eprintln!("📸 Checkpoint taken: {}", checkpoint);
        
        // In a real test, we would execute the runbook here:
        // fixture.execute_runbook("transfer").await.expect("Should execute");
        // 
        // Then check outputs:
        // let tx_hash = fixture.get_output("send_eth_output.tx_hash");
        // assert!(tx_hash.is_some());
        
        // For now, just verify the structure is correct
        assert!(fixture.project_dir.join("txtx.yml").exists(), "txtx.yml should exist");
        assert!(fixture.project_dir.join("runs/testing").exists(), "Output directory should exist");
        
        eprintln!("✅ Fixture example test completed successfully");
    }
    
    #[tokio::test] 
    async fn test_snapshot_isolation() {
        // Get the global manager
        let manager = get_anvil_manager().await.expect("Should get manager");
        
        // Create two fixtures that will share the same Anvil instance
        let mut fixture1 = FixtureBuilder::new("test_isolation_1")
            .with_anvil_manager(manager.clone())
            .build()
            .await
            .expect("Should build fixture1");
        
        let mut fixture2 = FixtureBuilder::new("test_isolation_2")
            .with_anvil_manager(manager.clone())
            .build()
            .await
            .expect("Should build fixture2");
        
        // Each fixture should have its own snapshot
        eprintln!("Fixture1 snapshot: {}", fixture1.anvil_handle.snapshot_id);
        eprintln!("Fixture2 snapshot: {}", fixture2.anvil_handle.snapshot_id);
        
        assert_ne!(
            fixture1.anvil_handle.snapshot_id, 
            fixture2.anvil_handle.snapshot_id,
            "Each fixture should have its own snapshot"
        );
        
        // Take additional checkpoints
        let checkpoint1 = fixture1.checkpoint().await.expect("Should checkpoint");
        let checkpoint2 = fixture2.checkpoint().await.expect("Should checkpoint");
        
        assert_ne!(checkpoint1, checkpoint2, "Checkpoints should be different");
        
        eprintln!("✅ Snapshot isolation test completed");
    }
    
    #[tokio::test]
    async fn test_confirmation_mining() {
        let manager = get_anvil_manager().await.expect("Should get manager");
        
        let fixture = FixtureBuilder::new("test_confirmations")
            .with_anvil_manager(manager.clone())
            .with_confirmations(6)  // Set default confirmations
            .build()
            .await
            .expect("Should build fixture");
        
        // Mine blocks for confirmations
        {
            let manager_guard = manager.lock().await;
            manager_guard.mine_blocks(6).await.expect("Should mine blocks");
        }
        
        eprintln!("✅ Mined 6 blocks for confirmations");
        
        // In a real test, we would:
        // 1. Execute a transaction
        // 2. Mine blocks for confirmations
        // 3. Verify the transaction has the expected confirmations
    }
}