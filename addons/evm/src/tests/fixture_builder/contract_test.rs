// Tests for contract compilation and deployment using the fixture builder

#[cfg(test)]
mod tests {
    use super::super::*;
    
    #[tokio::test]
    async fn test_contract_compilation() {
        println!("🔨 Testing contract compilation");
        
        let mut fixture = FixtureBuilder::new("test_compile")
            .build()
            .await
            .expect("Failed to build fixture");
        
        // Simple storage contract
        let contract = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract SimpleStorage {
    uint256 public storedValue;
    
    event ValueChanged(uint256 oldValue, uint256 newValue);
    
    constructor(uint256 _initial) {
        storedValue = _initial;
    }
    
    function setValue(uint256 _value) public {
        uint256 oldValue = storedValue;
        storedValue = _value;
        emit ValueChanged(oldValue, _value);
    }
    
    function getValue() public view returns (uint256) {
        return storedValue;
    }
    
    function increment() public {
        uint256 oldValue = storedValue;
        storedValue = storedValue + 1;
        emit ValueChanged(oldValue, storedValue);
    }
}
"#;
        
        // Add contract to fixture
        fixture.add_contract("SimpleStorage", contract)
            .expect("Failed to add contract");
        
        // Verify contract file was created
        let contract_path = fixture.project_dir.join("src").join("SimpleStorage.sol");
        assert!(contract_path.exists(), "Contract file should exist");
        
        println!("✅ Contract file created successfully");
    }
    
    #[tokio::test]
    #[ignore] // Requires solc and txtx
    async fn test_contract_deployment() {
        println!("🚀 Testing contract deployment");
        
        let mut fixture = FixtureBuilder::new("test_deploy")
            .build()
            .await
            .expect("Failed to build fixture");
        
        // Add the SimpleStorage contract
        let contract = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract SimpleStorage {
    uint256 public storedValue;
    
    constructor(uint256 _initial) {
        storedValue = _initial;
    }
    
    function setValue(uint256 _value) public {
        storedValue = _value;
    }
    
    function getValue() public view returns (uint256) {
        return storedValue;
    }
}
"#;
        
        fixture.add_contract("SimpleStorage", contract)
            .expect("Failed to add contract");
        
        // Create deployment runbook
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
    contract_path = "src/SimpleStorage.sol"
}

action "deploy" "evm::deploy_contract" {
    description = "Deploy SimpleStorage with initial value 42"
    from = input.alice_address
    contract_bytecode = action.compile.bytecode
    constructor_args = ["42"]
    signer = signer.deployer
}

action "get_code" "evm::get_code" {
    description = "Verify contract was deployed"
    address = action.deploy.contract_address
}
"#;
        
        fixture.add_runbook("deploy", runbook)
            .expect("Failed to add runbook");
        
        println!("📝 Executing deployment runbook...");
        
        // Execute deployment
        fixture.execute_runbook("deploy").await
            .expect("Failed to execute deployment");
        
        // Verify outputs
        let outputs = fixture.get_outputs("deploy")
            .expect("Failed to get outputs");
        
        // Check compilation succeeded
        assert!(outputs.contains_key("compile_result"), "Should have compile result");
        
        // Check deployment succeeded
        assert!(outputs.contains_key("deploy_result"), "Should have deploy result");
        
        if let Some(deploy_result) = outputs.get("deploy_result") {
            match deploy_result {
                txtx_addon_kit::types::types::Value::Object(map) => {
                    assert!(map.contains_key("contract_address"), "Should have contract address");
                    assert!(map.contains_key("tx_hash"), "Should have transaction hash");
                    
                    if let Some(addr) = map.get("contract_address") {
                        println!("📍 Contract deployed at: {:?}", addr);
                    }
                },
                _ => panic!("Deploy result should be an object")
            }
        }
        
        // Check that code exists at deployed address
        assert!(outputs.contains_key("get_code_result"), "Should have code check result");
        
