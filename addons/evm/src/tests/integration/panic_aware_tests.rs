//! Panic-aware error handling tests
//! 
//! These tests use panic handling to preserve test directories on failure

#[cfg(test)]
mod panic_aware_tests {
    use crate::tests::integration::anvil_harness::AnvilInstance;
    use crate::tests::fixture_builder::{
        get_anvil_manager, run_preserving_test, PanicAwareFixture
    };
    use std::fs;
    use serial_test::serial;
    use tokio;
    
    /// Test contract revert reasons with panic preservation
    #[tokio::test]
    #[serial(anvil)]
    async fn test_revert_with_panic_handler() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test - Anvil not installed");
            return;
        }
        
        run_preserving_test("revert_with_panic_handler", |test_dir| Box::pin(async move {
            println!("🔍 Testing revert reason extraction with panic handler");
            
            // Create directories
            fs::create_dir_all(test_dir.join("src"))?;
            fs::create_dir_all(test_dir.join("runbooks/revert_test"))?;
            fs::create_dir_all(test_dir.join("runs/testing"))?;
            
            // Create reverter contract
            let reverter_bytecode = "0x608060405234801561001057600080fd5b50610334806100206000396000f3fe608060405234801561001057600080fd5b506004361061004c5760003560e01c80631b9265b814610051578063398c08ec1461005b578063a3c2f6b61461006f578063ce83732e14610089575b600080fd5b6100596100a5565b005b610069600435610af565b60405180910390f35b61008760048036038101906100829190610214565b610127565b005b6100a360048036038101906100729190610265565b610185565b005b6040517f08c379a00000000000000000000000000000000000000000000000000000000081526004016100f190610301565b60405180910390fd5b60008111610126576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040161011d906102d1565b60405180910390fd5b50565b600073ffffffffffffffffffffffffffffffffffffffff168173ffffffffffffffffffffffffffffffffffffffff161415610183576040517fc5723b5100000000000000000000000000000000000000000000000000000000815260040160405180910390fd5b50565b60008082905060008111915050919050565b600080fd5b6000819050919050565b6101b081610198565b81146101bb57600080fd5b50565b6000813590506101cd816101a7565b92915050565b6000602082840312156101ea576101e9610193565b5b60006101f8848285016101be565b91505092915050565b600073ffffffffffffffffffffffffffffffffffffffff82169050919050565b600061022d82610201565b9050919050565b61023d81610222565b811461024857600080fd5b50565b60008135905061025a81610234565b92915050565b60006020828403121561027657610275610193565b5b60006102848482850161024b565b91505092915050565b600082825260208201905092915050565b7f56616c7565206d75737420626520706f7369746976650000000000000000006000820152505b50565b60006102d760178361028d565b91506102e28261029f565b602082019050919050565b600060208201905081810360008301526102f6816102c8565b9050919050565b7f506c61696e207265766572740000000000000000000000000000000000000060008201525056fe";
            
            // Create test runbook
            let runbook_content = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

variable "deployer" {
    value = evm::create_wallet(input.private_key)
    description = "Deploy wallet"
}

action "deploy_reverter" "evm::deploy_contract" {
    description = "Deploy reverter contract"
    from = variable.deployer
    contract = input.reverter_bytecode
}

output "deployed_address" {
    value = action.deploy_reverter.contract_address
}
"#;
            
            // Write runbook
            fs::write(test_dir.join("runbooks/revert_test/main.tx"), runbook_content)?;
            
            // Get anvil manager
            let manager = get_anvil_manager().await?;
            let mut anvil_guard = manager.lock().await;
            let anvil_handle = anvil_guard.get_handle("revert_test").await?;
            let rpc_url = anvil_handle.url.clone();
            drop(anvil_guard);
            
            // Write txtx.yml
            let txtx_yml = format!(r#"---
name: revert_test
id: revert_test
runbooks:
  - name: revert_test
    location: runbooks/revert_test
environments:
  testing:
    confirmations: 0
    evm_chain_id: 31337
    evm_rpc_api_url: {}
    chain_id: "31337"
    rpc_url: "{}"
    private_key: "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
    reverter_bytecode: "{}"
"#, rpc_url, rpc_url, reverter_bytecode);
            
            fs::write(test_dir.join("txtx.yml"), txtx_yml)?;
            
            // Execute runbook
            let result = crate::tests::fixture_builder::executor::execute_runbook(
                test_dir,
                "revert_test",
                "testing",
                &std::collections::HashMap::new(),
            )?;
            
            if !result.success {
                return Err(format!("Runbook execution failed: {}", result.stderr).into());
            }
            
            // Check outputs
            let deployed = result.outputs.get("deployed_address")
                .and_then(|v| v.as_string())
                .ok_or("Should have deployed address")?;
            
            assert!(deployed.starts_with("0x"), "Should have valid contract address");
            
            println!("✅ Test passed with panic handler");
            Ok(())
        })).await;
    }
    
    /// Test nonce management with panic handler
    #[tokio::test]
    #[serial(anvil)]
    async fn test_nonce_errors_with_panic_handler() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test - Anvil not installed");
            return;
        }
        
        run_preserving_test("nonce_errors_with_panic", |test_dir| Box::pin(async move {
            println!("🔍 Testing nonce errors with panic handler");
            
            // Create directories
            fs::create_dir_all(test_dir.join("src"))?;
            fs::create_dir_all(test_dir.join("runbooks/nonce_test"))?;
            fs::create_dir_all(test_dir.join("runs/testing"))?;
            
            // Create test runbook that tests nonce management
            let runbook_content = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

variable "sender" {
    value = evm::create_wallet(input.private_key)
    description = "Sender wallet"
}

variable "receiver" {
    value = evm::create_wallet()
    description = "Receiver wallet"
}

action "send_eth" "evm::send_eth" {
    from = variable.sender
    to = variable.receiver.address
    amount = "0.1"
}

output "tx_hash" {
    value = action.send_eth.tx_hash
}

output "receiver_address" {
    value = variable.receiver.address
}
"#;
            
            // Write runbook
            fs::write(test_dir.join("runbooks/nonce_test/main.tx"), runbook_content)?;
            
            // Get anvil manager
            let manager = get_anvil_manager().await?;
            let mut anvil_guard = manager.lock().await;
            let anvil_handle = anvil_guard.get_handle("nonce_test").await?;
            let rpc_url = anvil_handle.url.clone();
            drop(anvil_guard);
            
            // Write txtx.yml
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
    chain_id: "31337"
    rpc_url: "{}"
    private_key: "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
"#, rpc_url, rpc_url);
            
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
                eprintln!("  Stdout: {}", result.stdout);
                return Err(format!("Runbook execution failed: {}", result.stderr).into());
            }
            
            // Check outputs
            let tx_hash = result.outputs.get("tx_hash")
                .and_then(|v| v.as_string())
                .ok_or("Should have transaction hash")?;
            
            assert!(tx_hash.starts_with("0x"), "Should have valid transaction hash");
            
            println!("✅ Nonce test passed with panic handler");
            Ok(())
        })).await;
    }
    
    /// Test using PanicAwareFixture directly
    #[tokio::test]
    #[serial(anvil)]
    async fn test_with_panic_aware_fixture() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test - Anvil not installed");
            return;
        }
        
        // Get anvil manager
        let manager = get_anvil_manager().await.unwrap();
        let mut anvil_guard = manager.lock().await;
        let anvil_handle = anvil_guard.get_handle("panic_aware_test").await.unwrap();
        let rpc_url = anvil_handle.url.clone();
        drop(anvil_guard);
        
        // Create panic-aware fixture
        let mut fixture = PanicAwareFixture::new("panic_aware_test", rpc_url.clone())
            .await
            .expect("Failed to create fixture");
        
        // Run test that might panic
        let result = fixture.run_test(|project_dir, rpc_url| Box::pin(async move {
            println!("🧪 Running test with panic-aware fixture");
            
            // Create test runbook
            let runbook_content = r#"
addon "evm" {
    chain_id = "31337"
    rpc_api_url = input.rpc_url
}

variable "test_wallet" {
    value = evm::create_wallet()
}

output "wallet_address" {
    value = variable.test_wallet.address
}
"#;
            
            // Write runbook
            let runbook_dir = project_dir.join("runbooks/simple");
            fs::create_dir_all(&runbook_dir)?;
            fs::write(runbook_dir.join("main.tx"), runbook_content)?;
            
            // Write txtx.yml
            let txtx_yml = format!(r#"---
name: simple_test
id: simple_test
runbooks:
  - name: simple
    location: runbooks/simple
environments:
  testing:
    confirmations: 0
    rpc_url: "{}"
"#, rpc_url);
            
            fs::write(project_dir.join("txtx.yml"), txtx_yml)?;
            
            // Execute runbook
            let result = crate::tests::fixture_builder::executor::execute_runbook(
                project_dir,
                "simple",
                "testing",
                &std::collections::HashMap::new(),
            ).map_err(|e| format!("Failed to execute runbook: {}", e))?;
            
            if !result.success {
                return Err(format!("Runbook failed: {}", result.stderr).into());
            }
            
            // Check output
            let wallet_address = result.outputs.get("wallet_address")
                .and_then(|v| v.as_string())
                .ok_or("Should have wallet address")?;
            
            assert!(wallet_address.starts_with("0x"), "Should have valid address");
            
            println!("✅ Panic-aware fixture test passed");
            Ok(())
        })).await;
        
        match result {
            Ok(_) => println!("✅ Test completed successfully"),
            Err(e) => panic!("Test failed: {}", e),
        }
    }
}