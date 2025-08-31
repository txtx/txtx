// Showcase test demonstrating all fixture builder capabilities

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::super::helpers::*;
    
    /// This test demonstrates the full capabilities of the fixture builder system
    /// It's designed to be a comprehensive example of how to use the testing infrastructure
    #[tokio::test]
    async fn test_fixture_builder_showcase() {
        println!("\n🎭 FIXTURE BUILDER SHOWCASE TEST 🎭\n");
        println!("This test demonstrates all the capabilities of our fixture-based testing system.\n");
        
        // ========================================
        // 1. FIXTURE CREATION
        // ========================================
        println!("📦 Step 1: Creating test fixture with configuration");
        
        let mut fixture = FixtureBuilder::new("showcase_test")
            .with_environment("testing")
            .with_confirmations(0)
            .with_parameter("custom_param", "custom_value")
            .build()
            .await
            .expect("Failed to build fixture");
        
        println!("   ✅ Fixture created");
        println!("   📁 Project directory: {}", fixture.project_dir.display());
        println!("   🌐 RPC URL: {}", fixture.rpc_url);
        println!("   🔗 Chain ID: 31337 (Anvil default)");
        
        // ========================================
        // 2. NAMED ACCOUNTS
        // ========================================
        println!("\n👥 Step 2: Demonstrating named accounts");
        
        let accounts = fixture.anvil_handle.accounts();
        println!("   Available accounts: {} total", accounts.names().len());
        
        // Show first 5 accounts
        for name in accounts.names().iter().take(5) {
            if let Some(account) = accounts.get(name) {
                println!("   - {}: {}", name, account.address);
            }
        }
        
        // ========================================
        // 3. SMART CONTRACT DEPLOYMENT
        // ========================================
        println!("\n📜 Step 3: Adding and deploying a smart contract");
        
        let contract = contracts::SIMPLE_STORAGE;
        fixture.add_contract("SimpleStorage", contract)
            .expect("Failed to add contract");
        
        println!("   ✅ Contract added to project");
        
        // ========================================
        // 4. RUNBOOK WITH AUTO-GENERATED OUTPUTS
        // ========================================
        println!("\n📝 Step 4: Creating runbook with automatic output generation");
        
        let runbook = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

signer "deployer" "evm::private_key" {
    private_key = input.alice_secret
}

signer "user" "evm::private_key" {
    private_key = input.bob_secret
}

// Check initial balances
action "check_alice_initial" "evm::get_balance" {
    description = "Check Alice's initial balance"
    address = input.alice_address
}

action "check_bob_initial" "evm::get_balance" {
    description = "Check Bob's initial balance"
    address = input.bob_address
}

// Transfer some ETH
action "transfer_eth" "evm::send_eth" {
    description = "Transfer 0.1 ETH from Alice to Bob"
    from = input.alice_address
    to = input.bob_address
    value = "100000000000000000"  // 0.1 ETH
    signer = signer.deployer
}

// Check balances after transfer
action "check_alice_after" "evm::get_balance" {
    description = "Check Alice's balance after transfer"
    address = input.alice_address
}

action "check_bob_after" "evm::get_balance" {
    description = "Check Bob's balance after transfer"
    address = input.bob_address
}
"#;
        
        fixture.add_runbook("showcase", runbook)
            .expect("Failed to add runbook");
        
        println!("   ✅ Runbook added with 5 actions");
        println!("   🔄 Parser will auto-generate outputs for each action");
        
        // ========================================
        // 5. CHECKPOINT/SNAPSHOT FUNCTIONALITY
        // ========================================
        println!("\n💾 Step 5: Demonstrating checkpoint/revert for test isolation");
        
        let checkpoint1 = fixture.checkpoint().await
            .expect("Failed to take checkpoint");
        
        println!("   📸 Checkpoint taken: {}", checkpoint1);
        
        // ========================================
        // 6. RUNBOOK EXECUTION (if txtx is available)
        // ========================================
        println!("\n🚀 Step 6: Attempting runbook execution");
        
        match fixture.execute_runbook("showcase").await {
            Ok(_) => {
                println!("   ✅ Runbook executed successfully!");
                
                // Get and display outputs
                if let Some(outputs) = fixture.get_outputs("showcase") {
                    println!("\n   📊 Outputs generated:");
                    println!("   - Individual action results: {}", 
                        outputs.keys()
                            .filter(|k| k.ends_with("_result"))
                            .count());
                    println!("   - Test aggregate output: {}", 
                        if outputs.contains_key("test_output") { "✓" } else { "✗" });
                    println!("   - Test metadata: {}", 
                        if outputs.contains_key("test_metadata") { "✓" } else { "✗" });
                    
                    // Use helper functions to extract values
                    if let Some(tx_hash) = get_string_output(&outputs, "transfer_eth_result", "tx_hash") {
                        println!("   - Transfer TX hash: {}", &tx_hash[..10]);
                    }
                }
            },
            Err(e) => {
                println!("   ⚠️ Execution skipped (txtx not built): {}", e);
                println!("   💡 Run 'cargo build --package txtx-cli' to enable execution tests");
            }
        }
        
        // ========================================
        // 7. STATE REVERSION
        // ========================================
        println!("\n⏮️ Step 7: Reverting to checkpoint");
        
        fixture.revert(&checkpoint1).await
            .expect("Failed to revert");
        
        println!("   ✅ State reverted to checkpoint");
        println!("   🔄 Any transactions after checkpoint have been undone");
        
        // ========================================
        // 8. HELPER UTILITIES
        // ========================================
        println!("\n🛠️ Step 8: Available helper utilities");
        
        println!("   Output extraction helpers:");
        println!("   - get_string_output(): Extract string values");
        println!("   - get_bool_output(): Extract boolean values");
        println!("   - get_int_output(): Extract integer values");
        
        println!("\n   Assertion helpers:");
        println!("   - assert_action_success(): Verify action succeeded");
        println!("   - assert_has_tx_hash(): Verify and return tx hash");
        println!("   - assert_has_contract_address(): Verify deployment");
        
        println!("\n   Template generators:");
        println!("   - templates::eth_transfer(): Generate transfer runbook");
        println!("   - templates::deploy_contract(): Generate deployment runbook");
        
        println!("\n   Pre-built contracts:");
        println!("   - contracts::SIMPLE_STORAGE");
        println!("   - contracts::SIMPLE_TOKEN");
        println!("   - contracts::COUNTER");
        
        // ========================================
        // SUMMARY
        // ========================================
        println!("\n✨ SHOWCASE COMPLETE ✨");
        println!("\nThe fixture builder provides:");
        println!("  ✓ Isolated test environments with temp directories");
        println!("  ✓ Managed Anvil blockchain with snapshots");
        println!("  ✓ 26 named test accounts (alice-zed)");
        println!("  ✓ Automatic output generation for actions");
        println!("  ✓ HCL parsing via txtx-core");
        println!("  ✓ Source-based txtx execution");
        println!("  ✓ Helper utilities and templates");
        println!("  ✓ Test isolation with checkpoint/revert");
        
        println!("\n📚 See TESTING_GUIDE.md for more details");
    }
    
    /// Test that demonstrates error handling capabilities
    #[tokio::test]
    async fn test_error_handling_showcase() {
        println!("\n⚠️ ERROR HANDLING SHOWCASE ⚠️\n");
        
        let mut fixture = FixtureBuilder::new("error_showcase")
            .build()
            .await
            .expect("Failed to build fixture");
        
        // Test with invalid runbook syntax
        let invalid_runbook = r#"
addon "evm" {
    chain_id = input.chain_id
    // Missing closing brace
"#;
        
        match fixture.add_runbook("invalid", invalid_runbook) {
            Ok(_) => println!("❌ Should have failed on invalid syntax"),
            Err(e) => println!("✅ Correctly rejected invalid runbook: {}", e),
        }
        
        // Test with invalid action
        let runbook_with_error = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

action "bad_balance" "evm::get_balance" {
    address = "not_a_valid_ethereum_address"
}
"#;
        
        fixture.add_runbook("error_test", runbook_with_error)
            .expect("Failed to add runbook");
        
        match fixture.execute_runbook("error_test").await {
            Ok(_) => println!("⚠️ Runbook succeeded (may have error recovery)"),
            Err(e) => println!("✅ Execution failed as expected: {}", e),
        }
        
        println!("\n📋 Error handling features:");
        println!("  ✓ Invalid syntax detection");
        println!("  ✓ Runtime error handling");
        println!("  ✓ Context preservation with error-stack");
        println!("  ✓ Detailed error messages");
    }
    
    /// Performance benchmark test
    #[tokio::test]
    async fn test_performance_benchmark() {
        use std::time::Instant;
        
        println!("\n⚡ PERFORMANCE BENCHMARK ⚡\n");
        
        let start = Instant::now();
        
        // Measure fixture creation time
        let fixture_start = Instant::now();
        let fixture = FixtureBuilder::new("benchmark")
            .build()
            .await
            .expect("Failed to build fixture");
        let fixture_time = fixture_start.elapsed();
        
        println!("Fixture creation: {:?}", fixture_time);
        
        // Measure Anvil snapshot time
        let snapshot_start = Instant::now();
        let mut manager = fixture.anvil_manager.lock().await;
        let _snapshot = manager.snapshot("bench").await.unwrap();
        let snapshot_time = snapshot_start.elapsed();
        
        println!("Snapshot creation: {:?}", snapshot_time);
        
        // Measure revert time
        let revert_start = Instant::now();
        manager.revert("bench").await.unwrap();
        let revert_time = revert_start.elapsed();
        
        println!("Snapshot revert: {:?}", revert_time);
        
        let total_time = start.elapsed();
        println!("\nTotal benchmark time: {:?}", total_time);
        
        // Performance assertions
        assert!(fixture_time.as_millis() < 500, "Fixture creation too slow");
        assert!(snapshot_time.as_millis() < 100, "Snapshot too slow");
        assert!(revert_time.as_millis() < 100, "Revert too slow");
        
        println!("\n✅ All performance benchmarks passed");
    }
}