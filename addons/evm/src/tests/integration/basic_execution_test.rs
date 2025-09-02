
#[cfg(test)]
mod basic_tests {
    use crate::tests::integration::anvil_harness::AnvilInstance;
    use crate::tests::fixture_builder::{FixtureBuilder, get_anvil_manager};
    use serial_test::serial;
    use tokio;
    
    #[tokio::test]
    #[serial(anvil)]
    async fn test_minimal_runbook_execution() {
        eprintln!("🔍 TEST STARTING - test_minimal_runbook_execution");
        
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test - Anvil not installed");
            return;
        }
        
        // Create the simplest possible runbook - just outputs, no actions
        let minimal_runbook = r#"
# Minimal test runbook
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

output "test_value" {
    value = "hello"
}

output "chain_id_output" {
    value = input.chain_id
}
"#;
        
        eprintln!("📋 Creating test harness with minimal runbook");
        let mut fixture = FixtureBuilder::new("test_minimal")
            .with_anvil_manager(get_anvil_manager().await.unwrap())
            .with_runbook("minimal", minimal_runbook)
            .build()
            .await
            .expect("Failed to build fixture");
        
        // Setup the project
        eprintln!("📋 Setting up project in: {}", fixture.project_dir.display());
        
        // Add parameters
        fixture.config.parameters.insert("chain_id".to_string(), "31337".to_string());
        fixture.config.parameters.insert("rpc_url".to_string(), fixture.rpc_url.clone());
        
        // Execute the runbook
        fixture.execute_runbook("minimal").await
            .expect("Failed to execute minimal runbook");
        
        // For now, just verify setup works
        eprintln!("✅ Project setup completed successfully");
        
        // List files created - runbooks are now in directories with main.tx
        let runbook_dir = fixture.project_dir.join("runbooks").join("minimal");
        assert!(runbook_dir.exists(), "Runbook directory should exist");
        
        let main_file = runbook_dir.join("main.tx");
        assert!(main_file.exists(), "main.tx file should exist in runbook directory");
        
        let config_path = fixture.project_dir.join("txtx.yml");
        assert!(config_path.exists(), "Config file should exist");
        
        eprintln!("✅ Test completed - project structure verified");
    }
}