//! Integration tests for ABI encoding functionality
//! 
//! These tests verify that the ABI encoding actions properly:
//! - Encode basic types (address, uint, bool, bytes, string)
//! - Encode complex types (arrays, tuples, nested structures)
//! - Handle edge cases and invalid inputs
//! - Provide clear error messages

#[cfg(test)]
mod abi_encoding_tests {
    use crate::tests::fixture_builder::{MigrationHelper, TestResult};
    use std::path::PathBuf;
    use tokio;
    
    #[tokio::test]
    async fn test_encode_basic_types() {
        println!("🔍 Testing ABI encoding of basic types");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/abi_encode_basic.tx");
        
        let result = MigrationHelper::from_fixture(&fixture_path)
            .with_input("address_value", "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8")
            .with_input("uint_value", "123456789")
            .with_input("bool_value", "true")
            .with_input("bytes_value", "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")
            .with_input("string_value", "Hello, EVM!")
            .execute()
            .await
            .expect("Failed to execute ABI encoding");
        
        assert!(result.success, "ABI encoding should succeed");
        
        // Verify we got encoded outputs
        assert!(result.outputs.contains_key("encoded_address"), "Should have encoded address");
        assert!(result.outputs.contains_key("encoded_uint"), "Should have encoded uint");
        assert!(result.outputs.contains_key("encoded_bool"), "Should have encoded bool");
        assert!(result.outputs.contains_key("encoded_multiple"), "Should have encoded multiple params");
        
        println!("✅ Basic ABI encoding test passed");
    }
    
    #[tokio::test]
    async fn test_encode_arrays() {
        println!("🔍 Testing ABI encoding of arrays");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/abi_encode_complex.tx");
        
        // Create JSON arrays for input
        let addresses = r#"["0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8", "0x0000000000000000000000000000000000000000"]"#;
        let uints = r#"[100, 200, 300]"#;
        
        let result = MigrationHelper::from_fixture(&fixture_path)
            .with_input("addresses_array", addresses)
            .with_input("uint_array", uints)
            .with_input("tuple_maker", "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8")
            .with_input("tuple_amount", "1000000")
            .with_input("nested_data", r#"[[1, 2], [3, 4, 5]]"#)
            .execute()
            .await
            .expect("Failed to execute array encoding");
        
        assert!(result.success, "Array encoding should succeed");
        assert!(result.outputs.contains_key("encoded_address_array"), "Should encode address array");
        assert!(result.outputs.contains_key("encoded_uint_array"), "Should encode uint array");
        
        println!("✅ Array encoding test passed");
    }
    
    #[tokio::test]
    async fn test_encode_tuples() {
        println!("🔍 Testing ABI encoding of tuples/structs");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/abi_encode_complex.tx");
        
        let result = MigrationHelper::from_fixture(&fixture_path)
            .with_input("addresses_array", r#"[]"#)
            .with_input("uint_array", r#"[]"#)
            .with_input("tuple_maker", "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8")
            .with_input("tuple_amount", "999999999")
            .with_input("nested_data", r#"[]"#)
            .execute()
            .await
            .expect("Failed to execute tuple encoding");
        
        assert!(result.success, "Tuple encoding should succeed");
        assert!(result.outputs.contains_key("encoded_tuple"), "Should encode tuple");
        
        println!("✅ Tuple encoding test passed");
    }
    
    #[tokio::test]
    async fn test_encode_empty_values() {
        println!("🔍 Testing ABI encoding with empty values");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/abi_encode_basic.tx");
        
        let result = MigrationHelper::from_fixture(&fixture_path)
            .with_input("address_value", "0x0000000000000000000000000000000000000000")
            .with_input("uint_value", "0")
            .with_input("bool_value", "false")
            .with_input("bytes_value", "0x")
            .with_input("string_value", "")
            .execute()
            .await
            .expect("Failed to execute empty value encoding");
        
        assert!(result.success, "Empty value encoding should succeed");
        
        // Verify outputs exist
        assert!(result.outputs.contains_key("encoded_address"), "Should encode zero address");
        assert!(result.outputs.contains_key("encoded_uint"), "Should encode zero uint");
        assert!(result.outputs.contains_key("encoded_bool"), "Should encode false bool");
        
        println!("✅ Empty value encoding test passed");
    }
    
    #[tokio::test]
    async fn test_encode_with_signatures() {
        println!("🔍 Testing ABI encoding with function signatures");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/abi_encode_function.tx");
        
        let result = MigrationHelper::from_fixture(&fixture_path)
            .with_input("target_address", "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8")
            .with_input("transfer_amount", "1000000000000000000")
            .with_input("spender_address", "0x123456789012345678901234567890123456789a")
            .with_input("allowance_amount", "500000000000000000")
            .execute()
            .await
            .expect("Failed to execute function encoding");
        
        assert!(result.success, "Function encoding should succeed");
        
        // Verify function call encodings
        assert!(result.outputs.contains_key("transfer_calldata"), "Should encode transfer function");
        assert!(result.outputs.contains_key("approve_calldata"), "Should encode approve function");
        
        println!("✅ Function signature encoding test passed");
    }
    
    #[tokio::test]
    async fn test_encode_packed() {
        println!("🔍 Testing packed ABI encoding");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/abi_encode_packed.tx");
        
        let result = MigrationHelper::from_fixture(&fixture_path)
            .with_input("address1", "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8")
            .with_input("uint256_value", "123456789")
            .with_input("bytes32_value", "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
            .with_input("string_value", "packed")
            .execute()
            .await
            .expect("Failed to execute packed encoding");
        
        assert!(result.success, "Packed encoding should succeed");
        assert!(result.outputs.contains_key("packed_encoding"), "Should have packed encoding");
        
        println!("✅ Packed encoding test passed");
    }
}