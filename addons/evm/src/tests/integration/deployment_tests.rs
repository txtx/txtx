//! Integration tests for contract deployment
//!
//! Most deployment tests have been migrated to txtx fixtures for better maintainability.
//! See:
//! - fixtures/integration/deployments/ for basic deployment patterns
//! - foundry_deploy_tests.rs::test_deploy_with_create2_from_foundry for full CREATE2 deployment
//! - create2_deployment_tests.rs for CREATE2 address calculation
//! - Test error scenarios in fixtures/integration/errors/
//!
//! CREATE2 deployment is fully supported via the deploy_contract action:
//! ```
//! action "deploy" "evm::deploy_contract" {
//!     contract = variable.my_contract
//!     create2 = {
//!         salt = "0x..."
//!     }
//!     signer = signer.deployer
//! }
//! ```

#[cfg(test)]
mod deployment_integration_tests {
    use super::super::anvil_harness::AnvilInstance;
    use crate::rpc::EvmRpc;
    use alloy::primitives::{Address, Bytes, B256, U256};
    use std::str::FromStr;
    
    #[test]
    fn test_create2_address_calculation() {
        // Test CREATE2 address calculation without deployment
        let deployer = Address::from_str("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8").unwrap();
        let salt = B256::from([42u8; 32]);
        let minimal_bytecode = "0x602a60005260206000f3";
        let bytecode = Bytes::from_str(minimal_bytecode).unwrap();
        
        // Calculate CREATE2 address
        let init_code_hash = alloy::primitives::keccak256(&bytecode);
        let create2_hash = alloy::primitives::keccak256(
            [
                &[0xff],
                deployer.as_slice(),
                salt.as_slice(),
                init_code_hash.as_slice(),
            ].concat()
        );
        
        let expected_address = Address::from_slice(&create2_hash[12..]);
        println!("Calculated CREATE2 address: {}", expected_address);
        
        // Verify it's deterministic
        let recalculated = {
            let init_code_hash = alloy::primitives::keccak256(&bytecode);
            let create2_hash = alloy::primitives::keccak256(
                [
                    &[0xff],
                    deployer.as_slice(),
                    salt.as_slice(),
                    init_code_hash.as_slice(),
                ].concat()
            );
            Address::from_slice(&create2_hash[12..])
        };
        
        assert_eq!(expected_address, recalculated, "CREATE2 address should be deterministic");
    }
    
    #[tokio::test]
    async fn test_simple_storage_deployment_and_interaction() {
        use alloy::providers::Provider;
        use alloy::network::{EthereumWallet, TransactionBuilder};
        use alloy::rpc::types::TransactionRequest;
        use alloy::primitives::hex;
        use alloy::json_abi::JsonAbi;
        
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_simple_storage_deployment_and_interaction - Anvil not installed");
            return;
        }
        
        // Spawn Anvil instance
        let anvil = AnvilInstance::spawn();
        println!("Anvil spawned on {}", anvil.url);
        
        // Load the SimpleStorage contract bytecode from the JSON file
        let contract_json = include_str!("../fixtures/foundry/out/SimpleStorage.sol/SimpleStorage.json");
        let contract_artifact: serde_json::Value = serde_json::from_str(contract_json).unwrap();
        let bytecode_hex = contract_artifact["bytecode"]["object"].as_str().unwrap();
        let bytecode = Bytes::from_str(bytecode_hex).unwrap();
        
        // Get the ABI for encoding/decoding
        let abi_json = serde_json::to_string(&contract_artifact["abi"]).unwrap();
        let abi: JsonAbi = serde_json::from_str(&abi_json).unwrap();
        
        let deployer = &anvil.accounts[0];
        let wallet = EthereumWallet::from(deployer.signer.clone());
        let rpc = crate::rpc::EvmWalletRpc::new(&anvil.url, wallet.clone()).unwrap();
        
        println!("📝 Deploying SimpleStorage contract...");
        
