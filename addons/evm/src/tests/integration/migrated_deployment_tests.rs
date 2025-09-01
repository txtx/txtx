//! Contract deployment tests migrated to txtx framework

#[cfg(test)]
mod migrated_deployment_tests {
    use crate::tests::integration::anvil_harness::AnvilInstance;
    use txtx_addon_kit::types::types::Value;
    use std::path::PathBuf;
    use tokio;

    #[tokio::test]
    async fn test_minimal_contract_deployment_txtx() {
        let anvil = AnvilInstance::start();
        let fixture = PathBuf::from("fixtures/integration/deployments/minimal_contract.tx");
        
        // REMOVED:         let harness = MigrationHelper::from_fixture(&fixture)
            .with_input("chain_id", &anvil.chain_id().to_string())
            .with_input("rpc_url", &anvil.endpoint());

        let result = result.execute().await;
        
        match result {
            Ok(result) => {
                assert!(result.success, "Deployment should succeed");
                
                // Check outputs
                let outputs = &result.outputs;
                assert!(outputs.contains_key("contract_address"));
                assert!(outputs.contains_key("deploy_tx"));
                
                if let Some(Value::String(addr)) = outputs.get("contract_address") {
                    assert!(addr.starts_with("0x"));
                    assert_eq!(addr.len(), 42);
                    println!("Minimal contract deployed at: {}", addr);
                }
                
                println!("Minimal contract deployed successfully");
            }
            Err(e) => panic!("Deployment failed: {}", e),
        }
        
        harness.cleanup();
    }

    #[tokio::test]
    async fn test_constructor_args_deployment_txtx() {
        let anvil = AnvilInstance::start();
        let fixture = PathBuf::from("fixtures/integration/deployments/constructor_args.tx");
        
        // REMOVED:         let harness = MigrationHelper::from_fixture(&fixture)
            .with_input("chain_id", &anvil.chain_id().to_string())
            .with_input("rpc_url", &anvil.endpoint());

        let result = result.execute().await;
        
        match result {
            Ok(result) => {
                assert!(result.success, "Deployment with constructor args should succeed");
                
                // Verify constructor value was set
                if let Some(stored_value) = result.outputs.get("stored_value") {
                    match stored_value {
                        Value::Integer(v) => assert_eq!(*v, 42i128),
                        _ => panic!("Expected integer value"),
                    }
                }
                
                println!("Constructor args deployment succeeded");
            }
            Err(e) => panic!("Deployment failed: {}", e),
        }
        
        harness.cleanup();
    }

    #[tokio::test]
    async fn test_complex_constructor_deployment_txtx() {
        let anvil = AnvilInstance::start();
        let fixture = PathBuf::from("fixtures/integration/deployments/complex_constructor.tx");
        
        // REMOVED:         let harness = MigrationHelper::from_fixture(&fixture)
            .with_input("chain_id", &anvil.chain_id().to_string())
            .with_input("rpc_url", &anvil.endpoint());

        let result = result.execute().await;
        
        match result {
            Ok(result) => {
                assert!(result.success, "Complex constructor deployment should succeed");
                assert!(result.outputs.contains_key("contract_address"));
                println!("Complex constructor deployment succeeded");
            }
            Err(e) => panic!("Deployment failed: {}", e),
        }
        
        harness.cleanup();
    }

    #[tokio::test]
    async fn test_storage_contract_deployment_txtx() {
        let anvil = AnvilInstance::start();
        let fixture = PathBuf::from("fixtures/integration/deployments/storage_contract.tx");
        
        // REMOVED:         let harness = MigrationHelper::from_fixture(&fixture)
            .with_input("chain_id", &anvil.chain_id().to_string())
            .with_input("rpc_url", &anvil.endpoint());

        let result = result.execute().await;
        
        match result {
            Ok(result) => {
                assert!(result.success, "Storage deployment should succeed");
                println!("Storage contract deployed with constructor args");
            }
            Err(e) => panic!("Deployment failed: {}", e),
        }
        
        harness.cleanup();
    }

    #[tokio::test]
    async fn test_factory_pattern_deployment_txtx() {
        let anvil = AnvilInstance::start();
        let fixture = PathBuf::from("fixtures/integration/deployments/factory_pattern.tx");
        
        // REMOVED:         let harness = MigrationHelper::from_fixture(&fixture)
            .with_input("chain_id", &anvil.chain_id().to_string())
            .with_input("rpc_url", &anvil.endpoint());

        let result = result.execute().await;
        
        match result {
            Ok(result) => {
                assert!(result.success, "Factory deployment should succeed");
                
                // Check that both factory and child contracts were deployed
                assert!(result.outputs.contains_key("factory_address"));
                assert!(result.outputs.contains_key("child_address"));
                
                println!("Factory pattern deployment succeeded");
            }
            Err(e) => panic!("Deployment failed: {}", e),
        }
        
        harness.cleanup();
    }

    #[tokio::test]
    async fn test_upgradeable_proxy_deployment_txtx() {
        let anvil = AnvilInstance::start();
        let fixture = PathBuf::from("fixtures/integration/deployments/upgradeable_proxy.tx");
        
        // REMOVED:         let harness = MigrationHelper::from_fixture(&fixture)
            .with_input("chain_id", &anvil.chain_id().to_string())
            .with_input("rpc_url", &anvil.endpoint());

        let result = result.execute().await;
        
        match result {
            Ok(result) => {
                assert!(result.success, "Upgradeable proxy deployment should succeed");
                
                // Check proxy and implementation addresses
                assert!(result.outputs.contains_key("proxy_address"));
                assert!(result.outputs.contains_key("implementation_address"));
                
                println!("Upgradeable proxy deployment succeeded");
            }
            Err(e) => panic!("Deployment failed: {}", e),
        }
        
        harness.cleanup();
    }

    #[tokio::test]
    async fn test_deployment_with_interaction_txtx() {
        let anvil = AnvilInstance::start();
        let fixture = PathBuf::from("fixtures/integration/deployments/deploy_and_interact.tx");
        
        // REMOVED:         let harness = MigrationHelper::from_fixture(&fixture)
            .with_input("chain_id", &anvil.chain_id().to_string())
            .with_input("rpc_url", &anvil.endpoint());

        let result = result.execute().await;
        
        match result {
            Ok(result) => {
                assert!(result.success, "Deployment and interaction should succeed");
                
                println!("Counter contract deployed and interacted successfully");
                if let Some(initial) = result.outputs.get("initial_value") {
                    println!("   Initial value: {:?}", initial);
                }
                if let Some(incremented) = result.outputs.get("incremented_value") {
                    println!("   Value after increment: {:?}", incremented);
                }
            }
            Err(e) => panic!("Test failed: {}", e),
        }
    }
}