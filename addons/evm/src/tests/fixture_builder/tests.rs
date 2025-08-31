// Tests for the fixture builder system

#[cfg(test)]
mod tests {
    use super::super::*;
    
    #[tokio::test]
    async fn test_fixture_builder_basic() {
        // Create a simple fixture
        let fixture = FixtureBuilder::new("test_basic")
            .with_environment("testing")
            .with_confirmations(0)
            .build()
            .await
            .expect("Failed to build fixture");
        
        // Check that the project was created
        assert!(fixture.project_dir.exists());
        assert!(fixture.project_dir.join("txtx.yml").exists());
        assert!(fixture.project_dir.join("runbooks").exists());
        assert!(fixture.project_dir.join("runs/testing").exists());
        
        // Check that accounts are available (case-insensitive comparison)
        assert_eq!(
            fixture.anvil_handle.accounts().alice.address_string().to_lowercase(),
            "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266".to_lowercase()
        );
    }
    
    #[tokio::test]
    async fn test_runbook_with_auto_outputs() {
        let runbook = r#"
addon "evm" {
    chain_id = 31337
    rpc_api_url = input.rpc_url
}

signer "alice" "evm::secret_key" {
    secret_key = input.alice_secret
}

action "transfer" "evm::send_eth" {
    from = input.alice_address
    to = input.bob_address
    amount = "1000000000000000000"
    signer = signer.alice
}
"#;
        
        let fixture = FixtureBuilder::new("test_transfer")
            .with_runbook("transfer", runbook)
            .build()
            .await
            .expect("Failed to build fixture");
        
        // Check that runbook was created with outputs
        let runbook_path = fixture.project_dir.join("runbooks/transfer.tx");
        assert!(runbook_path.exists());
        
        let content = std::fs::read_to_string(runbook_path).unwrap();
        
        // Check that outputs were injected
        assert!(content.contains("transfer_result"), "Should contain transfer_result");
        assert!(content.contains("test_output"), "Should contain test_output");
        assert!(content.contains("test_metadata"), "Should contain test_metadata");
        assert!(content.contains("action.transfer.result"), "Should contain action.transfer.result");
    }
    
    #[tokio::test]
    async fn test_anvil_snapshot_revert() {
        let manager = get_anvil_manager().await.unwrap();
        let mut manager_guard = manager.lock().await;
        
        // Take a snapshot
        let snapshot1 = manager_guard.snapshot("test_snapshot_1").await.unwrap();
        
        // Mine some blocks
        manager_guard.mine_blocks(10).await.unwrap();
        
        // Take another snapshot
        let snapshot2 = manager_guard.snapshot("test_snapshot_2").await.unwrap();
        
        // Revert to first snapshot
        manager_guard.revert(&snapshot1).await.unwrap();
        
        // Second snapshot should be cleaned up
        assert!(!manager_guard.has_snapshot("test_snapshot_2"));
    }
    
    #[tokio::test]
    async fn test_named_accounts() {
        let accounts = NamedAccounts::from_anvil().unwrap();
        
        // Check all 26 accounts exist
        for name in accounts.names() {
            assert!(accounts.get(name).is_some(), "Account {} not found", name);
        }
        
        // Check that accounts can be converted to inputs
        let inputs = accounts.subset_as_inputs(&["alice", "bob", "charlie"]);
        
        assert!(inputs.contains_key("alice_address"));
        assert!(inputs.contains_key("alice_secret"));
        assert!(inputs.contains_key("bob_address"));
        assert!(inputs.contains_key("bob_secret"));
        assert!(inputs.contains_key("charlie_address"));
        assert!(inputs.contains_key("charlie_secret"));
    }
    
    #[tokio::test]
    async fn test_multiple_fixtures_with_isolation() {
        let manager = get_anvil_manager().await.unwrap();
        
        // Create first fixture
        let mut fixture1 = FixtureBuilder::new("test_isolation_1")
            .with_anvil_manager(manager.clone())
            .build()
            .await
            .unwrap();
        
        // Create second fixture
        let mut fixture2 = FixtureBuilder::new("test_isolation_2")
            .with_anvil_manager(manager.clone())
            .build()
            .await
            .unwrap();
        
        // Each fixture should have its own snapshot
        assert_ne!(fixture1.anvil_handle.snapshot_id, fixture2.anvil_handle.snapshot_id);
        
        // Take checkpoints in each
        let checkpoint1 = fixture1.checkpoint().await.unwrap();
        let checkpoint2 = fixture2.checkpoint().await.unwrap();
        
        assert_ne!(checkpoint1, checkpoint2);
    }

    #[test]
    fn test_runbook_parser() {
        let content = r#"
action "deploy_contract" "evm::deploy_contract" {
  description = "Deploy a test contract"
  contract = "0x1234"
}

action "call_function" "evm::call_contract_function" {
  function = "transfer"
}
"#;

        let parser = crate::tests::fixture_builder::runbook_parser::RunbookParser::new(content.to_string());
        let actions = parser.parse_actions().expect("Failed to parse actions");

        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].name, "deploy_contract");
        assert_eq!(actions[0].action_type, "evm::deploy_contract");
        assert_eq!(actions[0].description, "Deploy a test contract");

        assert_eq!(actions[1].name, "call_function");
        assert_eq!(actions[1].action_type, "evm::call_contract_function");

        let outputs = parser.generate_outputs(&actions);
        assert!(outputs.contains("output \"deploy_contract_result\""));
        assert!(outputs.contains("output \"call_function_result\""));
        assert!(outputs.contains("output \"test_output\""));
        assert!(outputs.contains("output \"test_metadata\""));
    }
}