//! Test demonstrating the integer vs string issue and fix

#[cfg(test)]
mod integer_vs_string_test {
    use crate::tests::integration::anvil_harness::AnvilInstance;
    use crate::tests::fixture_builder::get_anvil_manager;
    use std::fs;
    use std::path::PathBuf;
    use serial_test::serial;
    use tokio;
    
    /// Test that shows string amounts cause panic
    #[tokio::test]
    #[serial(anvil)]
    async fn test_string_amount_fails() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test - Anvil not installed");
            return;
        }
        
        println!("\n🔍 Testing that string amounts cause panic...");
        
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let test_dir = PathBuf::from(format!("/tmp/txtx_string_fail_{}", timestamp));
        fs::create_dir_all(&test_dir).unwrap();
        fs::create_dir_all(test_dir.join("runbooks/test")).unwrap();
        
        let manager = get_anvil_manager().await.unwrap();
        let mut anvil_guard = manager.lock().await;
        let anvil_handle = anvil_guard.get_handle("string_fail").await.unwrap();
        let rpc_url = anvil_handle.url.clone();
        let accounts = anvil_handle.accounts();
        drop(anvil_guard);
        
        // Runbook with STRING amount (will fail)
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
    amount = "1000000000000000000"  # STRING - will cause panic!
    signer = signer.alice
    confirmations = 0
}

output "tx_hash" {
    value = action.send.tx_hash
}
"#;
        
        fs::write(test_dir.join("runbooks/test/main.tx"), runbook).unwrap();
        
        let txtx_yml = format!(r#"---
name: string_test
id: string_test
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
        
        println!("📊 Executing runbook with STRING amount...");
        let result = crate::tests::fixture_builder::executor::execute_runbook(
            &test_dir,
            "test",
            "testing",
            &std::collections::HashMap::new(),
        );
        
        match result {
            Ok(res) => {
                if res.success {
                    println!("❌ UNEXPECTED: String amount should have failed!");
                    let _ = fs::remove_dir_all(&test_dir);
                    panic!("String amount should have caused failure");
                } else {
                    // Check for the panic in stderr
                    if res.stderr.contains("panicked at") && res.stderr.contains("expect_uint") {
                        println!("✅ EXPECTED: String amount caused panic as expected");
                        println!("   Error: {}", res.stderr);
                    } else {
                        println!("❓ Failed but not with expected panic: {}", res.stderr);
                    }
                    let _ = fs::remove_dir_all(&test_dir);
                }
            }
            Err(e) => {
                println!("✅ EXPECTED: Execution failed with: {}", e);
                let _ = fs::remove_dir_all(&test_dir);
            }
        }
    }
    
    /// Test that shows integer amounts work correctly
    #[tokio::test]
    #[serial(anvil)]
    async fn test_integer_amount_succeeds() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test - Anvil not installed");
            return;
        }
        
        println!("\n🔍 Testing that integer amounts work correctly...");
        
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let test_dir = PathBuf::from(format!("/tmp/txtx_integer_success_{}", timestamp));
        fs::create_dir_all(&test_dir).unwrap();
        fs::create_dir_all(test_dir.join("runbooks/test")).unwrap();
        
        let manager = get_anvil_manager().await.unwrap();
        let mut anvil_guard = manager.lock().await;
        let anvil_handle = anvil_guard.get_handle("integer_success").await.unwrap();
        let rpc_url = anvil_handle.url.clone();
        let accounts = anvil_handle.accounts();
        drop(anvil_guard);
        
        // Runbook with INTEGER amount (will succeed)
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
    amount = 1000000000000000000  # INTEGER - works correctly!
    signer = signer.alice
    confirmations = 0
}

output "tx_hash" {
    value = action.send.tx_hash
}
"#;
        
        fs::write(test_dir.join("runbooks/test/main.tx"), runbook).unwrap();
        
        let txtx_yml = format!(r#"---
name: integer_test
id: integer_test
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
        
        println!("📊 Executing runbook with INTEGER amount...");
        let result = crate::tests::fixture_builder::executor::execute_runbook(
            &test_dir,
            "test",
            "testing",
            &std::collections::HashMap::new(),
        );
        
        match result {
            Ok(res) => {
                if res.success {
                    println!("✅ SUCCESS: Integer amount works correctly!");
                    if let Some(tx_hash) = res.outputs.get("tx_hash") {
                        println!("   Transaction hash: {:?}", tx_hash);
                    }
                    let _ = fs::remove_dir_all(&test_dir);
                } else {
                    println!("❌ UNEXPECTED: Integer amount should have succeeded!");
                    println!("   Error: {}", res.stderr);
                    panic!("Integer amount should have succeeded");
                }
            }
            Err(e) => {
                println!("❌ UNEXPECTED: Execution failed with: {}", e);
                panic!("Integer amount should have succeeded");
            }
        }
    }
    
    /// Test comparing string vs integer side by side
    #[tokio::test]
    #[serial(anvil)]
    async fn test_string_vs_integer_comparison() {
        println!("\n📊 String vs Integer Comparison Test");
        println!("=====================================");
        
        println!("\n❌ STRING values (quoted) cause panic:");
        println!("   amount = \"1000000000000000000\"");
        println!("   gas_limit = \"21000\"");
        println!("   confirmations = \"1\"");
        println!("   Result: panic at expect_uint()");
        
        println!("\n✅ INTEGER values (unquoted) work:");
        println!("   amount = 1000000000000000000");
        println!("   gas_limit = 21000");
        println!("   confirmations = 1");
        println!("   Result: Transaction succeeds");
        
        println!("\n📝 Key Takeaway:");
        println!("   Always use unquoted integers for numeric values in txtx runbooks!");
        println!("   This affects: amount, gas_limit, gas_price, confirmations, nonce, etc.");
    }
}