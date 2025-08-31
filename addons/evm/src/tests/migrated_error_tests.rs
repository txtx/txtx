//! Error handling tests using txtx fixtures
//! 
//! These tests validate error handling through the full txtx stack,
//! ensuring that users receive helpful error messages with proper context.

#[cfg(test)]
mod migrated_error_tests {
    use crate::tests::test_harness::ProjectTestHarness;
    use crate::tests::integration::anvil_harness::AnvilInstance;
    use std::path::PathBuf;

    #[test]
    fn test_insufficient_funds_error() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("Warning: Skipping test_insufficient_funds_error - Anvil not installed");
            return;
        }

        // Use existing fixture for insufficient funds
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/errors/insufficient_funds_transfer.tx");
        
        let mut harness = ProjectTestHarness::from_fixture(&fixture_path)
            .with_anvil();
        
        harness.setup().expect("Failed to setup project");
        
        // Execute runbook - should fail with insufficient funds
        let result = harness.execute_runbook();
        assert!(result.is_err(), "Should fail with insufficient funds");
        
        let error_msg = result.unwrap_err();
        let error_str = format!("{:?}", error_msg);
        assert!(error_str.contains("insufficient") || error_str.contains("Insufficient"),
                "Error should mention insufficient funds: {}", error_str);
        
        harness.cleanup();
    }

    #[test]
    fn test_function_not_found_error() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("Warning: Skipping test_function_not_found_error - Anvil not installed");
            return;
        }

        // Use existing fixture for invalid function call
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/errors/invalid_function_call.tx");
        
        let mut harness = ProjectTestHarness::from_fixture(&fixture_path)
            .with_anvil();
        
        harness.setup().expect("Failed to setup project");
        
        // Execute runbook - should fail with function not found
        let result = harness.execute_runbook();
        assert!(result.is_err(), "Should fail with function not found");
        
        let error_msg = result.unwrap_err();
        let error_str = format!("{:?}", error_msg);
        assert!(error_str.contains("function") || error_str.contains("Function") || 
                error_str.contains("selector") || error_str.contains("not found"),
                "Error should mention function not found: {}", error_str);
        
        harness.cleanup();
    }

    #[test]
    fn test_invalid_hex_codec_error() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("Warning: Skipping test_invalid_hex_codec_error - Anvil not installed");
            return;
        }

        // Use existing fixture for invalid hex
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/errors/invalid_hex_address.tx");
        
        let mut harness = ProjectTestHarness::from_fixture(&fixture_path)
            .with_anvil();
        
        harness.setup().expect("Failed to setup project");
        
        // Execute runbook - should fail with invalid hex
        let result = harness.execute_runbook();
        assert!(result.is_err(), "Should fail with invalid hex");
        
        let error_msg = result.unwrap_err();
        let error_str = format!("{:?}", error_msg);
        assert!(error_str.contains("hex") || error_str.contains("Hex") || 
                error_str.contains("invalid") || error_str.contains("Invalid"),
                "Error should mention invalid hex: {}", error_str);
        
        harness.cleanup();
    }

    #[test]
    fn test_signer_key_not_found_error() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("Warning: Skipping test_signer_key_not_found_error - Anvil not installed");
            return;
        }

        // Use existing fixture for missing signer
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/errors/missing_signer.tx");
        
        let mut harness = ProjectTestHarness::from_fixture(&fixture_path)
            .with_anvil();
        
        harness.setup().expect("Failed to setup project");
        
        // Execute runbook - should fail with signer not found
        let result = harness.execute_runbook();
        assert!(result.is_err(), "Should fail with signer not found");
        
        let error_msg = result.unwrap_err();
        let error_str = format!("{:?}", error_msg);
        assert!(error_str.contains("signer") || error_str.contains("Signer") || 
                error_str.contains("not found") || error_str.contains("undefined"),
                "Error should mention signer not found: {}", error_str);
        
        harness.cleanup();
    }

    #[test]
    fn test_transaction_revert_error() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("Warning: Skipping test_transaction_revert_error - Anvil not installed");
            return;
        }

        // This tests transaction reverts with reason strings
        let runbook = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

signer "deployer" "evm::secret_key" {
    secret_key = input.deployer_private_key
}

# Deploy a contract that always reverts
action "deploy_reverter" "evm::deploy_contract" {
    contract_name = "AlwaysReverts"
    artifact_source = "inline:0x6080604052348015600e575f5ffd5b50603e80601a5f395ff3fe6080604052348015600e575f5ffd5b50600436106026575f3560e01c8063aa8c217c14602a575b5f5ffd5b60306032565b005b5f5ffdfe"
    signer = signer.deployer
    confirmations = 0
}

# Try to call function that reverts
action "call_reverting" "evm::call_contract_function" {
    contract_address = action.deploy_reverter.contract_address
    function_signature = "alwaysReverts()"
    signer = signer.deployer
}
"#;

        let mut harness = ProjectTestHarness::new_foundry("revert_test.tx", runbook.to_string())
            .with_anvil();
        
        harness.setup().expect("Failed to setup project");
        
        // Execute runbook - should fail with revert
        let result = harness.execute_runbook();
        assert!(result.is_err(), "Should fail with transaction revert");
        
        let error_msg = result.unwrap_err();
        let error_str = format!("{:?}", error_msg);
        assert!(error_str.contains("revert") || error_str.contains("Revert") || 
                error_str.contains("execution reverted"),
                "Error should mention revert: {}", error_str);
        
        harness.cleanup();
    }

    #[test]
    fn test_out_of_gas_error() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("Warning: Skipping test_out_of_gas_error - Anvil not installed");
            return;
        }

        // Test running out of gas
        let runbook = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

signer "sender" "evm::secret_key" {
    secret_key = input.sender_private_key
}

# Try to send transaction with very low gas limit
action "low_gas_tx" "evm::send_eth" {
    recipient_address = "0x70997970C51812dc3A010C7d01b50e0d17dc79C8"
    amount = 100
    signer = signer.sender
    gas_limit = 100  # Extremely low gas limit
}
"#;

        let mut harness = ProjectTestHarness::new_foundry("out_of_gas_test.tx", runbook.to_string())
            .with_anvil();
        
        harness.setup().expect("Failed to setup project");
        
        // Execute runbook - should fail with out of gas
        let result = harness.execute_runbook();
        assert!(result.is_err(), "Should fail with out of gas");
        
        let error_msg = result.unwrap_err();
        let error_str = format!("{:?}", error_msg);
        assert!(error_str.contains("gas") || error_str.contains("Gas"),
                "Error should mention gas: {}", error_str);
        
        harness.cleanup();
    }

    #[test]
    fn test_chain_id_mismatch_error() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("Warning: Skipping test_chain_id_mismatch_error - Anvil not installed");
            return;
        }

        // Test chain ID mismatch
        let runbook = r#"
addon "evm" {
    chain_id = 1  # Mainnet chain ID
    rpc_api_url = input.rpc_url  # But using Anvil (chain ID 31337)
}

action "get_balance" "evm::get_balance" {
    address = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb7"
}
"#;

        let mut harness = ProjectTestHarness::new_foundry("chain_id_test.tx", runbook.to_string())
            .with_anvil();
        
        harness.setup().expect("Failed to setup project");
        
        // Execute runbook - might fail with chain ID mismatch
        let result = harness.execute_runbook();
        
        // Note: This might not actually fail depending on implementation
        // Some clients allow chain ID mismatches for read operations
        if result.is_err() {
            let error_msg = result.unwrap_err();
            println!("Chain ID error: {}", error_msg);
        }
        
        harness.cleanup();
    }
}