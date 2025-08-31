//! Test view/pure function detection and handling

#[cfg(test)]
mod view_function_tests {
    use crate::tests::test_harness::ProjectTestHarness;
    use crate::tests::integration::anvil_harness::AnvilInstance;

    #[test]
    fn test_view_function_call_without_gas() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test - Anvil not installed");
            return;
        }

        println!("🔍 Testing view function calls without gas fees");

        // Create test harness using the fixture
        let harness = ProjectTestHarness::new_foundry_from_fixture("integration/test_view_function.tx")
            .with_anvil()
            .with_input("caller_private_key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80");

        // Execute the runbook
        match harness.execute_runbook() {
            Ok(result) => {
                assert!(result.success, "Runbook execution failed");
                println!("View function call succeeded without gas fees");
                
                // Check that we got a result
                if let Some(view_result) = result.outputs.get("view_result") {
                    println!("   View function returned: {:?}", view_result);
                } else {
                    panic!("No view_result output found");
                }
            }
            Err(e) => {
                panic!("Test failed: {}", e);
            }
        }
    }

    #[test]
    fn test_state_changing_function_requires_gas() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test - Anvil not installed");
            return;
        }

        println!("⛽ Testing state-changing functions require gas");

        use std::path::PathBuf;
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/view_functions/state_changing_function.tx");

        let harness = ProjectTestHarness::from_fixture(&fixture_path)
            .with_anvil()
            .with_input("caller_private_key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80");

        match harness.execute_runbook() {
            Ok(result) => {
                assert!(result.success, "Runbook execution failed");
                println!("State-changing function executed with gas");
                
                // Verify we got a transaction hash (meaning it was sent as a transaction)
                if let Some(tx_hash) = result.outputs.get("tx_hash") {
                    println!("   Transaction hash: {:?}", tx_hash);
                } else {
                    panic!("No transaction hash found for state-changing function");
                }
            }
            Err(e) => {
                panic!("Test failed: {}", e);
            }
        }
    }
}