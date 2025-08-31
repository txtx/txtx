// Simple test to verify fixture builder works

#[cfg(test)]
mod tests {
    use super::super::*;
    
    #[tokio::test]
    async fn test_fixture_creation() {
        println!("Creating test fixture...");
        
        // Create a fixture
        let fixture = FixtureBuilder::new("test_simple")
            .with_environment("testing")
            .build()
            .await
            .expect("Failed to build fixture");
        
        println!("Fixture created at: {:?}", fixture.project_dir);
        
        // Verify basic structure was created
        assert!(fixture.project_dir.exists(), "Project directory should exist");
        
        let txtx_yml = fixture.project_dir.join("txtx.yml");
        assert!(txtx_yml.exists(), "txtx.yml should exist");
        
        let runbooks_dir = fixture.project_dir.join("runbooks");
        assert!(runbooks_dir.exists(), "runbooks directory should exist");
        
        println!("✅ Fixture structure verified");
    }
    
    #[tokio::test]
    async fn test_anvil_integration() {
        println!("Testing Anvil integration...");
        
        let fixture = FixtureBuilder::new("test_anvil")
            .build()
            .await
            .expect("Failed to build fixture");
        
        // Check that we have an RPC URL
        assert!(!fixture.rpc_url.is_empty(), "RPC URL should be set");
        println!("RPC URL: {}", fixture.rpc_url);
        
        // Check that we have accounts
        let accounts = fixture.anvil_handle.accounts();
        assert!(accounts.names().len() > 0, "Should have named accounts");
        println!("Available accounts: {:?}", accounts.names());
        
        println!("✅ Anvil integration verified");
    }
}