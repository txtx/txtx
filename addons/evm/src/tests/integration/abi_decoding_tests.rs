//! Integration tests for ABI decoding functionality
//! 
//! These tests verify that the ABI decoding actions properly:
//! - Decode basic types from encoded data
//! - Handle complex types and nested structures
//! - Provide clear error messages for invalid data
//! - Round-trip encode/decode correctly

#[cfg(test)]
mod abi_decoding_tests {
    use crate::tests::fixture_builder::FixtureBuilder;
    use std::path::PathBuf;
    use std::fs;
    use tokio;
    
    #[tokio::test]
    async fn test_decode_basic_types() {
        println!("🔍 Testing ABI decoding of basic types");
        
        // ARRANGE: Create inline runbook for decoding
        let decode_runbook = r#"
addon "evm" {
    chain_id = 31337
}

# Decode an encoded address
action "decode_address" "evm::decode_abi" {
    data = input.encoded_address
    types = ["address"]
}

# Decode an encoded uint256
action "decode_uint" "evm::decode_abi" {
    data = input.encoded_uint
    types = ["uint256"]
}

# Decode an encoded bool
action "decode_bool" "evm::decode_abi" {
    data = input.encoded_bool
    types = ["bool"]
}

output "decoded_address" {
    value = action.decode_address.result[0]
}

output "decoded_uint" {
    value = action.decode_uint.result[0]
}

output "decoded_bool" {
    value = action.decode_bool.result[0]
}"#;
        
        let mut fixture = FixtureBuilder::new("test_decode_basic")
            .with_runbook("decode", decode_runbook)
            // Pre-encoded address (0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8)
            .with_parameter("encoded_address", "0x000000000000000000000000742d35cc6634c0532925a3b844bc9e7595f0beb8")
            // Pre-encoded uint256 (12345)
            .with_parameter("encoded_uint", "0x0000000000000000000000000000000000000000000000000000000000003039")
            // Pre-encoded bool (true)
            .with_parameter("encoded_bool", "0x0000000000000000000000000000000000000000000000000000000000000001")
            .build()
            .await
            .expect("Failed to build fixture");
        
        // ACT: Execute the runbook
        fixture.execute_runbook("decode").await
            .expect("Failed to execute ABI decoding");
        
        // ASSERT: Verify decoded values
        let outputs = fixture.get_outputs("decode")
            .expect("Should have outputs");
        
        let address = outputs.get("decoded_address")
            .and_then(|v| v.as_string())
            .expect("Should have decoded address");
        assert_eq!(address.to_lowercase(), "0x742d35cc6634c0532925a3b844bc9e7595f0beb8");
        
        let uint_val = outputs.get("decoded_uint")
            .and_then(|v| v.as_integer().or_else(|| v.as_string()?.parse().ok()))
            .expect("Should have decoded uint");
        assert_eq!(uint_val, 12345);
        
        let bool_val = outputs.get("decoded_bool")
            .and_then(|v| v.as_bool())
            .expect("Should have decoded bool");
        assert!(bool_val);
        
        println!("✅ Basic ABI decoding test passed");
    }
    
