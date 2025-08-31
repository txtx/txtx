// Integration test demonstrating the fixture builder in action

#[cfg(test)]
mod tests {
    use super::super::*;
    use std::collections::HashMap;
    
    #[tokio::test]
    async fn test_simple_eth_transfer() {
        // Create a test fixture
        let mut fixture = FixtureBuilder::new("test_eth_transfer")
            .with_environment("testing")
            .build()
            .await
            .expect("Failed to build fixture");
        
        // Define a simple ETH transfer runbook
        let runbook = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

signer "alice" "evm::private_key" {
    private_key = input.alice_secret
}

action "transfer" "evm::send_eth" {
    description = "Transfer 1 ETH from Alice to Bob"
    from = input.alice_address
    to = input.bob_address
    value = "1000000000000000000"  // 1 ETH in wei
    signer = signer.alice
}
"#;
        
        // Add the runbook to the fixture
        fixture.add_runbook("transfer", runbook)
            .expect("Failed to add runbook");
        
        // Execute the runbook
        fixture.execute_runbook("transfer").await
            .expect("Failed to execute runbook");
        
        // Verify outputs exist
        let outputs = fixture.get_outputs("transfer")
            .expect("Failed to get outputs");
        
        // Check that we have the expected outputs
        assert!(outputs.contains_key("transfer_result"), "Missing transfer_result output");
        assert!(outputs.contains_key("test_output"), "Missing test_output");
        assert!(outputs.contains_key("test_metadata"), "Missing test_metadata");
        
        // Verify the transfer result
        if let Some(transfer_result) = outputs.get("transfer_result") {
            // The result should be an object with tx_hash
            match transfer_result {
                txtx_addon_kit::types::types::Value::Object(map) => {
                    assert!(map.contains_key("tx_hash"), "Missing tx_hash in result");
                    assert!(map.contains_key("success"), "Missing success flag");
                },
                _ => panic!("Expected transfer_result to be an object")
            }
        }
    }
    
    #[tokio::test]
    async fn test_contract_deployment() {
        let mut fixture = FixtureBuilder::new("test_deploy")
            .with_environment("testing")
            .build()
            .await
            .expect("Failed to build fixture");
        
        // Simple storage contract
        let contract_source = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract SimpleStorage {
    uint256 public value;
    
    constructor(uint256 _initial) {
        value = _initial;
    }
    
    function setValue(uint256 _value) public {
        value = _value;
    }
    
    function getValue() public view returns (uint256) {
        return value;
    }
}
"#;
        
        // Add the contract
        fixture.add_contract("SimpleStorage", contract_source)
            .expect("Failed to add contract");
        
        // Deployment runbook
        let runbook = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

signer "deployer" "evm::private_key" {
    private_key = input.alice_secret
}

action "compile" "evm::compile_contract" {
    description = "Compile SimpleStorage contract"
    source_path = "src/SimpleStorage.sol"
}

action "deploy" "evm::deploy_contract" {
    description = "Deploy SimpleStorage with initial value 42"
    from = input.alice_address
    contract = action.compile.bytecode
    constructor_args = [42]
    signer = signer.deployer
}
"#;
        
        fixture.add_runbook("deploy", runbook)
            .expect("Failed to add runbook");
        
        // Execute deployment
        fixture.execute_runbook("deploy").await
            .expect("Failed to execute deployment");
        
        // Verify deployment outputs
        let outputs = fixture.get_outputs("deploy")
            .expect("Failed to get outputs");
        
        assert!(outputs.contains_key("deploy_result"), "Missing deploy_result");
        
        if let Some(deploy_result) = outputs.get("deploy_result") {
            match deploy_result {
                txtx_addon_kit::types::types::Value::Object(map) => {
                    assert!(map.contains_key("contract_address"), "Missing contract_address");
                    assert!(map.contains_key("tx_hash"), "Missing deployment tx_hash");
                },
                _ => panic!("Expected deploy_result to be an object")
            }
        }
    }
    
