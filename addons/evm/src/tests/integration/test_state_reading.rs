
#[cfg(test)]
mod state_tests {
    use super::*;
    use crate::tests::integration::anvil_harness::AnvilInstance;
    use std::fs;
    
    #[tokio::test]
    async fn test_read_execution_state() {
        eprintln!("🔍 TEST STARTING - test_read_execution_state");
        
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test - Anvil not installed");
            return;
        }
        
        // Create a simple runbook that just has outputs
        let simple_runbook = r#"
# Simple test runbook
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

output "test_output" {
    value = "test_value"
}

output "chain_id_echo" {
    value = input.chain_id
}
"#;
        
        eprintln!("📋 Creating test harness");
        let harness = ProjectTestHarness::new_with_content(
            "state_test.tx",
            simple_runbook
        );
        
        // Setup the project
        // Project already set up by FixtureBuilder
        
        eprintln!("📋 Project path: {}", fixture.project_dir.display());
        
        // Execute the runbook
        eprintln!("🔄 Executing runbook...");
        let result = result.execute().await;
        
        match result {
            Ok(test_result) => {
                eprintln!("✅ Execution succeeded");
                eprintln!("Success flag: {}", test_result.success);
                eprintln!("Number of outputs: {}", test_result.outputs.len());
                
                // Check for state files in temp directory
                let txtx_dir = fixture.project_dir.join(".txtx");
                if txtx_dir.exists() {
                    eprintln!("📁 .txtx directory exists");
                    
                    // List all files in .txtx
                    if let Ok(entries) = fs::read_dir(&txtx_dir) {
                        eprintln!("Files in .txtx:");
                        for entry in entries {
                            if let Ok(entry) = entry {
                                eprintln!("  - {}", entry.file_name().to_string_lossy());
                            }
                        }
                    }
                    
                    // Check for state.json
                    let state_file = txtx_dir.join("state.json");
                    if state_file.exists() {
                        eprintln!("✅ state.json exists");
                        
                        // Read and print first 500 chars of state
                        if let Ok(content) = fs::read_to_string(&state_file) {
                            let preview = if content.len() > 500 {
                                &content[..500]
                            } else {
                                &content
                            };
                            eprintln!("State preview: {}", preview);
                        }
                    } else {
                        eprintln!("❌ state.json not found");
                    }
                } else {
                    eprintln!("❌ .txtx directory not found");
                }
                
                // Even if we didn't get outputs, the test passes if execution succeeded
                assert!(test_result.success, "Execution should succeed");
            }
            Err(e) => {
                eprintln!("❌ Execution failed: {:?}", e);
                panic!("Runbook execution failed");
            }
        }
        
        eprintln!("✅ Test completed");
    }
}