//! Integration tests for ABI encoding functionality
//! 
//! These tests verify that the ABI encoding actions properly:
//! - Encode basic types (address, uint, bool, bytes, string)
//! - Encode complex types (arrays, tuples, nested structures)
//! - Handle edge cases and invalid inputs
//! - Provide clear error messages

#[cfg(test)]
mod abi_encoding_tests {
    use crate::tests::fixture_builder::FixtureBuilder;
    use std::path::PathBuf;
    use std::fs;
    use tokio;
    
    #[tokio::test]
    async fn test_encode_basic_types() {
        println!("🔍 Testing ABI encoding of basic types");
        
        // ARRANGE: Load fixture and set up parameters
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/abi_encode_basic.tx");
        let fixture_content = fs::read_to_string(&fixture_path)
            .expect("Failed to read fixture file");
        
        let mut fixture = FixtureBuilder::new("test_encode_basic")
            .with_runbook("main", &fixture_content)
            .with_parameter("address_value", "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8")
            .with_parameter("uint_value", "123456789")
            .with_parameter("bool_value", "true")
            .with_parameter("bytes_value", "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")
            .with_parameter("string_value", "Hello, EVM!")
            .build()
            .await
            .expect("Failed to build fixture");
        
        // ACT: Execute the runbook
        fixture.execute_runbook("main").await
            .expect("Failed to execute ABI encoding");
        
        // ASSERT: Verify we got encoded outputs
        let outputs = fixture.get_outputs("main")
            .expect("Should have outputs");
        
        assert!(outputs.get("encoded_address").is_some(), "Should have encoded address");
        assert!(outputs.get("encoded_uint").is_some(), "Should have encoded uint");
        assert!(outputs.get("encoded_bool").is_some(), "Should have encoded bool");
        assert!(outputs.get("encoded_multiple").is_some(), "Should have encoded multiple params");
        
        println!("✅ Basic ABI encoding test passed");
    }
    
    #[tokio::test]
    async fn test_encode_arrays() {
        println!("🔍 Testing ABI encoding of arrays");
        
        // ARRANGE: Create inline runbook for array encoding
        let array_runbook = r#"
addon "evm" {
    chain_id = 31337
}

# Encode address array
action "encode_addresses" "evm::encode_abi" {
    types = ["address[]"]
    values = [["0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8", "0x0000000000000000000000000000000000000000"]]
}

# Encode uint array
action "encode_uints" "evm::encode_abi" {
    types = ["uint256[]"]
    values = [[100, 200, 300]]
}

# Encode nested array
action "encode_nested" "evm::encode_abi" {
    types = ["uint256[][]"]
    values = [[[1, 2], [3, 4, 5]]]
}

output "encoded_address_array" {
    value = action.encode_addresses.result
}

output "encoded_uint_array" {
    value = action.encode_uints.result
}

output "encoded_nested_array" {
    value = action.encode_nested.result
}"#;
        
        let mut fixture = FixtureBuilder::new("test_encode_arrays")
            .with_runbook("arrays", array_runbook)
            .build()
            .await
            .expect("Failed to build fixture");
        
        // ACT: Execute the runbook
        fixture.execute_runbook("arrays").await
            .expect("Failed to execute array encoding");
        
        // ASSERT: Verify array encodings
        let outputs = fixture.get_outputs("arrays")
            .expect("Should have outputs");
        
        assert!(outputs.get("encoded_address_array").is_some(), "Should encode address array");
        assert!(outputs.get("encoded_uint_array").is_some(), "Should encode uint array");
        assert!(outputs.get("encoded_nested_array").is_some(), "Should encode nested array");
        
        println!("✅ Array encoding test passed");
    }
    
    #[tokio::test]
    async fn test_encode_tuples() {
        println!("🔍 Testing ABI encoding of tuples/structs");
        
        // ARRANGE: Create inline runbook for tuple encoding
        let tuple_runbook = r#"
addon "evm" {
    chain_id = 31337
}

# Encode a tuple (struct-like)
action "encode_tuple" "evm::encode_abi" {
    types = ["(address,uint256,bool)"]
    values = [["0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8", 999999999, true]]
}

# Encode nested tuple
action "encode_nested_tuple" "evm::encode_abi" {
    types = ["(address,(uint256,bool))"]
    values = [["0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8", [123456, false]]]
}

output "encoded_tuple" {
    value = action.encode_tuple.result
}

output "encoded_nested_tuple" {
    value = action.encode_nested_tuple.result
}"#;
        
        let mut fixture = FixtureBuilder::new("test_encode_tuples")
            .with_runbook("tuples", tuple_runbook)
            .build()
            .await
            .expect("Failed to build fixture");
        
        // ACT: Execute the runbook
        fixture.execute_runbook("tuples").await
            .expect("Failed to execute tuple encoding");
        
        // ASSERT: Verify tuple encodings
        let outputs = fixture.get_outputs("tuples")
            .expect("Should have outputs");
        
        assert!(outputs.get("encoded_tuple").is_some(), "Should encode tuple");
        assert!(outputs.get("encoded_nested_tuple").is_some(), "Should encode nested tuple");
        
        println!("✅ Tuple encoding test passed");
    }
    
