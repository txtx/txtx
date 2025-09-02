//! Simplified panic-aware tests that preserve directories on failure

#[cfg(test)]
mod simple_panic_tests {
    use crate::tests::integration::anvil_harness::AnvilInstance;
    use crate::tests::fixture_builder::get_anvil_manager;
    use std::fs;
    use std::path::PathBuf;
    use serial_test::serial;
    use tokio;
    
    /// Simple test helper that preserves directory on failure
    fn create_test_dir(test_name: &str) -> PathBuf {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let dir_name = format!("/tmp/txtx_test_{}_{}", test_name, timestamp);
        let path = PathBuf::from(dir_name);
        fs::create_dir_all(&path).expect("Failed to create test dir");
        
        eprintln!("📁 Test directory: {}", path.display());
        path
    }
    
    /// Test send_eth with correct field names
    #[tokio::test]
    #[serial(anvil)]
    async fn test_send_eth_fixed() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test - Anvil not installed");
            return;
        }
        
        let test_dir = create_test_dir("send_eth_simple");
        let result = run_send_eth_test(&test_dir).await;
        
        match result {
            Ok(_) => {
                // Clean up on success
                let _ = fs::remove_dir_all(&test_dir);
                eprintln!("✅ Test passed - cleaned up directory");
            }
            Err(e) => {
                eprintln!("\n════════════════════════════════════════");
                eprintln!("❌ TEST FAILED: {}", e);
                eprintln!("📁 Directory preserved at:");
                eprintln!("   {}", test_dir.display());
                eprintln!("════════════════════════════════════════\n");
                panic!("Test failed: {}", e);
            }
        }
    }
    
    async fn run_send_eth_test(test_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔍 Testing send_eth with proper fixture setup");
        
        // Create directories
        fs::create_dir_all(test_dir.join("src"))?;
        fs::create_dir_all(test_dir.join("runbooks/send_eth"))?;
        fs::create_dir_all(test_dir.join("runs/testing"))?;
        
        // Copy signers fixture to the runbook directory - this is key!
        // The signers file must be in the same directory as the runbook
        let signers_content = r#"# Signer definitions for testing environment
# These signers are loaded when using --env testing

signer "alice_signer" "evm::secret_key" {
    secret_key = input.alice_secret
}

signer "bob_signer" "evm::secret_key" {
    secret_key = input.bob_secret
}
"#;
        fs::write(test_dir.join("runbooks/send_eth/signers.testing.tx"), signers_content)?;
        eprintln!("📝 Created runbooks/send_eth/signers.testing.tx");
        
        // Create foundry.toml (even though we're not compiling contracts)
        let foundry_toml = r#"[profile.default]
src = "src"
out = "out"
libs = ["lib"]
"#;
        fs::write(test_dir.join("foundry.toml"), foundry_toml)?;
        eprintln!("📝 Created foundry.toml");
        
        // Create test runbook - signers will be loaded from signers.testing.tx
        let runbook_content = r#"
addon "evm" {
    chain_id = input.evm_chain_id
    rpc_api_url = input.evm_rpc_api_url
}

action "send_eth" "evm::send_eth" {
    description = "Send 0.1 ETH from alice to bob"
    recipient_address = input.bob_address
    amount = 100000000000000000  // 0.1 ETH in wei - INTEGER, not string!
    signer = signer.alice_signer
    confirmations = 0
}

output "tx_hash" {
    value = action.send_eth.tx_hash
}

output "from_address" {
    value = input.alice_address
}

output "to_address" {
    value = input.bob_address
}
"#;
        
        // Write runbook
        fs::write(test_dir.join("runbooks/send_eth/main.tx"), runbook_content)?;
        eprintln!("📝 Created runbook at runbooks/send_eth/main.tx");
        
        // Get anvil manager
        let manager = get_anvil_manager().await?;
        let mut anvil_guard = manager.lock().await;
        let anvil_handle = anvil_guard.get_handle("send_eth_test").await?;
        let rpc_url = anvil_handle.url.clone();
        let accounts = anvil_handle.accounts();
        drop(anvil_guard);
        
        // Write txtx.yml with all the inputs that FixtureBuilder would provide
        let txtx_yml = format!(r#"---
name: send_eth_test
id: send_eth_test
runbooks:
  - name: send_eth
    location: runbooks/send_eth
environments:
  testing:
    confirmations: 0
    evm_chain_id: 31337
    evm_rpc_api_url: {}
    # Alice account
    alice_address: "{}"
    alice_secret: "{}"
    # Bob account  
    bob_address: "{}"
    bob_secret: "{}"
"#, 
            rpc_url,
            accounts.alice.address_string(),
            accounts.alice.secret_string(),
            accounts.bob.address_string(),
            accounts.bob.secret_string()
        );
        
        fs::write(test_dir.join("txtx.yml"), txtx_yml)?;
        eprintln!("📝 Created txtx.yml with testing environment");
        
        // Before execution, let's verify our setup
        eprintln!("\n📋 Pre-execution verification:");
        eprintln!("  Alice address: {}", accounts.alice.address_string());
        eprintln!("  Bob address: {}", accounts.bob.address_string());
        eprintln!("  RPC URL: {}", rpc_url);
        
        // Execute runbook
        let result = crate::tests::fixture_builder::executor::execute_runbook(
            test_dir,
            "send_eth",
            "testing",
            &std::collections::HashMap::new(),
        )?;
        
        if !result.success {
            eprintln!("❌ Runbook execution failed!");
            eprintln!("  Stderr: {}", result.stderr);
            eprintln!("  Stdout: {}", result.stdout);
            
            // Check if files exist
            eprintln!("\n📁 Checking test directory structure:");
            eprintln!("  txtx.yml: {}", test_dir.join("txtx.yml").exists());
            eprintln!("  signers.testing.tx: {}", test_dir.join("signers.testing.tx").exists());
            eprintln!("  foundry.toml: {}", test_dir.join("foundry.toml").exists());
            eprintln!("  runbooks/send_eth/main.tx: {}", test_dir.join("runbooks/send_eth/main.tx").exists());
            
            return Err(format!("Runbook execution failed: {}", result.stderr).into());
        }
        
        // Check outputs
        let tx_hash = result.outputs.get("tx_hash")
            .and_then(|v| v.as_string())
            .ok_or("Should have transaction hash")?;
        
        assert!(tx_hash.starts_with("0x"), "Should have valid transaction hash");
        assert_eq!(tx_hash.len(), 66, "Transaction hash should be 66 characters");
        
        println!("✅ Send ETH test passed with tx: {}", tx_hash);
        Ok(())
    }
    
    /// Test nonce management with directory preservation
    #[tokio::test]
    #[serial(anvil)]
    async fn test_nonce_simple() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test - Anvil not installed");
            return;
        }
        
        let test_dir = create_test_dir("nonce_simple");
        let result = run_nonce_test(&test_dir).await;
        
        match result {
            Ok(_) => {
                // Clean up on success
                let _ = fs::remove_dir_all(&test_dir);
                eprintln!("✅ Test passed - cleaned up directory");
            }
            Err(e) => {
                eprintln!("\n════════════════════════════════════════");
                eprintln!("❌ TEST FAILED: {}", e);
                eprintln!("📁 Directory preserved at:");
                eprintln!("   {}", test_dir.display());
                eprintln!("\nTo investigate:");
                eprintln!("  cd {}", test_dir.display());
                eprintln!("  cat txtx.yml");
                eprintln!("  cat runbooks/nonce_test/main.tx");
                eprintln!("════════════════════════════════════════\n");
                panic!("Test failed: {}", e);
            }
        }
    }
    
    async fn run_nonce_test(test_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔍 Testing nonce management");
        
        // Create directories
        fs::create_dir_all(test_dir.join("src"))?;
        fs::create_dir_all(test_dir.join("runbooks/nonce_test"))?;
        fs::create_dir_all(test_dir.join("runs/testing"))?;
        
        // Copy signers fixture to the runbook directory
        let signers_content = r#"# Signer definitions for testing environment
signer "sender_signer" "evm::secret_key" {
    secret_key = input.sender_secret
}
"#;
        fs::write(test_dir.join("runbooks/nonce_test/signers.testing.tx"), signers_content)?;
        eprintln!("📝 Created runbooks/nonce_test/signers.testing.tx");
        
        // Create foundry.toml
        let foundry_toml = r#"[profile.default]
src = "src"
out = "out"
libs = ["lib"]
"#;
        fs::write(test_dir.join("foundry.toml"), foundry_toml)?;
        eprintln!("📝 Created foundry.toml");
        
        // Create test runbook - signers loaded from signers.testing.tx
        let runbook_content = r#"
addon "evm" {
    chain_id = input.evm_chain_id
    rpc_api_url = input.evm_rpc_api_url
}

action "send_eth" "evm::send_eth" {
    description = "Send ETH in nonce test"
    recipient_address = input.receiver_address
    amount = 100000000000000000  // 0.1 ETH - INTEGER, not string!
    signer = signer.sender_signer
    confirmations = 0
}

output "tx_hash" {
    value = action.send_eth.tx_hash
}

output "receiver_address" {
    value = input.receiver_address
}
"#;
        
        // Write runbook
        fs::write(test_dir.join("runbooks/nonce_test/main.tx"), runbook_content)?;
        
        // Get anvil manager
        let manager = get_anvil_manager().await?;
        let mut anvil_guard = manager.lock().await;
        let anvil_handle = anvil_guard.get_handle("nonce_simple").await?;
        let rpc_url = anvil_handle.url.clone();
        let accounts = anvil_handle.accounts();
        drop(anvil_guard);
        
        // Write txtx.yml with proper account information
        let txtx_yml = format!(r#"---
name: nonce_test
id: nonce_test
runbooks:
  - name: nonce_test
    location: runbooks/nonce_test
environments:
  testing:
    confirmations: 0
    evm_chain_id: 31337
    evm_rpc_api_url: {}
    # Sender account (alice)
    sender_address: "{}"
    sender_secret: "{}"
    # Receiver account (bob)
    receiver_address: "{}"
    receiver_secret: "{}"
"#, 
            rpc_url,
            accounts.alice.address_string(),
            accounts.alice.secret_string(),
            accounts.bob.address_string(),
            accounts.bob.secret_string()
        );
        
        fs::write(test_dir.join("txtx.yml"), txtx_yml)?;
        
        // Execute runbook
        println!("📊 Executing nonce test runbook...");
        let result = crate::tests::fixture_builder::executor::execute_runbook(
            test_dir,
            "nonce_test",
            "testing",
            &std::collections::HashMap::new(),
        )?;
        
        if !result.success {
            eprintln!("❌ Runbook failed:");
            eprintln!("  Stderr: {}", result.stderr);
            if !result.stdout.is_empty() {
                eprintln!("  Stdout: {}", result.stdout);
            }
            return Err(format!("Runbook execution failed: {}", result.stderr).into());
        }
        
        // Check outputs
        let tx_hash = result.outputs.get("tx_hash")
            .and_then(|v| v.as_string())
            .ok_or("Should have transaction hash")?;
        
        assert!(tx_hash.starts_with("0x"), "Should have valid transaction hash");
        
        println!("✅ Nonce test passed");
        Ok(())
    }
}