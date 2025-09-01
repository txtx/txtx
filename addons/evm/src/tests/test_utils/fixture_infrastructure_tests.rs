// Infrastructure tests for FixtureBuilder
// These verify the test infrastructure works, not EVM functionality

#[cfg(test)]
mod fixture_infrastructure_tests {
    use crate::tests::fixture_builder::*;
    
    #[tokio::test]
    async fn test_fixture_creates_required_directories() {
        // ARRANGE: Set up test parameters
        let test_name = "infrastructure_test";
        
        // ACT: Create a fixture
        let fixture = FixtureBuilder::new(test_name)
            .with_environment("testing")
            .build()
            .await
            .expect("Failed to build fixture");
        
        // ASSERT: Verify infrastructure was created correctly
        assert!(fixture.project_dir.exists(), "Project directory should exist");
        assert!(fixture.project_dir.join("txtx.yml").exists(), "txtx.yml should exist");
        assert!(fixture.project_dir.join("runbooks").exists(), "runbooks directory should exist");
        assert!(fixture.project_dir.join("runs/testing").exists(), "runs/testing directory should exist");
    }
    
    #[tokio::test]
    async fn test_fixture_provides_anvil_connection() {
        // ARRANGE: Create fixture name
        let test_name = "anvil_connection_test";
        
        // ACT: Build fixture with Anvil
        let fixture = FixtureBuilder::new(test_name)
            .build()
            .await
            .expect("Failed to build fixture");
        
        // ASSERT: Verify Anvil connection details
        assert!(!fixture.rpc_url.is_empty(), "RPC URL should be set");
        assert!(fixture.rpc_url.starts_with("http://"), "RPC URL should be HTTP");
        
        let accounts = fixture.anvil_handle.accounts();
        assert_eq!(accounts.names().len(), 26, "Should have 26 named accounts (alice-zed)");
        assert!(accounts.alice.address_string().starts_with("0x"), "Address should be hex");
    }
    
    #[tokio::test] 
    async fn test_fixture_parameter_substitution() {
        // ARRANGE: Set up parameters
        let mut fixture = FixtureBuilder::new("param_test")
            .with_parameter("test_key", "test_value")
            .with_parameter("chain_id", "31337")
            .build()
            .await
            .expect("Failed to build fixture");
        
        // ACT: Add a runbook that uses parameters
        let runbook = r#"
addon "evm" {
    chain_id = input.chain_id
}
output "param_echo" {
    value = input.test_key
}"#;
        fixture.add_runbook("test", runbook).unwrap();
        
        // ASSERT: Verify parameters are accessible
        // Note: This tests infrastructure, actual parameter substitution 
        // would be tested in integration tests
        assert_eq!(fixture.config.parameters.get("test_key"), Some(&"test_value".to_string()));
        assert_eq!(fixture.config.parameters.get("chain_id"), Some(&"31337".to_string()));
    }
}