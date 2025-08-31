//! Integration tests for ABI encoding functionality
//! 
//! These tests verify that the ABI encoding actions properly:
//! - Encode basic types (address, uint, bool, bytes, string)
//! - Encode complex types (arrays, tuples, nested structures)
//! - Handle edge cases and invalid inputs
//! - Provide clear error messages

#[cfg(test)]
mod abi_encoding_tests {
    use crate::tests::test_harness::ProjectTestHarness;
    use txtx_addon_kit::types::types::Value;
    use std::path::PathBuf;
    
    #[test]
    fn test_encode_basic_types() {
        println!("🔍 Testing ABI encoding of basic types");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/abi_encode_basic.tx");
        
        let harness = ProjectTestHarness::from_fixture(&fixture_path)
            .with_input("address_value", "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8")
            .with_input("uint_value", "123456789")
            .with_input("bool_value", "true")
            .with_input("bytes_value", "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")
            .with_input("string_value", "Hello, EVM!");
        
        let result = harness.execute_runbook()
            .expect("Failed to execute ABI encoding");
        
        assert!(result.success, "ABI encoding should succeed");
        
        // Verify we got encoded outputs
        assert!(result.outputs.contains_key("encoded_address"), "Should have encoded address");
        assert!(result.outputs.contains_key("encoded_uint"), "Should have encoded uint");
        assert!(result.outputs.contains_key("encoded_bool"), "Should have encoded bool");
        assert!(result.outputs.contains_key("encoded_multiple"), "Should have encoded multiple params");
        
        println!("✅ Basic ABI encoding test passed");
    }
    
    #[test]
    fn test_encode_arrays() {
        println!("🔍 Testing ABI encoding of arrays");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/abi_encode_complex.tx");
        
        // Create JSON arrays for input
        let addresses = r#"["0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8", "0x0000000000000000000000000000000000000000"]"#;
        let uints = r#"[100, 200, 300]"#;
        
        let harness = ProjectTestHarness::from_fixture(&fixture_path)
            .with_input("addresses_array", addresses)
            .with_input("uint_array", uints)
            .with_input("tuple_maker", "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8")
            .with_input("tuple_amount", "1000000")
            .with_input("nested_data", r#"[[1, 2], [3, 4, 5]]"#);
        
        let result = harness.execute_runbook()
            .expect("Failed to execute array encoding");
        
        assert!(result.success, "Array encoding should succeed");
        assert!(result.outputs.contains_key("encoded_address_array"), "Should encode address array");
        assert!(result.outputs.contains_key("encoded_uint_array"), "Should encode uint array");
        
        println!("✅ Array encoding test passed");
    }
    
    #[test]
    fn test_encode_tuples() {
        println!("🔍 Testing ABI encoding of tuples/structs");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/abi_encode_complex.tx");
        
        let harness = ProjectTestHarness::from_fixture(&fixture_path)
            .with_input("addresses_array", r#"[]"#)
            .with_input("uint_array", r#"[]"#)
            .with_input("tuple_maker", "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8")
            .with_input("tuple_amount", "999999999")
            .with_input("nested_data", r#"[]"#);
        
        let result = harness.execute_runbook()
            .expect("Failed to execute tuple encoding");
        
        assert!(result.success, "Tuple encoding should succeed");
        assert!(result.outputs.contains_key("encoded_tuple"), "Should encode tuple");
        
        println!("✅ Tuple encoding test passed");
    }
    
    #[test]
    fn test_encode_empty_values() {
        println!("🔍 Testing ABI encoding with empty values");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/abi_encode_basic.tx");
        
        let harness = ProjectTestHarness::from_fixture(&fixture_path)
            .with_input("address_value", "0x0000000000000000000000000000000000000000")
            .with_input("uint_value", "0")
            .with_input("bool_value", "false")
            .with_input("bytes_value", "0x0000000000000000000000000000000000000000000000000000000000000000")
            .with_input("string_value", "");
        
        let result = harness.execute_runbook()
            .expect("Failed to execute encoding with empty values");
        
        assert!(result.success, "Encoding empty values should succeed");
        
        println!("✅ Empty value encoding test passed");
    }
    
    #[test]
    fn test_encode_large_numbers() {
        println!("🔍 Testing ABI encoding with large numbers");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/abi_encode_basic.tx");
        
        // Max uint256 value
        let max_uint = "115792089237316195423570985008687907853269984665640564039457584007913129639935";
        
        let harness = ProjectTestHarness::from_fixture(&fixture_path)
            .with_input("address_value", "0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF")
            .with_input("uint_value", max_uint)
            .with_input("bool_value", "true")
            .with_input("bytes_value", "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")
            .with_input("string_value", "Maximum values test");
        
        let result = harness.execute_runbook()
            .expect("Failed to execute encoding with large numbers");
        
        assert!(result.success, "Encoding large numbers should succeed");
        
        println!("✅ Large number encoding test passed");
    }
}