        // Encode constructor arguments (initial value = 42)
        let init_value = U256::from(42);
        let constructor_data = alloy::dyn_abi::DynSolValue::Uint(init_value, 256).abi_encode();
        
        // Combine bytecode with constructor arguments
        let mut deploy_data = bytecode.to_vec();
        deploy_data.extend_from_slice(&constructor_data);
        
        // Build deployment transaction
        let mut deploy_tx = TransactionRequest::default();
        deploy_tx.set_create();  // Mark as contract deployment (no `to` address)
        deploy_tx = deploy_tx
            .from(deployer.address)
            .input(deploy_data.into())
            .nonce(rpc.provider.get_transaction_count(deployer.address).await.unwrap())
            .gas_limit(1_000_000)
            .max_fee_per_gas(20_000_000_000u128)
            .max_priority_fee_per_gas(1_000_000_000u128);
        
        deploy_tx.set_chain_id(31337);
        
        // Deploy the contract
        let deploy_envelope = deploy_tx.build(&wallet).await.unwrap();
        let deploy_hash = rpc.sign_and_send_tx(deploy_envelope).await.unwrap();
        
        println!("📨 Deployment tx sent: 0x{}", hex::encode(deploy_hash));
        
        // Wait for deployment
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        // Get deployment receipt to find contract address
        let receipt = rpc.provider.get_transaction_receipt(deploy_hash.into()).await.unwrap()
            .expect("Deployment should be mined");
        
        let contract_address = receipt.contract_address.expect("Should have contract address");
        println!("Contract deployed at: {}", contract_address);
        
        // Test 1: Call retrieve() - should return initial value (42)
        println!("\n📖 Testing retrieve() function...");
        let retrieve_fn = abi.function("retrieve").unwrap().first().unwrap();
        let mut retrieve_data = retrieve_fn.selector().to_vec();
        
        let call_tx = TransactionRequest::default()
            .to(contract_address)
            .input(retrieve_data.clone().into());
        let call_result = rpc.provider.call(call_tx.clone()).await.unwrap();
        
        let initial_value = U256::from_be_slice(&call_result).to::<u64>();
        
        assert_eq!(initial_value, 42, "Initial value should be 42");
        println!("   ✓ Initial value: {}", initial_value);
        
        // Test 2: Call store() to update the value to 123
        println!("\n📝 Testing store() function...");
        let store_fn = abi.function("store").unwrap().first().unwrap();
        let new_value = U256::from(123);
        let mut store_data = store_fn.selector().to_vec();
        store_data.extend_from_slice(&alloy::dyn_abi::DynSolValue::Uint(new_value, 256).abi_encode());
        
        let mut store_tx = TransactionRequest::default();
        store_tx = store_tx
            .from(deployer.address)
            .to(contract_address)
            .input(store_data.into())
            .nonce(rpc.provider.get_transaction_count(deployer.address).await.unwrap())
            .gas_limit(100_000)
            .max_fee_per_gas(20_000_000_000u128)
            .max_priority_fee_per_gas(1_000_000_000u128);
        
        store_tx.set_chain_id(31337);
        
        let store_envelope = store_tx.build(&wallet).await.unwrap();
        let store_hash = rpc.sign_and_send_tx(store_envelope).await.unwrap();
        
        println!("   Store tx sent: 0x{}", hex::encode(store_hash));
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        // Test 3: Call retrieve() again - should return new value (123)
        let call_tx = TransactionRequest::default()
            .to(contract_address)
            .input(retrieve_data.into());
        let call_result = rpc.provider.call(call_tx.clone()).await.unwrap();
        
        let updated_value = U256::from_be_slice(&call_result).to::<u64>();
        
        assert_eq!(updated_value, 123, "Updated value should be 123");
        println!("   ✓ Updated value: {}", updated_value);
        

        
        println!("\nSimpleStorage contract deployment and interaction test completed successfully!");
    }
}