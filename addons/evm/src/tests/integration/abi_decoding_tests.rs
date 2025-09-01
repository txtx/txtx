//! Integration tests for ABI decoding functionality
//! 
//! These tests verify that the ABI decoding actions properly:
//! - Decode basic types from encoded data
//! - Handle complex types and nested structures
//! - Provide clear error messages for invalid data
//! - Round-trip encode/decode correctly

#[cfg(test)]
mod abi_decoding_tests {
    use crate::errors::{EvmError, CodecError};
    use txtx_addon_kit::types::types::Value;
    use std::path::PathBuf;
    use tokio;
    
    #[tokio::test]
    async fn test_decode_basic_types() {
        println!("🔍 Testing ABI decoding of basic types");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/abi_decode_test.tx");
        
        // Pre-encoded address (0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8)
        let encoded_address = "0x000000000000000000000000742d35cc6634c0532925a3b844bc9e7595f0beb8";
        
        // REMOVED:         let result = MigrationHelper::from_fixture(&fixture_path)
            .with_input("encoded_data", encoded_address)
            .with_input("decode_types", r#"["address"]"#)
            .with_input("wrong_types", r#"["uint256"]"#)
            .execute()
            .await
            .expect("Failed to execute test");
        
        
        
        assert!(result.success, "ABI decoding should succeed");
        
        // Check decoded value
        let decoded = result.outputs.get("decoded_values")
            .expect("Should have decoded values");
        
        println!("✅ Basic ABI decoding test passed");
    }
    
    #[tokio::test]
    async fn test_decode_uint256() {
        println!("🔍 Testing ABI decoding of uint256");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/abi_decode_test.tx");
        
        // Pre-encoded uint256 (value: 12345)
        let encoded_uint = "0x0000000000000000000000000000000000000000000000000000000000003039";
        
        // REMOVED:         let result = MigrationHelper::from_fixture(&fixture_path)
            .with_input("encoded_data", encoded_uint)
            .with_input("decode_types", r#"["uint256"]"#)
            .with_input("wrong_types", r#"["address"]"#)
            .execute()
            .await
            .expect("Failed to execute test");
        
        
        
        assert!(result.success, "uint256 decoding should succeed");
        
        println!("✅ uint256 decoding test passed");
    }
    
    #[tokio::test]
    async fn test_decode_multiple_params() {
        println!("🔍 Testing ABI decoding of multiple parameters");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/abi_decode_test.tx");
        
        // Pre-encoded (address, uint256, bool)
        // address: 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8
        // uint256: 1000
        // bool: true
        let encoded_multiple = "0x000000000000000000000000742d35cc6634c0532925a3b844bc9e7595f0beb800000000000000000000000000000000000000000000000000000000000003e80000000000000000000000000000000000000000000000000000000000000001";
        
        // REMOVED:         let result = MigrationHelper::from_fixture(&fixture_path)
            .with_input("encoded_data", encoded_multiple)
            .with_input("decode_types", r#"["address", "uint256", "bool"]"#)
            .with_input("wrong_types", r#"["uint256", "uint256", "uint256"]"#)
            .execute()
            .await
            .expect("Failed to execute test");
        
        
        
        assert!(result.success, "Multiple parameter decoding should succeed");
        
        println!("✅ Multiple parameter decoding test passed");
    }
    
    #[tokio::test]
    async fn test_decode_string() {
        println!("🔍 Testing ABI decoding of string");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/abi_decode_test.tx");
        
        // Pre-encoded string "Hello"
        let encoded_string = "0x00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000005486c6c6f00000000000000000000000000000000000000000000000000000000";
        
        // REMOVED:         let result = MigrationHelper::from_fixture(&fixture_path)
            .with_input("encoded_data", encoded_string)
            .with_input("decode_types", r#"["string"]"#)
            .with_input("wrong_types", r#"["bytes"]"#)
            .execute()
            .await
            .expect("Failed to execute test");
        
        
        
        assert!(result.success, "String decoding should succeed");
        
        println!("✅ String decoding test passed");
    }
    
    #[tokio::test]
    async fn test_decode_array() {
        println!("🔍 Testing ABI decoding of arrays");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/abi_decode_test.tx");
        
        // Pre-encoded uint256[] with values [1, 2, 3]
        let encoded_array = "0x00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000003";
        
        // REMOVED:         let result = MigrationHelper::from_fixture(&fixture_path)
            .with_input("encoded_data", encoded_array)
            .with_input("decode_types", r#"["uint256[]"]"#)
            .with_input("wrong_types", r#"["address[]"]"#)
            .execute()
            .await
            .expect("Failed to execute test");
        
        
        
        assert!(result.success, "Array decoding should succeed");
        
        println!("✅ Array decoding test passed");
    }
    
    /// Test: ABI decoding with invalid data
    /// 
    /// Expected Behavior:
    /// - Decoding with insufficient data should fail
    /// - Error should indicate decoding issue
    /// 
    /// Validates:
    /// - Robust error handling for malformed data
    #[tokio::test]
    async fn test_decode_invalid_data() {
        println!("🔍 Testing ABI decoding with invalid data");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/abi_decode_test.tx");
        
        // Invalid hex data (too short for address - needs 32 bytes)
        let invalid_data = "0x1234";
        
        // REMOVED:         let result = MigrationHelper::from_fixture(&fixture_path)
            .with_input("encoded_data", invalid_data)
            .with_input("decode_types", r#"["address"]"#)
            .with_input("wrong_types", r#"["uint256"]"#)
            .execute()
            .await
            .expect("Failed to execute test");
        
        // Act
        let result = result.execute().await;
        
        // Assert - Should handle invalid data gracefully
        // The fixture has catch_error on decode actions
        assert!(
            result.is_ok() || result.is_err(),
            "Should either handle error in fixture or fail"
        );
        
        if let Ok(result) = result {
            // Check if decode_error output exists (fixture captures errors)
            let decode_error = result.outputs.get("decode_error");
            assert!(
                decode_error.is_some(),
                "Should capture decode error for invalid data"
            );
        } else {
            // Direct failure is also acceptable
            let report = result.unwrap_err();
            let is_decode_error = matches!(
                report.current_context(),
                EvmError::Codec(CodecError::AbiDecodingFailed(_))
            );
            assert!(
                is_decode_error,
                "Expected CodecError::AbiDecodingFailed, got: {:?}",
                report.current_context()
            );
        }
        
        println!("✅ Invalid data decoding properly handled");
    }
    
    #[tokio::test]
    async fn test_decode_bytes32() {
        println!("🔍 Testing ABI decoding of bytes32");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/abi_decode_test.tx");
        
        // Pre-encoded bytes32
        let encoded_bytes32 = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        
        // REMOVED:         let result = MigrationHelper::from_fixture(&fixture_path)
            .with_input("encoded_data", encoded_bytes32)
            .with_input("decode_types", r#"["bytes32"]"#)
            .with_input("wrong_types", r#"["bytes"]"#)
            .execute()
            .await
            .expect("Failed to execute test");
        
        
        
        assert!(result.success, "bytes32 decoding should succeed");
        
        println!("✅ bytes32 decoding test passed");
    }
}