    #[tokio::test]
    async fn test_snapshot_isolation() {
        let manager = get_anvil_manager().await.unwrap();
        
        // Create two fixtures sharing the same Anvil instance
        let mut fixture1 = FixtureBuilder::new("test_isolation_1")
            .with_anvil_manager(manager.clone())
            .build()
            .await
            .unwrap();
        
        let mut fixture2 = FixtureBuilder::new("test_isolation_2")
            .with_anvil_manager(manager.clone())
            .build()
            .await
            .unwrap();
        
        // Simple transfer runbook
        let runbook = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

signer "alice" "evm::private_key" {
    private_key = input.alice_secret
}

action "transfer" "evm::send_eth" {
    from = input.alice_address
    to = input.bob_address
    value = "1000000000000000000"
    signer = signer.alice
}
"#;
        
        // Add to both fixtures
        fixture1.add_runbook("transfer", runbook).unwrap();
        fixture2.add_runbook("transfer", runbook).unwrap();
        
        // Execute in fixture1
        fixture1.execute_runbook("transfer").await.unwrap();
        
        // Take a checkpoint in fixture1
        let checkpoint1 = fixture1.checkpoint().await.unwrap();
        
        // Execute in fixture2 (should be isolated)
        fixture2.execute_runbook("transfer").await.unwrap();
        
        // Revert fixture1 to checkpoint
        fixture1.revert(&checkpoint1).await.unwrap();
        
        // Execute again in fixture1 - should succeed because state was reverted
        fixture1.execute_runbook("transfer").await.unwrap();
    }
    
    #[tokio::test]
    async fn test_template_usage() {
        // Create a fixture with a template
        let mut fixture = FixtureBuilder::new("test_template")
            .with_template("erc20_transfer")
            .with_parameter("token_address", "0x123...")
            .with_parameter("recipient", "0x456...")
            .build()
            .await
            .expect("Failed to build fixture");
        
        // The template should have created appropriate runbooks
        // This test would work once templates are implemented
        
        // For now, just verify the fixture was created
        assert!(fixture.project_dir.exists());
    }
    
    #[tokio::test]
    async fn test_multi_action_runbook() {
        let mut fixture = FixtureBuilder::new("test_multi_action")
            .build()
            .await
            .unwrap();
        
        let runbook = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

signer "alice" "evm::private_key" {
    private_key = input.alice_secret
}

action "transfer1" "evm::send_eth" {
    description = "First transfer"
    from = input.alice_address
    to = input.bob_address
    value = "1000000000000000000"
    signer = signer.alice
}

action "transfer2" "evm::send_eth" {
    description = "Second transfer"
    from = input.alice_address
    to = input.charlie_address
    value = "2000000000000000000"
    signer = signer.alice
}

action "transfer3" "evm::send_eth" {
    description = "Third transfer"
    from = input.alice_address
    to = input.dave_address
    value = "3000000000000000000"
    signer = signer.alice
}
"#;
        
        fixture.add_runbook("multi", runbook).unwrap();
        fixture.execute_runbook("multi").await.unwrap();
        
        let outputs = fixture.get_outputs("multi").unwrap();
        
        // Verify all three transfers have outputs
        assert!(outputs.contains_key("transfer1_result"));
        assert!(outputs.contains_key("transfer2_result"));
        assert!(outputs.contains_key("transfer3_result"));
        
        // Verify the test_metadata contains all three actions
        if let Some(metadata) = outputs.get("test_metadata") {
            match metadata {
                txtx_addon_kit::types::types::Value::Object(map) => {
                    assert_eq!(map.len(), 3, "Should have metadata for 3 actions");
                    assert!(map.contains_key("transfer1"));
                    assert!(map.contains_key("transfer2"));
                    assert!(map.contains_key("transfer3"));
                },
                _ => panic!("Expected test_metadata to be an object")
            }
        }
    }
}