// Helper utilities for fixture-based tests

use txtx_addon_kit::types::types::Value;
use std::collections::HashMap;

/// Extract a string value from outputs
pub fn get_string_output(
    outputs: &HashMap<String, Value>,
    key: &str,
    field: &str
) -> Option<String> {
    outputs.get(key)
        .and_then(|v| match v {
            Value::Object(map) => map.get(field),
            _ => None
        })
        .and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            _ => None
        })
}

/// Extract a boolean value from outputs
pub fn get_bool_output(
    outputs: &HashMap<String, Value>,
    key: &str,
    field: &str
) -> Option<bool> {
    outputs.get(key)
        .and_then(|v| match v {
            Value::Object(map) => map.get(field),
            _ => None
        })
        .and_then(|v| match v {
            Value::Bool(b) => Some(*b),
            _ => None
        })
}

/// Extract an integer value from outputs
pub fn get_int_output(
    outputs: &HashMap<String, Value>,
    key: &str,
    field: &str
) -> Option<i128> {
    outputs.get(key)
        .and_then(|v| match v {
            Value::Object(map) => map.get(field),
            _ => None
        })
        .and_then(|v| match v {
            Value::Integer(i) => Some(*i),
            _ => None
        })
}

/// Assert that an action succeeded
pub fn assert_action_success(
    outputs: &HashMap<String, Value>,
    action_name: &str
) {
    let result_key = format!("{}_result", action_name);
    assert!(
        outputs.contains_key(&result_key),
        "Missing result for action '{}'",
        action_name
    );
    
    // Check success flag if present
    if let Some(success) = get_bool_output(outputs, &result_key, "success") {
        assert!(success, "Action '{}' failed", action_name);
    }
    
    // Check for tx_hash as indicator of success for transactions
    if let Some(tx_hash) = get_string_output(outputs, &result_key, "tx_hash") {
        assert!(!tx_hash.is_empty(), "Action '{}' has empty tx_hash", action_name);
    }
}

/// Assert that a transaction has a valid hash
pub fn assert_has_tx_hash(
    outputs: &HashMap<String, Value>,
    action_name: &str
) -> String {
    let result_key = format!("{}_result", action_name);
    let tx_hash = get_string_output(outputs, &result_key, "tx_hash")
        .expect(&format!("Action '{}' should have tx_hash", action_name));
    
    // Basic validation - should be hex string starting with 0x
    assert!(tx_hash.starts_with("0x"), "Invalid tx_hash format");
    assert!(tx_hash.len() == 66, "Invalid tx_hash length"); // 0x + 64 hex chars
    
    tx_hash
}

/// Assert that a deployment has a contract address
pub fn assert_has_contract_address(
    outputs: &HashMap<String, Value>,
    action_name: &str
) -> String {
    let result_key = format!("{}_result", action_name);
    let address = get_string_output(outputs, &result_key, "contract_address")
        .expect(&format!("Action '{}' should have contract_address", action_name));
    
    // Basic validation - should be hex string starting with 0x
    assert!(address.starts_with("0x"), "Invalid address format");
    assert!(address.len() == 42, "Invalid address length"); // 0x + 40 hex chars
    
    address
}

/// Common test contracts
pub mod contracts {
    /// Simple storage contract
    pub const SIMPLE_STORAGE: &str = r#"
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

    /// ERC20-like token contract
    pub const SIMPLE_TOKEN: &str = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract SimpleToken {
    mapping(address => uint256) public balanceOf;
    uint256 public totalSupply;
    
    event Transfer(address indexed from, address indexed to, uint256 value);
    
    constructor(uint256 _initialSupply) {
        balanceOf[msg.sender] = _initialSupply;
        totalSupply = _initialSupply;
    }
    
    function transfer(address _to, uint256 _value) public returns (bool) {
        require(balanceOf[msg.sender] >= _value, "Insufficient balance");
        balanceOf[msg.sender] -= _value;
        balanceOf[_to] += _value;
        emit Transfer(msg.sender, _to, _value);
        return true;
    }
}
"#;

    /// Counter contract for testing interactions
    pub const COUNTER: &str = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Counter {
    uint256 public count;
    
    event CountChanged(uint256 newCount);
    
    function increment() public {
        count += 1;
        emit CountChanged(count);
    }
    
    function decrement() public {
        require(count > 0, "Counter cannot go below zero");
        count -= 1;
        emit CountChanged(count);
    }
    
    function setCount(uint256 _count) public {
        count = _count;
        emit CountChanged(_count);
    }
}
"#;
}

/// Common runbook templates
pub mod templates {
    /// Basic ETH transfer template
    pub fn eth_transfer(from: &str, to: &str, amount: &str) -> String {
        format!(r#"
addon "evm" {{
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}}

signer "sender" "evm::private_key" {{
    private_key = input.{}_secret
}}

action "transfer" "evm::send_eth" {{
    from = input.{}_address
    to = input.{}_address
    value = "{}"
    signer = signer.sender
}}
"#, from, from, to, amount)
    }
    
    /// Contract deployment template
    pub fn deploy_contract(contract_name: &str, deployer: &str) -> String {
        format!(r#"
addon "evm" {{
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}}

signer "deployer" "evm::private_key" {{
    private_key = input.{}_secret
}}

action "compile" "evm::compile_contract" {{
    contract_path = "src/{}.sol"
}}

action "deploy" "evm::deploy_contract" {{
    from = input.{}_address
    contract_bytecode = action.compile.bytecode
    constructor_args = []
    signer = signer.deployer
}}
"#, deployer, contract_name, deployer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_output_extraction() {
        use txtx_addon_kit::indexmap::IndexMap;
        
        let mut outputs = HashMap::new();
        let mut inner = IndexMap::new();
        inner.insert("tx_hash".to_string(), Value::String("0x123".to_string()));
        inner.insert("success".to_string(), Value::Bool(true));
        inner.insert("gas_used".to_string(), Value::Integer(21000));
        outputs.insert("transfer_result".to_string(), Value::Object(inner));
        
        assert_eq!(
            get_string_output(&outputs, "transfer_result", "tx_hash"),
            Some("0x123".to_string())
        );
        
        assert_eq!(
            get_bool_output(&outputs, "transfer_result", "success"),
            Some(true)
        );
        
        assert_eq!(
            get_int_output(&outputs, "transfer_result", "gas_used"),
            Some(21000)
        );
    }
    
    #[test]
    fn test_templates() {
        let transfer = templates::eth_transfer("alice", "bob", "1000000000000000000");
        assert!(transfer.contains("alice_secret"));
        assert!(transfer.contains("bob_address"));
        assert!(transfer.contains("1000000000000000000"));
        
        let deploy = templates::deploy_contract("SimpleStorage", "alice");
        assert!(deploy.contains("alice_secret"));
        assert!(deploy.contains("src/SimpleStorage.sol"));
    }
}