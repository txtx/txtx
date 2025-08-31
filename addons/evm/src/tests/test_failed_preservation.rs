//! Test to verify temp directory preservation on failure

#[cfg(test)]
mod test_preservation {
    use crate::tests::test_harness::ProjectTestHarness;
    
    #[test]
    #[ignore] // Run manually with: cargo test test_failed_preservation -- --ignored --nocapture
    fn test_temp_dir_preserved_on_failure() {
        println!("🧪 Testing temp directory preservation on failure...");
        
        // Create a deliberately broken runbook (missing required field)
        let broken_runbook = r#"
addon "evm" {
    chain_id = 31337
    rpc_api_url = "http://localhost:8545"
}

signer "test_signer" "evm::secret_key" {
    secret_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
}

# This action will fail because 'recipient_address' is missing
action "broken" "evm::send_eth" {
    amount = 1000000000000000000
    signer = signer.test_signer
    # recipient_address is missing - this will cause an error
}
"#;
        
        let harness = ProjectTestHarness::new_foundry("broken_test.tx", broken_runbook.to_string());
        
        println!("📁 Created test in: {}", harness.project_path.display());
        
        // Setup should work
        match harness.setup() {
            Ok(_) => println!("Setup succeeded"),
            Err(e) => {
                println!("Setup failed: {}", e);
                return;
            }
        }
        
        // Execution should fail
        match harness.execute_runbook() {
            Ok(_) => {
                println!("Unexpected: Execution succeeded when it should have failed!");
                panic!("Test should have failed but didn't");
            }
            Err(e) => {
                println!("Expected failure: {}", e);
                println!("📂 Check if temp directory was preserved...");
                // The Drop trait should now preserve the directory
            }
        }
        
        // When harness goes out of scope, Drop should preserve the directory
        println!("🔍 Test complete - check console output for preserved directory path");
    }
}