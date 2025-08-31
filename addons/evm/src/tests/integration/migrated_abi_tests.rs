//! ABI encoding/decoding tests using txtx framework with filesystem fixtures

#[cfg(test)]
mod abi_tests {
    use crate::tests::fixture_builder::{MigrationHelper, TestResult};
    use crate::tests::integration::anvil_harness::AnvilInstance;
    use std::path::PathBuf;
    use tokio;

    #[tokio::test]
    async fn test_complex_abi_encoding() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("Warning: Skipping test - Anvil not installed");
            return;
        }

        println!("Testing complex ABI encoding with structs and arrays");

        // Use existing complex_types fixture
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/abi/complex_types.tx");
        
        let runbook = std::fs::read_to_string(&fixture_path)
            .expect("Failed to read fixture");

        // Original inline runbook kept for reference
        let _original_runbook = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

signer "deployer" "evm::secret_key" {
    secret_key = input.deployer_private_key
}

# Deploy a contract that accepts complex types
variable "complex_contract" {
    value = {
        bytecode = "0x608060405234801561001057600080fd5b50610474806100206000396000f3fe608060405234801561001057600080fd5b50600436106100415760003560e01c80632e1a7d4d14610046578063b8966710146100625780639c6f1a2a14610092575b600080fd5b610060600480360381019061005b91906102e0565b6100ae565b005b61007c600480360381019061007791906102e0565b6100fb565b6040516100899190610318565b60405180910390f35b6100ac60048036038101906100a791906103a8565b610112565b005b806000808282546100bf9190610437565b9250508190555050565b600080610106610290565b90508091505092915050565b50505050565b600081519050919050565b600082825260208201905092915050565b60005b83811015610153578082015181840152602081019050610138565b60008484015250505050565b6000601f19601f8301169050919050565b600061017c82610119565b6101868185610124565b9350610196818560208601610135565b61019f8161015f565b840191505092915050565b600060208201905081810360008301526101c48184610170565b905092915050565b600073ffffffffffffffffffffffffffffffffffffffff82169050919050565b60006101f7826101cc565b9050919050565b610207816101ec565b82525050565b6000602082019050610222600083018461"
        abi = evm::json_encode([
            {
                "name": "processOrder",
                "type": "function",
                "inputs": [
                    {
                        "name": "order",
                        "type": "tuple",
                        "components": [
                            {"name": "orderId", "type": "uint256"},
                            {"name": "buyer", "type": "address"},
                            {"name": "amounts", "type": "uint256[]"}
                        ]
                    }
                ],
                "outputs": [],
                "stateMutability": "nonpayable"
            },
            {
                "name": "processMultipleAddresses",
                "type": "function",
                "inputs": [
                    {"name": "addresses", "type": "address[]"},
                    {"name": "amounts", "type": "uint256[]"}
                ],
                "outputs": [],
                "stateMutability": "nonpayable"
            },
            {
                "name": "getBalance",
                "type": "function",
                "inputs": [{"name": "account", "type": "address"}],
                "outputs": [{"name": "", "type": "uint256"}],
                "stateMutability": "view"
            }
        ])
    }
}

action "deploy" "evm::deploy_contract" {
    contract = variable.complex_contract
    signer = signer.deployer
    confirmations = 0
}

# Test calling with struct parameter
variable "order_struct" {
    value = [
        42,  # orderId
        "0x70997970C51812dc3A010C7d01b50e0d17dc79C8",  # buyer
        [100, 200, 300]  # amounts array
    ]
}

action "call_with_struct" "evm::call_contract" {
    contract_address = action.deploy.contract_address
    contract_abi = variable.complex_contract.abi
    function_name = "processOrder"
    function_args = [variable.order_struct]
    signer = signer.deployer
    confirmations = 1
}

# Test calling with multiple arrays
variable "addresses_list" {
    value = [
        "0x70997970C51812dc3A010C7d01b50e0d17dc79C8",
        "0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC",
        "0x90F79bf6EB2c4f870365E785982E1f101E93b906"
    ]
}

variable "amounts_list" {
    value = [1000, 2000, 3000]
}

action "call_with_arrays" "evm::call_contract" {
    contract_address = action.deploy.contract_address
    contract_abi = variable.complex_contract.abi
    function_name = "processMultipleAddresses"
    function_args = [variable.addresses_list, variable.amounts_list]
    signer = signer.deployer
    confirmations = 1
}

output "struct_call_tx" {
    value = action.call_with_struct.tx_hash
}

output "arrays_call_tx" {
    value = action.call_with_arrays.tx_hash
}
"#;

        let result = ProjectTestHarness::new_foundry("complex_abi_test.tx", runbook.to_string())
            .with_anvil()
            .with_input("deployer_private_key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
            .execute()
            .await
            .expect("Failed to execute test");

        match result.execute().await {
            Ok(result) => {
                assert!(result.success, "Complex ABI calls should succeed");
                
                println!("Complex ABI encoding test passed");
                println!("   Struct call tx: {:?}", result.outputs.get("struct_call_tx"));
                println!("   Arrays call tx: {:?}", result.outputs.get("arrays_call_tx"));
            }
            Err(e) => panic!("Complex ABI test failed: {}", e),
        }
    }

    #[tokio::test]
    async fn test_abi_edge_cases() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test - Anvil not installed");
            return;
        }

        println!("🔧 Testing ABI edge cases");

        let runbook = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

