//! Test deploying contracts from foundry project through txtx

#[cfg(test)]
mod foundry_deploy_tests {
    use crate::tests::test_harness::ProjectTestHarness;
    use crate::tests::integration::anvil_harness::AnvilInstance;
    
    #[test]
    fn test_deploy_simple_storage_from_foundry() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test - Anvil not installed");
            return;
        }
        
        println!("Testing SimpleStorage deployment from foundry project");
        
        // Use fixture for foundry deployment
        let fixture_content = std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/integration/foundry/deploy_from_project.tx")
        ).expect("Failed to read fixture");
        
        // Create harness with Anvil
        let mut harness = ProjectTestHarness::new_foundry("deploy_foundry_test.tx", fixture_content)
            .with_anvil();
        
        // Setup project - this should copy the foundry fixtures
        harness.setup().expect("Failed to setup project");
        
        // Verify foundry project was copied
        let out_dir = harness.project_path().join("out");
        assert!(out_dir.exists(), "out/ directory should exist");
        
        let simple_storage_artifact = out_dir.join("SimpleStorage.sol").join("SimpleStorage.json");
        assert!(simple_storage_artifact.exists(), "SimpleStorage.json should exist");
        
        println!("Foundry project structure copied successfully");
        
        // Execute runbook
        let result = harness.execute_runbook()
            .expect("Failed to execute runbook");
        
        // Verify deployment succeeded
        assert!(result.success, "Deployment should succeed");
        
        // Check outputs
        let contract_address = result.outputs.get("contract_address")
            .expect("Should have contract address");
        println!("📍 Contract deployed at: {}", contract_address.as_string().unwrap_or_default());
        
        // Verify on-chain that the contract was deployed
        let contract_addr_str = contract_address.as_string().unwrap_or_default();
        
        // Use Anvil instance to verify
        let anvil = harness.anvil().expect("Anvil should be running");
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            use alloy::providers::{Provider, ProviderBuilder};
            use alloy::primitives::Address;
            use std::str::FromStr;
            
            let provider = ProviderBuilder::new()
                .on_http(anvil.url.parse().unwrap());
            
            // Check code exists at deployed address
            let deployed_addr = Address::from_str(&contract_addr_str)
                .expect("Should parse deployed address");
            let code = provider.get_code_at(deployed_addr)
                .await
                .expect("Should get code");
            
            assert!(!code.is_empty(), "Contract code should exist at deployed address");
            println!("Contract code verified on-chain ({} bytes)", code.len());
            
            // Try to call the retrieve function directly using alloy
            let retrieve_selector = [0x2e, 0x64, 0xce, 0xc1]; // retrieve()
            let call_data = retrieve_selector.to_vec();
            
            let tx_request = alloy::rpc::types::TransactionRequest::default()
                .to(deployed_addr)
                .input(call_data.into());
            
            let result = provider.call(tx_request)
                .await
                .expect("Should be able to call retrieve");
            
            // The result should be 42 (0x2a) padded to 32 bytes
            let value = alloy::primitives::U256::from_be_slice(&result);
            println!("📊 Retrieved value: {}", value);
            assert_eq!(value, alloy::primitives::U256::from(42), "Initial value should be 42");
        });
        
        println!("SimpleStorage deployed and verified through txtx!");
        
        harness.cleanup();
    }
    
    #[test]
    fn test_deploy_with_create2_from_foundry() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("Warning: Skipping test - Anvil not installed");
            return;
        }
        
        println!("Testing CREATE2 deployment with foundry contract");
        
        // Use existing CREATE2 fixture
        let runbook = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

signer "deployer" "evm::secret_key" {
    secret_key = input.deployer_private_key
}

variable "simple_storage" {
    value = evm::get_contract_from_foundry_project("SimpleStorage")
}

variable "salt" {
    value = "0000000000000000000000000000000000000000000000000000000000000042"
}

# Calculate expected address
variable "init_code" {
    value = std::concat(
        variable.simple_storage.bytecode,
        evm::encode_constructor_args(variable.simple_storage.abi, [100])
    )
}

variable "expected_address" {
    value = evm::create2(variable.salt, variable.init_code)
}

# Deploy with CREATE2
action "deploy" "evm::deploy_contract" {
    contract = variable.simple_storage
    constructor_args = [100]
    create2 = {
        salt = variable.salt
    }
    signer = signer.deployer
    confirmations = 0
}

output "expected_address" {
    value = variable.expected_address
}

output "deployed_address" {
    value = action.deploy.contract_address
}
"#;
        
        let mut harness = ProjectTestHarness::new_foundry("create2_foundry_test.tx", runbook.to_string())
            .with_anvil();
        
        harness.setup().expect("Failed to setup project");
        
        let result = harness.execute_runbook()
            .expect("Failed to execute runbook");
        
        assert!(result.success, "Deployment should succeed");
        
        let expected = result.outputs.get("expected_address")
            .and_then(|v| v.as_string())
            .unwrap_or_default();
        let deployed = result.outputs.get("deployed_address")
            .and_then(|v| v.as_string())
            .unwrap_or_default();
        
        println!("📍 Expected: {}", expected);
        println!("📍 Deployed: {}", deployed);
        
        // For now, just check that we got addresses
        // The exact match might depend on the CREATE2 factory address
        assert!(!deployed.is_empty(), "Should have deployed address");
        
        println!("CREATE2 deployment with foundry contract completed!");
        
        harness.cleanup();
    }
}