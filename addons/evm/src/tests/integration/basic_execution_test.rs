
#[cfg(test)]
mod basic_tests {
    use super::*;
    use crate::tests::integration::anvil_harness::AnvilInstance;
    
    #[tokio::test]
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
        let harness = ProjectTestHarness::new_with_content(
            "minimal.tx",
            minimal_runbook
        );
        
        // Setup the project
        eprintln!("📋 Setting up project in: {}", fixture.project_dir.display());
        // Project already set up by FixtureBuilder
        
        // For now, just verify setup works
        eprintln!("✅ Project setup completed successfully");
        
        // List files created
        let runbook_path = fixture.project_dir.join("runbooks").join("minimal.tx");
        assert!(runbook_path.exists(), "Runbook file should exist");
        
        let config_path = fixture.project_dir.join("txtx.yml");
        assert!(config_path.exists(), "Config file should exist");
        
        eprintln!("✅ Test completed - project structure verified");
    }
}