//! Integration tests that execute txtx runbooks and verify blockchain state changes
//! 
//! These tests validate the full txtx stack from runbook parsing through
//! execution against a real Ethereum node (Anvil).

mod txtx_execution_tests {
    use crate::tests::test_harness::{ProjectTestHarness, CompilationFramework};
    use crate::tests::integration::anvil_harness::AnvilInstance;
    use crate::tests::test_constants::{ANVIL_ACCOUNTS, ANVIL_PRIVATE_KEYS};
    use crate::anvil_test;
    use txtx_addon_kit::types::types::Value;
    use std::collections::HashMap;



    anvil_test!(test_send_eth_through_txtx, {
        // Start Anvil instance
        let anvil = AnvilInstance::spawn();

        // Set up project with the runbook fixture
        let result = ProjectTestHarness::new_foundry_from_fixture(
            "integration/send_eth.tx"
        )
        .with_input("rpc_url", anvil.rpc_url())
        .with_input("sender_address", ANVIL_ACCOUNTS[0])
        .with_input("recipient_address", ANVIL_ACCOUNTS[1])
        .with_input("sender_private_key", ANVIL_PRIVATE_KEYS[0])
            .execute()
            .await
            .expect("Failed to execute test");

        // Setup the project structure
        harness.setup().expect("Failed to setup project");

        // Execute the runbook through txtx
        let result = result.execute().await;
        
        // Verify execution succeeded
        assert!(result.is_ok(), "Runbook execution failed: {:?}", result);
        
        let execution_result = result.unwrap();
        assert!(execution_result.success, "Runbook execution was not successful");
        
        // Verify we got a transaction hash in the outputs
        assert!(
            execution_result.outputs.contains_key("tx_hash"),
            "Transaction hash not found in outputs"
        );
        
        // Verify the transaction status is successful (1)
        if let Some(Value::Integer(status)) = execution_result.outputs.get("receipt_status") {
            assert_eq!(*status, 1, "Transaction failed with status 0");
        } else {
            panic!("Receipt status not found or invalid type");
        }
        
        // Verify gas was used
        assert!(
            execution_result.outputs.contains_key("gas_used"),
            "Gas used not found in outputs"
        );
    });

    anvil_test!(test_deploy_contract_through_txtx, {
        // Start Anvil instance
        let anvil = AnvilInstance::spawn();

        // Set up Foundry-based project with fixture
        let result = ProjectTestHarness::new_foundry_from_fixture(
            "integration/deploy_contract.tx"
        )
        .with_input("rpc_url", anvil.rpc_url())
        .with_input("deployer_address", ANVIL_ACCOUNTS[0])
        .with_input("deployer_private_key", ANVIL_PRIVATE_KEYS[0])
            .execute()
            .await
            .expect("Failed to execute test");

        // Setup the project structure
        harness.setup().expect("Failed to setup Foundry project");

        // Execute the runbook
        let result = result.execute().await;
        
        // Verify execution succeeded
        assert!(result.is_ok(), "Runbook execution failed: {:?}", result);
        
        let execution_result = result.unwrap();
        assert!(execution_result.success, "Runbook execution was not successful");
        
        // Verify we got a contract address
        assert!(
            execution_result.outputs.contains_key("contract_address"),
            "Contract address not found in outputs"
        );
        
        // Verify we got a deployment transaction hash
        assert!(
            execution_result.outputs.contains_key("deployment_tx"),
            "Deployment transaction hash not found in outputs"
        );
        
        // Verify the deployment was successful
        if let Some(Value::Integer(status)) = execution_result.outputs.get("deployment_status") {
            assert_eq!(*status, 1, "Contract deployment failed with status 0");
        } else {
            panic!("Deployment status not found or invalid type");
        }
    });

    anvil_test!(test_contract_interaction_through_txtx, {
        // Start Anvil instance
        let anvil = AnvilInstance::spawn();

        // Set up project with interaction fixture
        let result = ProjectTestHarness::new_foundry_from_fixture(
            "integration/interact_contract.tx"
        )
        .with_input("rpc_url", anvil.rpc_url())
        .with_input("deployer_address", ANVIL_ACCOUNTS[0])
        .with_input("deployer_private_key", ANVIL_PRIVATE_KEYS[0])
            .execute()
            .await
            .expect("Failed to execute test");

        // Setup the project structure
        harness.setup().expect("Failed to setup project");

        // Execute the runbook
        let result = result.execute().await;
        
        // Verify execution succeeded
        assert!(result.is_ok(), "Runbook execution failed: {:?}", result);
        
        let execution_result = result.unwrap();
        assert!(execution_result.success, "Runbook execution was not successful");
        
        // Verify the contract was deployed
        assert!(
            execution_result.outputs.contains_key("contract_address"),
            "Contract address not found"
        );
        
        // Verify the set transaction succeeded
        assert!(
            execution_result.outputs.contains_key("set_tx"),
            "Set transaction hash not found"
        );
        
        // Verify the set transaction status
        if let Some(Value::Integer(status)) = execution_result.outputs.get("set_status") {
            assert_eq!(*status, 1, "Set transaction failed with status 0");
        }
        
        // Verify the stored value is correct
        if let Some(Value::Integer(value)) = execution_result.outputs.get("stored_value") {
            assert_eq!(*value, 42, "Stored value should be 42, got {}", value);
        } else {
            panic!("Stored value not found or invalid type");
        }
    });

    anvil_test!(test_hardhat_deployment_through_txtx, {
        // Start Anvil instance
        let anvil = AnvilInstance::spawn();

        // Set up Hardhat-based project with fixture
        let result = ProjectTestHarness::new_hardhat_from_fixture(
            "integration/hardhat_deploy.tx"
        )
        .with_input("rpc_url", anvil.rpc_url())
        .with_input("deployer_address", ANVIL_ACCOUNTS[0])
        .with_input("deployer_private_key", ANVIL_PRIVATE_KEYS[0])
            .execute()
            .await
            .expect("Failed to execute test");

        // Setup the project structure
        harness.setup().expect("Failed to setup Hardhat project");

        // Execute the runbook
        let result = result.execute().await;
        
        // Verify execution succeeded
        assert!(result.is_ok(), "Runbook execution failed: {:?}", result);
        
        let execution_result = result.unwrap();
        assert!(execution_result.success, "Runbook execution was not successful");
        
        // Verify deployment outputs
        assert!(
            execution_result.outputs.contains_key("contract_address"),
            "Contract address not found for Hardhat deployment"
        );
        
        assert!(
            execution_result.outputs.contains_key("tx_hash"),
            "Transaction hash not found for Hardhat deployment"
        );
        
        // Verify deployment status
        if let Some(Value::Integer(status)) = execution_result.outputs.get("deployment_status") {
            assert_eq!(*status, 1i128, "Hardhat deployment failed with status 0");
        }
    });
}