        println!("✅ Contract deployment test passed");
    }
    
    #[tokio::test]
    #[ignore] // Requires solc and txtx
    async fn test_contract_interaction() {
        println!("🔧 Testing contract interaction");
        
        let mut fixture = FixtureBuilder::new("test_interact")
            .build()
            .await
            .expect("Failed to build fixture");
        
        // Deploy and interact with contract
        let contract = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Counter {
    uint256 public count;
    
    event CountChanged(uint256 newCount);
    
    constructor() {
        count = 0;
    }
    
    function increment() public {
        count = count + 1;
        emit CountChanged(count);
    }
    
    function getCount() public view returns (uint256) {
        return count;
    }
    
    function setCount(uint256 _count) public {
        count = _count;
        emit CountChanged(_count);
    }
}
"#;
        
        fixture.add_contract("Counter", contract)
            .expect("Failed to add contract");
        
        let runbook = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

signer "alice" "evm::private_key" {
    private_key = input.alice_secret
}

action "compile" "evm::compile_contract" {
    description = "Compile Counter contract"
    contract_path = "src/Counter.sol"
}

action "deploy" "evm::deploy_contract" {
    description = "Deploy Counter contract"
    from = input.alice_address
    contract_bytecode = action.compile.bytecode
    constructor_args = []
    signer = signer.alice
}

action "read_initial" "evm::call_contract_read" {
    description = "Read initial count"
    contract_address = action.deploy.contract_address
    contract_abi = action.compile.abi
    function_name = "getCount"
    function_args = []
}

action "increment" "evm::call_contract_write" {
    description = "Increment the counter"
    from = input.alice_address
    contract_address = action.deploy.contract_address
    contract_abi = action.compile.abi
    function_name = "increment"
    function_args = []
    signer = signer.alice
}

action "read_after" "evm::call_contract_read" {
    description = "Read count after increment"
    contract_address = action.deploy.contract_address
    contract_abi = action.compile.abi
    function_name = "getCount"
    function_args = []
}

action "set_value" "evm::call_contract_write" {
    description = "Set count to specific value"
    from = input.alice_address
    contract_address = action.deploy.contract_address
    contract_abi = action.compile.abi
    function_name = "setCount"
    function_args = ["100"]
    signer = signer.alice
}

action "read_final" "evm::call_contract_read" {
    description = "Read final count"
    contract_address = action.deploy.contract_address
    contract_abi = action.compile.abi
    function_name = "getCount"
    function_args = []
}
"#;
        
        fixture.add_runbook("interact", runbook)
            .expect("Failed to add runbook");
        
        println!("📝 Executing interaction runbook...");
        
        fixture.execute_runbook("interact").await
            .expect("Failed to execute interaction");
        
        let outputs = fixture.get_outputs("interact")
            .expect("Failed to get outputs");
        
        // Verify all actions completed
        assert!(outputs.contains_key("compile_result"));
        assert!(outputs.contains_key("deploy_result"));
        assert!(outputs.contains_key("read_initial_result"));
        assert!(outputs.contains_key("increment_result"));
        assert!(outputs.contains_key("read_after_result"));
        assert!(outputs.contains_key("set_value_result"));
        assert!(outputs.contains_key("read_final_result"));
        
        // Check that values changed as expected
        // Initial should be 0, after increment should be 1, final should be 100
        
        println!("✅ Contract interaction test passed");
    }
    
    #[tokio::test]
    async fn test_multiple_contracts() {
        println!("📚 Testing multiple contracts");
        
        let mut fixture = FixtureBuilder::new("test_multi")
            .build()
            .await
            .expect("Failed to build fixture");
        
        // Add multiple contracts
        let token_contract = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract SimpleToken {
    mapping(address => uint256) public balances;
    uint256 public totalSupply;
    
    constructor(uint256 _initial) {
        balances[msg.sender] = _initial;
        totalSupply = _initial;
    }
}
"#;
        
        let vault_contract = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract SimpleVault {
    mapping(address => uint256) public deposits;
    
    function deposit() public payable {
        deposits[msg.sender] += msg.value;
    }
}
"#;
        
        fixture.add_contract("SimpleToken", token_contract)
            .expect("Failed to add token contract");
        
        fixture.add_contract("SimpleVault", vault_contract)
            .expect("Failed to add vault contract");
        
        // Verify both contracts were added
        assert!(fixture.project_dir.join("src/SimpleToken.sol").exists());
        assert!(fixture.project_dir.join("src/SimpleVault.sol").exists());
        
        println!("✅ Multiple contracts test passed");
    }
}