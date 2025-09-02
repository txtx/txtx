//! Minimal test to isolate the panic issue

#[cfg(test)]
mod minimal_test {
    use crate::tests::integration::anvil_harness::AnvilInstance;
    use crate::tests::fixture_builder::get_anvil_manager;
    use std::fs;
    use std::path::PathBuf;
    use serial_test::serial;
    use tokio;
    
    /// Absolutely minimal runbook test
    #[tokio::test]
    #[serial(anvil)]
    async fn test_minimal_runbook() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing minimal runbook");
        
        // Create test directory
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let test_dir = PathBuf::from(format!("/tmp/txtx_minimal_{}", timestamp));
        fs::create_dir_all(&test_dir).unwrap();
        
        // Create minimal structure
        fs::create_dir_all(test_dir.join("runbooks/test")).unwrap();
        
        // Get anvil
        let manager = get_anvil_manager().await.unwrap();
        let mut anvil_guard = manager.lock().await;
        let anvil_handle = anvil_guard.get_handle("minimal").await.unwrap();
        let rpc_url = anvil_handle.url.clone();
        drop(anvil_guard);
        
        // Create the SIMPLEST possible runbook - just an addon block
        let runbook = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

output "test" {
    value = "hello"
}
"#;
        
        fs::write(test_dir.join("runbooks/test/main.tx"), runbook).unwrap();
        
        // Create minimal txtx.yml
        let txtx_yml = format!(r#"---
name: minimal
id: minimal
runbooks:
  - name: test
    location: runbooks/test
environments:
  testing:
    chain_id: 31337
    rpc_url: {}
"#, rpc_url);
        
        fs::write(test_dir.join("txtx.yml"), txtx_yml).unwrap();
        
        // Try to execute
        println!("📊 Executing minimal runbook...");
        let result = crate::tests::fixture_builder::executor::execute_runbook(
            &test_dir,
            "test",
            "testing",
            &std::collections::HashMap::new(),
        );
        
        match result {
            Ok(res) => {
                if res.success {
                    println!("✅ Minimal runbook executed successfully!");
                    println!("  Outputs: {:?}", res.outputs);
                    
                    // Clean up
                    let _ = fs::remove_dir_all(&test_dir);
                } else {
                    println!("❌ Execution failed:");
                    println!("  Stderr: {}", res.stderr);
                    println!("  Stdout: {}", res.stdout);
                    println!("📁 Directory preserved: {}", test_dir.display());
                }
            }
            Err(e) => {
                println!("❌ Error: {}", e);
                println!("📁 Directory preserved: {}", test_dir.display());
            }
        }
    }
    
    /// Test with just a signer
    #[tokio::test]
    #[serial(anvil)]
    async fn test_with_signer() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing runbook with signer");
        
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let test_dir = PathBuf::from(format!("/tmp/txtx_signer_{}", timestamp));
        fs::create_dir_all(&test_dir).unwrap();
        fs::create_dir_all(test_dir.join("runbooks/test")).unwrap();
        
        let manager = get_anvil_manager().await.unwrap();
        let mut anvil_guard = manager.lock().await;
        let anvil_handle = anvil_guard.get_handle("signer").await.unwrap();
        let rpc_url = anvil_handle.url.clone();
        let accounts = anvil_handle.accounts();
        drop(anvil_guard);
        
        // Runbook with a signer
        let runbook = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

signer "alice" "evm::secret_key" {
    secret_key = input.alice_secret
}

output "alice_address" {
    value = input.alice_address
}
"#;
        
        fs::write(test_dir.join("runbooks/test/main.tx"), runbook).unwrap();
        
        // Put signer in signers.testing.tx
        let signers = r#"
signer "alice" "evm::secret_key" {
    secret_key = input.alice_secret
}
"#;
        fs::write(test_dir.join("runbooks/test/signers.testing.tx"), signers).unwrap();
        
