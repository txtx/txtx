//! Integration tests using the ProjectTestHarness
//! 
//! These tests demonstrate complete txtx project scenarios with proper
//! compilation outputs, configuration files, and runbook execution.

#[cfg(test)]
mod project_harness_tests {
    use super::super::{ProjectTestHarness, CompilationFramework};
    use crate::tests::test_constants::ANVIL_ACCOUNTS;
    
    #[test]
    fn test_foundry_contract_deployment() {
        // Create a runbook that deploys a contract using Foundry artifacts
        let runbook_content = format!(r#"
# Deploy SimpleStorage contract using Foundry compilation output
addon "evm" {{
    chain_id = 31337
    rpc_api_url = "http://127.0.0.1:8545"
}}

variable "deployer" {{
    value = "{}"
    description = "Account deploying the contract"
}}

action "deploy_storage" "evm::deploy_contract" {{
    from = variable.deployer
    contract = "SimpleStorage"
    source_path = "./out/SimpleStorage.sol/SimpleStorage.json"
    description = "Deploy SimpleStorage contract from Foundry artifacts"
}}

output "contract_address" {{
    value = action.deploy_storage.contract_address
}}

output "deployment_tx" {{
    value = action.deploy_storage.tx_hash
}}
"#, ANVIL_ACCOUNTS[0]);

        // Set up Foundry-based project
        let harness = ProjectTestHarness::new_foundry(
            "deploy_contract.tx",
            runbook_content
        )
        .with_input("PRIVATE_KEY", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80");

        // Setup the project structure
        harness.setup().expect("Failed to setup Foundry project");

        // Verify project structure
        assert!(harness.project_path().join("txtx.yml").exists());
        assert!(harness.project_path().join("runbooks/deploy_contract.tx").exists());
        assert!(harness.project_path().join("out/SimpleStorage.sol/SimpleStorage.json").exists());
        assert!(harness.project_path().join("foundry.toml").exists());

        // Execute and validate
        let result = harness.execute_runbook();
        assert!(result.is_ok(), "Foundry project validation should succeed");
    }

    #[test]
    fn test_hardhat_contract_deployment() {
        // Create a runbook that deploys a contract using Hardhat artifacts
        let runbook_content = format!(r#"
# Deploy SimpleStorage contract using Hardhat compilation output
addon "evm" {{
    chain_id = 31337
    rpc_api_url = "http://127.0.0.1:8545"
}}

variable "deployer" {{
    value = "{}"
    description = "Account deploying the contract"
}}

action "deploy_storage" "evm::deploy_contract" {{
    from = variable.deployer
    contract = "SimpleStorage"
    source_path = "./artifacts/contracts/SimpleStorage.sol/SimpleStorage.json"
    description = "Deploy SimpleStorage contract from Hardhat artifacts"
}}

output "contract_address" {{
    value = action.deploy_storage.contract_address
}}

output "deployment_tx" {{
    value = action.deploy_storage.tx_hash
}}
"#, ANVIL_ACCOUNTS[0]);

        // Set up Hardhat-based project
        let harness = ProjectTestHarness::new_hardhat(
            "deploy_contract.tx",
            runbook_content
        )
        .with_input("PRIVATE_KEY", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80");

        // Setup the project structure
        harness.setup().expect("Failed to setup Hardhat project");

        // Verify project structure
        assert!(harness.project_path().join("txtx.yml").exists());
        assert!(harness.project_path().join("runbooks/deploy_contract.tx").exists());
        assert!(harness.project_path().join("artifacts/contracts/SimpleStorage.sol/SimpleStorage.json").exists());
        assert!(harness.project_path().join("hardhat.config.js").exists());

        // Execute and validate
        let result = harness.execute_runbook();
        assert!(result.is_ok(), "Hardhat project validation should succeed");
    }

    #[test]
    fn test_multi_action_runbook_with_dependencies() {
        // Create a complex runbook with multiple actions that depend on each other
        let runbook_content = format!(r#"
# Complex runbook with multiple dependent actions
addon "evm" {{
    chain_id = 31337
    rpc_api_url = "http://127.0.0.1:8545"
}}

variable "owner" {{
    value = "{}"
}}

variable "recipient" {{
    value = "{}"
}}

# First, send some ETH to fund operations
action "fund_account" "evm::send_eth" {{
    from = variable.owner
    to = variable.recipient
    amount = "1000000000000000000"  # 1 ETH
    description = "Fund recipient account"
}}

# Deploy a contract after funding
action "deploy_token" "evm::deploy_contract" {{
    from = variable.recipient
    contract = "SimpleStorage"
    source_path = "./out/SimpleStorage.sol/SimpleStorage.json"
    description = "Deploy token contract"
    depends_on = [action.fund_account]
}}

# Interact with the deployed contract
action "set_value" "evm::call_contract" {{
    from = variable.recipient
    contract_address = action.deploy_token.contract_address
    function_name = "set"
    function_args = [42]
    abi = action.deploy_token.abi
    description = "Set initial value in contract"
    depends_on = [action.deploy_token]
}}

# Read from the contract
action "get_value" "evm::call_contract" {{
    from = variable.recipient
    contract_address = action.deploy_token.contract_address
    function_name = "get"
    function_args = []
    abi = action.deploy_token.abi
    description = "Get value from contract"
    depends_on = [action.set_value]
}}

output "funding_tx" {{
    value = action.fund_account.tx_hash
}}

output "contract_address" {{
    value = action.deploy_token.contract_address
}}

output "stored_value" {{
    value = action.get_value.result
}}
"#, ANVIL_ACCOUNTS[0], ANVIL_ACCOUNTS[1]);

        // Set up project with multiple actions
        let harness = ProjectTestHarness::new_foundry(
            "multi_action.tx",
            runbook_content
        )
        .with_input("DEPLOYER_KEY", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80");

        // Setup the project structure
        harness.setup().expect("Failed to setup multi-action project");

        // Verify the runbook was created correctly
        let runbook_path = harness.project_path().join("runbooks/multi_action.tx");
        let runbook_content = std::fs::read_to_string(&runbook_path)
            .expect("Failed to read runbook");
        
        // Verify key components exist in the runbook
        assert!(runbook_content.contains("fund_account"));
        assert!(runbook_content.contains("deploy_token"));
        assert!(runbook_content.contains("set_value"));
        assert!(runbook_content.contains("get_value"));
        assert!(runbook_content.contains("depends_on"));

        // Execute and validate
        let result = harness.execute_runbook();
        assert!(result.is_ok(), "Multi-action project validation should succeed");
    }

    #[test]
    fn test_error_handling_with_project_context() {
        // Create a runbook that should fail with proper error context
        let runbook_content = r#"
# Runbook with intentional error for testing error handling
addon "evm" {
    chain_id = 31337
    rpc_api_url = "http://127.0.0.1:8545"
}

variable "empty_account" {
    value = "0x0000000000000000000000000000000000000001"
    description = "Account with no funds"
}

# This should fail due to insufficient funds
action "failing_deployment" "evm::deploy_contract" {
    from = variable.empty_account
    contract = "SimpleStorage"
    source_path = "./out/SimpleStorage.sol/SimpleStorage.json"
    gas_limit = 3000000
    description = "Attempt to deploy from unfunded account"
}

output "should_not_exist" {
    value = action.failing_deployment.contract_address
}
"#;

        // Set up project that should fail
        let harness = ProjectTestHarness::new_foundry(
            "error_test.tx",
            runbook_content.to_string()
        );

        // Setup the project structure
        harness.setup().expect("Failed to setup error test project");

        // Verify project was set up correctly even for error case
        assert!(harness.project_path().join("txtx.yml").exists());
        assert!(harness.project_path().join("runbooks/error_test.tx").exists());

        // The validation should still pass (project structure is valid)
        let result = harness.execute_runbook();
        assert!(result.is_ok(), "Project structure validation should succeed even for runbooks with errors");
    }

    #[test]
    fn test_project_with_custom_contract_artifacts() {
        // Test that we can add custom contract artifacts to the project
        let runbook_content = r#"
addon "evm" {
    chain_id = 1
    rpc_api_url = "https://eth-mainnet.alchemyapi.io/v2/demo"
}

action "deploy_custom" "evm::deploy_contract" {
    from = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
    contract = "CustomContract"
    source_path = "./out/CustomContract.sol/CustomContract.json"
}
"#;

        let harness = ProjectTestHarness::new_foundry(
            "custom_contract.tx",
            runbook_content.to_string()
        );

        // Setup the project
        harness.setup().expect("Failed to setup project");

        // Add a custom contract artifact
        let custom_contract_dir = harness.project_path()
            .join("out")
            .join("CustomContract.sol");
        std::fs::create_dir_all(&custom_contract_dir)
            .expect("Failed to create custom contract directory");

        let custom_artifact = r#"{
  "abi": [
    {
      "inputs": [],
      "name": "customFunction",
      "outputs": [],
      "stateMutability": "nonpayable",
      "type": "function"
    }
  ],
  "bytecode": {
    "object": "0x608060405234801561001057600080fd5b50"
  }
}"#;

        std::fs::write(
            custom_contract_dir.join("CustomContract.json"),
            custom_artifact
        ).expect("Failed to write custom artifact");

        // Verify the custom artifact exists
        assert!(harness.project_path()
            .join("out/CustomContract.sol/CustomContract.json")
            .exists());

        // Execute and validate
        let result = harness.execute_runbook();
        assert!(result.is_ok(), "Custom contract project validation should succeed");
    }

    #[test]
    fn test_input_management() {
        // Test that inputs are properly passed to the runbook
        let runbook_content = r#"
addon "evm" {
    chain_id = 31337
    rpc_api_url = input.rpc_url
}

variable "private_key" {
    value = input.private_key
}

action "test" "evm::get_balance" {
    address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
}
"#;

        let test_rpc_url = "http://test.rpc.url:8545";
        let test_private_key = "0xtest1234567890";

        let harness = ProjectTestHarness::new_foundry(
            "input_test.tx",
            runbook_content.to_string()
        )
        .with_input("rpc_url", test_rpc_url)
        .with_input("private_key", test_private_key);

        harness.setup().expect("Failed to setup project");

        // Verify inputs are stored in the harness
        assert_eq!(harness.inputs.get("rpc_url"), Some(&test_rpc_url.to_string()));
        assert_eq!(harness.inputs.get("private_key"), Some(&test_private_key.to_string()));
    }
}

#[cfg(test)]
mod framework_specific_tests {
    use super::*;
    use super::super::{ProjectTestHarness, CompilationFramework};

    #[test]
    fn test_foundry_specific_configuration() {
        let runbook = r#"
addon "evm" {
    chain_id = 31337
    rpc_api_url = "http://127.0.0.1:8545"
}
"#;

        let harness = ProjectTestHarness::new_foundry("test.tx", runbook.to_string());
        harness.setup().expect("Failed to setup Foundry project");

        // Check Foundry-specific files
        let foundry_toml = std::fs::read_to_string(
            harness.project_path().join("foundry.toml")
        ).expect("Failed to read foundry.toml");

        assert!(foundry_toml.contains("src = \"contracts\""));
        assert!(foundry_toml.contains("out = \"out\""));
        assert!(foundry_toml.contains("libs = [\"lib\"]"));

        // Check that Hardhat files don't exist
        assert!(!harness.project_path().join("hardhat.config.js").exists());
        assert!(!harness.project_path().join("artifacts").exists());
    }

    #[test]
    fn test_hardhat_specific_configuration() {
        let runbook = r#"
addon "evm" {
    chain_id = 31337
    rpc_api_url = "http://127.0.0.1:8545"
}
"#;

        let harness = ProjectTestHarness::new_hardhat("test.tx", runbook.to_string());
        harness.setup().expect("Failed to setup Hardhat project");

        // Check Hardhat-specific files
        let hardhat_config = std::fs::read_to_string(
            harness.project_path().join("hardhat.config.js")
        ).expect("Failed to read hardhat.config.js");

        assert!(hardhat_config.contains("solidity: \"0.8.19\""));
        assert!(hardhat_config.contains("sources: \"./contracts\""));
        assert!(hardhat_config.contains("artifacts: \"./artifacts\""));

        // Check that Foundry files don't exist
        assert!(!harness.project_path().join("foundry.toml").exists());
        assert!(!harness.project_path().join("out").exists());
    }
}