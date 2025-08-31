
#[cfg(test)]
mod debug_tests {
    use crate::tests::fixture_builder::MigrationHelper;
    use super::*;
    use crate::tests::integration::anvil_harness::AnvilInstance;

    #[tokio::test]
    async fn test_simple_unsupervised_execution() {
        eprintln!("🔍 TEST STARTING - test_simple_unsupervised_execution");
        
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test - Anvil not installed");
            return;
        }
        
        eprintln!("🚀 Starting simple unsupervised execution test");
        
        // Create a minimal runbook that should execute quickly
        let minimal_runbook = r#"
# Minimal test runbook - no actions, just outputs
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

output "test_output" {
    value = "Hello from unsupervised mode"
}

output "chain_id" {
    value = input.chain_id
}
"#;
        
        // Create harness with the minimal runbook
        let mut harness = ProjectTestHarness::new_with_content(
            "minimal_test.tx",
            minimal_runbook
        );
        
        // Setup the project
        // Project already set up by FixtureBuilder
        
        eprintln!("📋 Executing minimal runbook...");
        
        // Execute directly without threading for now
        let execution_result = result.execute().await;
        
        match execution_result {
            Ok(result) => {
                eprintln!("✅ Execution completed successfully");
                eprintln!("Outputs: {:?}", result.outputs);
                assert!(result.success, "Execution should succeed");
                assert!(result.outputs.contains_key("test_output"), "Should have test_output");
            }
            Err(e) => {
                panic!("❌ Execution failed: {:?}", e);
            }
        }
        
        eprintln!("✅ Test completed successfully");
    }
}