        let txtx_yml = format!(r#"---
name: signer_test
id: signer_test
runbooks:
  - name: test
    location: runbooks/test
environments:
  testing:
    chain_id: 31337
    rpc_url: {}
    alice_address: "{}"
    alice_secret: "{}"
"#, rpc_url, accounts.alice.address_string(), accounts.alice.secret_string());
        
        fs::write(test_dir.join("txtx.yml"), txtx_yml).unwrap();
        
        println!("📊 Executing signer runbook...");
        let result = crate::tests::fixture_builder::executor::execute_runbook(
            &test_dir,
            "test",
            "testing",
            &std::collections::HashMap::new(),
        );
        
        match result {
            Ok(res) => {
                if res.success {
                    println!("✅ Signer runbook executed successfully!");
                    println!("  Outputs: {:?}", res.outputs);
                    let _ = fs::remove_dir_all(&test_dir);
                } else {
                    println!("❌ Execution failed:");
                    println!("  Stderr: {}", res.stderr);
                    println!("  Stdout: {}", res.stdout);
                    println!("📁 Directory preserved: {}", test_dir.display());
                }
            }
            Err(e) => {
                println!("❌ Error: {}", e);
                println!("📁 Directory preserved: {}", test_dir.display());
            }
        }
    }
    
    /// Test send_eth with minimal setup
    #[tokio::test]
    #[serial(anvil)]
    async fn test_minimal_send_eth() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing minimal send_eth");
        
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let test_dir = PathBuf::from(format!("/tmp/txtx_send_minimal_{}", timestamp));
        fs::create_dir_all(&test_dir).unwrap();
        fs::create_dir_all(test_dir.join("runbooks/test")).unwrap();
        
        let manager = get_anvil_manager().await.unwrap();
        let mut anvil_guard = manager.lock().await;
        let anvil_handle = anvil_guard.get_handle("send_minimal").await.unwrap();
        let rpc_url = anvil_handle.url.clone();
        let accounts = anvil_handle.accounts();
        drop(anvil_guard);
        
        // Minimal send_eth runbook
        let runbook = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

signer "alice" "evm::secret_key" {
    secret_key = input.alice_secret
}

action "send" "evm::send_eth" {
    recipient_address = input.bob_address
    amount = 1000000000000000000  // No quotes - it's an integer!
    signer = signer.alice
    confirmations = 0
}

output "tx_hash" {
    value = action.send.tx_hash
}
"#;
        
        fs::write(test_dir.join("runbooks/test/main.tx"), runbook).unwrap();
        
        let txtx_yml = format!(r#"---
name: send_test
id: send_test
runbooks:
  - name: test
    location: runbooks/test
environments:
  testing:
    chain_id: 31337
    rpc_url: {}
    alice_address: "{}"
    alice_secret: "{}"
    bob_address: "{}"
"#, rpc_url, 
    accounts.alice.address_string(), 
    accounts.alice.secret_string(),
    accounts.bob.address_string());
        
        fs::write(test_dir.join("txtx.yml"), txtx_yml).unwrap();
        
        println!("📊 Executing minimal send_eth runbook...");
        println!("  Alice: {}", accounts.alice.address_string());
        println!("  Bob: {}", accounts.bob.address_string());
        
        let result = crate::tests::fixture_builder::executor::execute_runbook(
            &test_dir,
            "test",
            "testing",
            &std::collections::HashMap::new(),
        );
        
        match result {
            Ok(res) => {
                if res.success {
                    println!("✅ Send ETH executed successfully!");
                    println!("  Outputs: {:?}", res.outputs);
                    let _ = fs::remove_dir_all(&test_dir);
                } else {
                    println!("❌ Execution failed:");
                    println!("  Stderr: {}", res.stderr);
                    println!("  Stdout: {}", res.stdout);
                    println!("📁 Directory preserved: {}", test_dir.display());
                    panic!("Send ETH failed");
                }
            }
            Err(e) => {
                println!("❌ Error: {}", e);
                println!("📁 Directory preserved: {}", test_dir.display());
                panic!("Error: {}", e);
            }
        }
    }
}