signer "deployer" "evm::secret_key" {
    secret_key = input.deployer_private_key
}

# Test various edge cases
variable "edge_case_contract" {
    value = {
        bytecode = "0x608060405234801561001057600080fd5b506103e8806100206000396000f3fe608060405234801561001057600080fd5b50600436106100415760003560e01c80631f8b93e214610046578063522bb70414610062578063d5dcf1271461007e575b600080fd5b610060600480360381019061005b91906102c4565b61009a565b005b61007c60048036038101906100779190610318565b6100a4565b005b61009860048036038101906100939190610385565b6100ae565b005b8060008190555050565b8060018190555050565b600082905050505050565b600080fd5b6000819050919050565b6100d1816100be565b81146100dc57600080fd5b50565b6000813590506100ee816100c8565b92915050565b60006020828403121561010a576101096100b9565b5b6000610118848285016100df565b91505092915050565b600073ffffffffffffffffffffffffffffffffffffffff82169050919050565b600061014c82610121565b9050919050565b61015c81610142565b811461016757600080fd5b50565b60008135905061017981610153565b92915050565b600060208284031215610195576101946100b9565b5b60006101a38482850161016a565b91505092915050565b600080fd5b600080fd5b6000601f19601f8301169050919050565b7f4e487b7100000000000000000000000000000000000000000000000000000000600052604160045260246000fd5b610200826101b6565b810181811067ffffffffffffffff8211171561021f5761021e6101c7565b5b80604052505050565b60006102326102b5565b905061023e82826101f6565b919050565b600067ffffffffffffffff82111561025e5761025d6101c7565b5b610267826101b6565b9050602081019050919050565b82818337600083830152505050565b600061029661029184610243565b610228565b9050828152602081018484840111156102b2576102b16101b1565b5b6102bd848285610274565b509392505050565b600082601f8301126102da576102d96101ac565b5b81356102ea848260208601610283565b91505092915050565b600080600060608486031215610"
        abi = evm::json_encode([
            {
                "name": "acceptEmptyBytes",
                "type": "function",
                "inputs": [{"name": "data", "type": "bytes"}],
                "outputs": [],
                "stateMutability": "nonpayable"
            },
            {
                "name": "acceptEmptyArray",
                "type": "function",
                "inputs": [{"name": "numbers", "type": "uint256[]"}],
                "outputs": [],
                "stateMutability": "nonpayable"
            },
            {
                "name": "acceptZeroAddress",
                "type": "function",
                "inputs": [{"name": "addr", "type": "address"}],
                "outputs": [],
                "stateMutability": "nonpayable"
            },
            {
                "name": "acceptBytes32",
                "type": "function",
                "inputs": [{"name": "data", "type": "bytes32"}],
                "outputs": [],
                "stateMutability": "nonpayable"
            }
        ])
    }
}

action "deploy" "evm::deploy_contract" {
    contract = variable.edge_case_contract
    signer = signer.deployer
    confirmations = 0
}

# Test with empty bytes
action "call_empty_bytes" "evm::call_contract" {
    contract_address = action.deploy.contract_address
    contract_abi = variable.edge_case_contract.abi
    function_name = "acceptEmptyBytes"
    function_args = ["0x"]
    signer = signer.deployer
    confirmations = 1
}

# Test with empty array
action "call_empty_array" "evm::call_contract" {
    contract_address = action.deploy.contract_address
    contract_abi = variable.edge_case_contract.abi
    function_name = "acceptEmptyArray"
    function_args = [[]]
    signer = signer.deployer
    confirmations = 1
}

# Test with zero address
action "call_zero_address" "evm::call_contract" {
    contract_address = action.deploy.contract_address
    contract_abi = variable.edge_case_contract.abi
    function_name = "acceptZeroAddress"
    function_args = ["0x0000000000000000000000000000000000000000"]
    signer = signer.deployer
    confirmations = 1
}

# Test with bytes32 (full 32 bytes)
variable "bytes32_value" {
    value = "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
}

action "call_bytes32" "evm::call_contract" {
    contract_address = action.deploy.contract_address
    contract_abi = variable.edge_case_contract.abi
    function_name = "acceptBytes32"
    function_args = [variable.bytes32_value]
    signer = signer.deployer
    confirmations = 1
}

output "empty_bytes_tx" {
    value = action.call_empty_bytes.tx_hash
}

output "empty_array_tx" {
    value = action.call_empty_array.tx_hash
}

output "zero_address_tx" {
    value = action.call_zero_address.tx_hash
}

output "bytes32_tx" {
    value = action.call_bytes32.tx_hash
}
"#;

        let result = ProjectTestHarness::new_foundry("abi_edge_cases.tx", runbook.to_string())
            .with_anvil()
            .with_input("deployer_private_key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
            .execute()
            .await
            .expect("Failed to execute test");

        match result.execute().await {
            Ok(result) => {
                assert!(result.success, "ABI edge case calls should succeed");
                
                println!("ABI edge cases handled correctly");
                assert!(result.outputs.contains_key("empty_bytes_tx"), "Empty bytes call should work");
                assert!(result.outputs.contains_key("empty_array_tx"), "Empty array call should work");
                assert!(result.outputs.contains_key("zero_address_tx"), "Zero address call should work");
                assert!(result.outputs.contains_key("bytes32_tx"), "Bytes32 call should work");
            }
            Err(e) => panic!("ABI edge case test failed: {}", e),
        }
    }
}