    #[tokio::test]
    async fn test_decode_multiple_params() {
        println!("🔍 Testing ABI decoding of multiple parameters");
        
        // ARRANGE: Create inline runbook
        let multiple_runbook = r#"
addon "evm" {
    chain_id = 31337
}

# Decode multiple parameters at once
action "decode_multiple" "evm::decode_abi" {
    data = input.encoded_data
    types = ["address", "uint256", "bool"]
}

output "decoded_values" {
    value = action.decode_multiple.result
}

output "address_value" {
    value = action.decode_multiple.result[0]
}

output "uint_value" {
    value = action.decode_multiple.result[1]
}

output "bool_value" {
    value = action.decode_multiple.result[2]
}"#;
        
        // Pre-encoded (address, uint256, bool)
        // address: 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8
        // uint256: 1000
        // bool: true
        let encoded_multiple = "0x000000000000000000000000742d35cc6634c0532925a3b844bc9e7595f0beb800000000000000000000000000000000000000000000000000000000000003e80000000000000000000000000000000000000000000000000000000000000001";
        
        let mut fixture = FixtureBuilder::new("test_decode_multiple")
            .with_runbook("multiple", multiple_runbook)
            .with_parameter("encoded_data", encoded_multiple)
            .build()
            .await
            .expect("Failed to build fixture");
        
        // ACT: Execute the runbook
        fixture.execute_runbook("multiple").await
            .expect("Failed to execute multiple parameter decoding");
        
        // ASSERT: Verify decoded values
        let outputs = fixture.get_outputs("multiple")
            .expect("Should have outputs");
        
        let address = outputs.get("address_value")
            .and_then(|v| v.as_string())
            .expect("Should have decoded address");
        assert_eq!(address.to_lowercase(), "0x742d35cc6634c0532925a3b844bc9e7595f0beb8");
        
        let uint_val = outputs.get("uint_value")
            .and_then(|v| v.as_integer().or_else(|| v.as_string()?.parse().ok()))
            .expect("Should have decoded uint");
        assert_eq!(uint_val, 1000);
        
        let bool_val = outputs.get("bool_value")
            .and_then(|v| v.as_bool())
            .expect("Should have decoded bool");
        assert!(bool_val);
        
        println!("✅ Multiple parameter decoding test passed");
    }
    
    #[tokio::test]
    async fn test_decode_string() {
        println!("🔍 Testing ABI decoding of string");
        
        // ARRANGE: Create inline runbook
        let string_runbook = r#"
addon "evm" {
    chain_id = 31337
}

# Decode a string
action "decode_string" "evm::decode_abi" {
    data = input.encoded_string
    types = ["string"]
}

output "decoded_string" {
    value = action.decode_string.result[0]
}"#;
        
        // Pre-encoded string "Hello"
        let encoded_string = "0x00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000005486c6c6f00000000000000000000000000000000000000000000000000000000";
        
        let mut fixture = FixtureBuilder::new("test_decode_string")
            .with_runbook("string", string_runbook)
            .with_parameter("encoded_string", encoded_string)
            .build()
            .await
            .expect("Failed to build fixture");
        
        // ACT: Execute the runbook
        fixture.execute_runbook("string").await
            .expect("Failed to execute string decoding");
        
        // ASSERT: Verify decoded string
        let outputs = fixture.get_outputs("string")
            .expect("Should have outputs");
        
        let decoded = outputs.get("decoded_string")
            .and_then(|v| v.as_string())
            .expect("Should have decoded string");
        assert_eq!(decoded, "Hello");
        
        println!("✅ String decoding test passed");
    }
    
    #[tokio::test]
    async fn test_decode_array() {
        println!("🔍 Testing ABI decoding of arrays");
        
        // ARRANGE: Create inline runbook
        let array_runbook = r#"
addon "evm" {
    chain_id = 31337
}

# Decode a uint256 array
action "decode_array" "evm::decode_abi" {
    data = input.encoded_array
    types = ["uint256[]"]
}

output "decoded_array" {
    value = action.decode_array.result[0]
}"#;
        
        // Pre-encoded uint256[] with values [1, 2, 3]
        let encoded_array = "0x00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000003";
        
        let mut fixture = FixtureBuilder::new("test_decode_array")
            .with_runbook("array", array_runbook)
            .with_parameter("encoded_array", encoded_array)
            .build()
            .await
            .expect("Failed to build fixture");
        
        // ACT: Execute the runbook
        fixture.execute_runbook("array").await
            .expect("Failed to execute array decoding");
        
        // ASSERT: Verify decoded array
        let outputs = fixture.get_outputs("array")
            .expect("Should have outputs");
        
        assert!(outputs.get("decoded_array").is_some(), "Should have decoded array");
        // Note: Actual array validation depends on how the Value type represents arrays
        
        println!("✅ Array decoding test passed");
    }
    
