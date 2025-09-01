//! Simple test to verify migration from ProjectTestHarness to FixtureBuilder works

#[cfg(test)]
mod tests {
    use crate::tests::fixture_builder::FixtureBuilder;
    
    #[tokio::test]
    async fn test_simple_fixture() {
        eprintln!("🔍 Testing simple fixture builder");
        
        // Create a simple runbook
        let runbook = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

output "test_value" {
    value = "hello from fixture builder"
}

output "chain_id" {
    value = input.chain_id
}
"#;
        
        // Build fixture
        let mut fixture = FixtureBuilder::new("simple_test")
            .with_runbook("main", runbook)
            .build()
            .await
            .expect("Failed to build fixture");
        
        // Execute runbook
        fixture.execute_runbook("main").await
            .expect("Failed to execute runbook");
        
        // Check outputs
        let outputs = fixture.get_outputs("main")
            .expect("Should have outputs");
        
        assert!(outputs.contains_key("test_value"), "Should have test_value output");
        assert!(outputs.contains_key("chain_id"), "Should have chain_id output");
        
        eprintln!("✅ Simple fixture test passed");
    }
}