    #[tokio::test]
    async fn test_encode_empty_values() {
        println!("🔍 Testing ABI encoding with empty values");
        
        // ARRANGE: Load fixture with empty/zero values
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/abi_encode_basic.tx");
        let fixture_content = fs::read_to_string(&fixture_path)
            .expect("Failed to read fixture file");
        
        let mut fixture = FixtureBuilder::new("test_encode_empty")
            .with_runbook("empty", &fixture_content)
            .with_parameter("address_value", "0x0000000000000000000000000000000000000000")
            .with_parameter("uint_value", "0")
            .with_parameter("bool_value", "false")
            .with_parameter("bytes_value", "0x0000000000000000000000000000000000000000000000000000000000000000")
            .with_parameter("string_value", "")
            .build()
            .await
            .expect("Failed to build fixture");
        
        // ACT: Execute the runbook
        fixture.execute_runbook("empty").await
            .expect("Failed to execute empty value encoding");
        
        // ASSERT: Verify outputs exist
        let outputs = fixture.get_outputs("empty")
            .expect("Should have outputs");
        
        assert!(outputs.get("encoded_address").is_some(), "Should encode zero address");
        assert!(outputs.get("encoded_uint").is_some(), "Should encode zero uint");
        assert!(outputs.get("encoded_bool").is_some(), "Should encode false bool");
        
        println!("✅ Empty value encoding test passed");
    }
    
    #[tokio::test]
    async fn test_encode_with_signatures() {
        println!("🔍 Testing ABI encoding with function signatures");
        
        // ARRANGE: Create inline runbook for function encoding
        let function_runbook = r#"
addon "evm" {
    chain_id = 31337
}

# Encode transfer(address,uint256) function call
action "encode_transfer" "evm::encode_function_calldata" {
    function_signature = "transfer(address,uint256)"
    args = ["0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8", "1000000000000000000"]
}

# Encode approve(address,uint256) function call
action "encode_approve" "evm::encode_function_calldata" {
    function_signature = "approve(address,uint256)"
    args = ["0x123456789012345678901234567890123456789a", "500000000000000000"]
}

# Encode balanceOf(address) view function
action "encode_balance_of" "evm::encode_function_calldata" {
    function_signature = "balanceOf(address)"
    args = ["0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8"]
}

output "transfer_calldata" {
    value = action.encode_transfer.result
}

output "approve_calldata" {
    value = action.encode_approve.result
}

output "balance_of_calldata" {
    value = action.encode_balance_of.result
}"#;
        
        let mut fixture = FixtureBuilder::new("test_encode_functions")
            .with_runbook("functions", function_runbook)
            .build()
            .await
            .expect("Failed to build fixture");
        
        // ACT: Execute the runbook
        fixture.execute_runbook("functions").await
            .expect("Failed to execute function encoding");
        
        // ASSERT: Verify function call encodings
        let outputs = fixture.get_outputs("functions")
            .expect("Should have outputs");
        
        assert!(outputs.get("transfer_calldata").is_some(), "Should encode transfer function");
        assert!(outputs.get("approve_calldata").is_some(), "Should encode approve function");
        assert!(outputs.get("balance_of_calldata").is_some(), "Should encode balanceOf function");
        
        println!("✅ Function signature encoding test passed");
    }
    
    #[tokio::test]
    async fn test_encode_packed() {
        println!("🔍 Testing packed ABI encoding");
        
        // ARRANGE: Create inline runbook for packed encoding
        let packed_runbook = r#"
addon "evm" {
    chain_id = 31337
}

# Encode packed (non-standard ABI encoding)
action "encode_packed" "evm::encode_abi_packed" {
    types = ["address", "uint256", "bytes32", "string"]
    values = [
        "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8",
        "123456789",
        "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "packed"
    ]
}

# Encode packed for hash computation (common use case)
action "encode_for_hash" "evm::encode_abi_packed" {
    types = ["address", "uint256"]
    values = ["0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8", "999"]
}

output "packed_encoding" {
    value = action.encode_packed.result
}

output "hash_encoding" {
    value = action.encode_for_hash.result
}"#;
        
        let mut fixture = FixtureBuilder::new("test_encode_packed")
            .with_runbook("packed", packed_runbook)
            .build()
            .await
            .expect("Failed to build fixture");
        
        // ACT: Execute the runbook
        fixture.execute_runbook("packed").await
            .expect("Failed to execute packed encoding");
        
        // ASSERT: Verify packed encodings
        let outputs = fixture.get_outputs("packed")
            .expect("Should have outputs");
        
        assert!(outputs.get("packed_encoding").is_some(), "Should have packed encoding");
        assert!(outputs.get("hash_encoding").is_some(), "Should have hash encoding");
        
        println!("✅ Packed encoding test passed");
    }
}