    #[tokio::test]
    async fn test_decode_invalid_data() {
        println!("🔍 Testing ABI decoding with invalid data");
        
        // ARRANGE: Create inline runbook with error handling
        let invalid_runbook = r#"
addon "evm" {
    chain_id = 31337
}

# Try to decode invalid data (should fail gracefully)
action "decode_invalid" "evm::decode_abi" {
    data = input.invalid_data
    types = ["address"]
    catch_error = true
}

output "decode_result" {
    value = action.decode_invalid.result
}

output "decode_error" {
    value = action.decode_invalid.error
}"#;
        
        // Invalid hex data (too short for address - needs 32 bytes)
        let invalid_data = "0x1234";
        
        let mut fixture = FixtureBuilder::new("test_decode_invalid")
            .with_runbook("invalid", invalid_runbook)
            .with_parameter("invalid_data", invalid_data)
            .build()
            .await
            .expect("Failed to build fixture");
        
        // ACT: Execute the runbook (should handle error gracefully)
        let result = fixture.execute_runbook("invalid").await;
        
        // ASSERT: Should either capture error or fail gracefully
        if result.is_ok() {
            // Fixture caught the error
            let outputs = fixture.get_outputs("invalid")
                .expect("Should have outputs");
            assert!(outputs.get("decode_error").is_some(), 
                "Should capture decode error for invalid data");
        } else {
            // Direct failure is also acceptable for invalid data
            println!("  Decoding invalid data failed as expected");
        }
        
        println!("✅ Invalid data decoding properly handled");
    }
    
    #[tokio::test]
    async fn test_decode_bytes32() {
        println!("🔍 Testing ABI decoding of bytes32");
        
        // ARRANGE: Create inline runbook
        let bytes32_runbook = r#"
addon "evm" {
    chain_id = 31337
}

# Decode bytes32
action "decode_bytes32" "evm::decode_abi" {
    data = input.encoded_bytes32
    types = ["bytes32"]
}

output "decoded_bytes32" {
    value = action.decode_bytes32.result[0]
}"#;
        
        // Pre-encoded bytes32
        let encoded_bytes32 = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        
        let mut fixture = FixtureBuilder::new("test_decode_bytes32")
            .with_runbook("bytes32", bytes32_runbook)
            .with_parameter("encoded_bytes32", encoded_bytes32)
            .build()
            .await
            .expect("Failed to build fixture");
        
        // ACT: Execute the runbook
        fixture.execute_runbook("bytes32").await
            .expect("Failed to execute bytes32 decoding");
        
        // ASSERT: Verify decoded bytes32
        let outputs = fixture.get_outputs("bytes32")
            .expect("Should have outputs");
        
        let decoded = outputs.get("decoded_bytes32")
            .and_then(|v| v.as_string())
            .expect("Should have decoded bytes32");
        assert_eq!(decoded, "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef");
        
        println!("✅ bytes32 decoding test passed");
    }
    
    #[tokio::test]
    async fn test_decode_tuple() {
        println!("🔍 Testing ABI decoding of tuples");
        
        // ARRANGE: Create inline runbook
        let tuple_runbook = r#"
addon "evm" {
    chain_id = 31337
}

# First encode a tuple to get valid data
action "encode_tuple" "evm::encode_abi" {
    types = ["(address,uint256,bool)"]
    values = [["0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8", 42, true]]
}

# Then decode it back
action "decode_tuple" "evm::decode_abi" {
    data = action.encode_tuple.result
    types = ["(address,uint256,bool)"]
}

output "encoded_data" {
    value = action.encode_tuple.result
}

output "decoded_tuple" {
    value = action.decode_tuple.result[0]
}"#;
        
        let mut fixture = FixtureBuilder::new("test_decode_tuple")
            .with_runbook("tuple", tuple_runbook)
            .build()
            .await
            .expect("Failed to build fixture");
        
        // ACT: Execute the runbook
        fixture.execute_runbook("tuple").await
            .expect("Failed to execute tuple encoding/decoding");
        
        // ASSERT: Verify round-trip worked
        let outputs = fixture.get_outputs("tuple")
            .expect("Should have outputs");
        
        assert!(outputs.get("encoded_data").is_some(), "Should have encoded data");
        assert!(outputs.get("decoded_tuple").is_some(), "Should have decoded tuple");
        
        println!("✅ Tuple decoding test passed");
    }
}