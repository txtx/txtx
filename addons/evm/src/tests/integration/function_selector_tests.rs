//! Integration tests for function selector and call encoding
//! 
//! These tests verify that function selector encoding properly:
//! - Generates correct 4-byte selectors from function signatures
//! - Encodes function calls with parameters
//! - Handles different parameter types
//! - Matches Solidity's keccak256 encoding

#[cfg(test)]
mod function_selector_tests {
    use crate::tests::test_harness::ProjectTestHarness;
    use txtx_addon_kit::types::types::Value;
    use std::path::PathBuf;
    
    #[test]
    fn test_encode_transfer_selector() {
        println!("🔍 Testing function selector for transfer(address,uint256)");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/function_selector_test.tx");
        
        let harness = ProjectTestHarness::from_fixture(&fixture_path)
            .with_input("function_signature", "transfer(address,uint256)")
            .with_input("function_params", r#"["0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8", "1000000"]"#);
        
        let result = harness.execute_runbook()
            .expect("Failed to encode function selector");
        
        assert!(result.success, "Function selector encoding should succeed");
        
        // The selector for transfer(address,uint256) should be 0xa9059cbb
        let selector = result.outputs.get("selector")
            .and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None
            })
            .expect("Should have selector output");
        
        assert!(selector.starts_with("0xa9059cbb"), 
            "transfer selector should be 0xa9059cbb, got {}", selector);
        
        println!("✅ Transfer selector test passed: {}", selector);
    }
    
    #[test]
    fn test_encode_approve_selector() {
        println!("🔍 Testing function selector for approve(address,uint256)");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/function_selector_test.tx");
        
        let harness = ProjectTestHarness::from_fixture(&fixture_path)
            .with_input("function_signature", "approve(address,uint256)")
            .with_input("function_params", r#"["0x0000000000000000000000000000000000000000", "0"]"#);
        
        let result = harness.execute_runbook()
            .expect("Failed to encode function selector");
        
        assert!(result.success, "Function selector encoding should succeed");
        
        // The selector for approve(address,uint256) should be 0x095ea7b3
        let selector = result.outputs.get("selector")
            .and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None
            })
            .expect("Should have selector output");
        
        assert!(selector.starts_with("0x095ea7b3"), 
            "approve selector should be 0x095ea7b3, got {}", selector);
        
        println!("✅ Approve selector test passed: {}", selector);
    }
    
    #[test]
    fn test_encode_balanceof_selector() {
        println!("🔍 Testing function selector for balanceOf(address)");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/function_selector_test.tx");
        
        let harness = ProjectTestHarness::from_fixture(&fixture_path)
            .with_input("function_signature", "balanceOf(address)")
            .with_input("function_params", r#"["0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8"]"#);
        
        let result = harness.execute_runbook()
            .expect("Failed to encode function selector");
        
        assert!(result.success, "Function selector encoding should succeed");
        
        // The selector for balanceOf(address) should be 0x70a08231
        let selector = result.outputs.get("selector")
            .and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None
            })
            .expect("Should have selector output");
        
        assert!(selector.starts_with("0x70a08231"), 
            "balanceOf selector should be 0x70a08231, got {}", selector);
        
        println!("✅ BalanceOf selector test passed: {}", selector);
    }
    
    #[test]
    fn test_encode_complex_function_selector() {
        println!("🔍 Testing function selector for complex signature");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/function_selector_test.tx");
        
        // Complex function with multiple parameter types
        let signature = "swapExactTokensForTokens(uint256,uint256,address[],address,uint256)";
        
        let harness = ProjectTestHarness::from_fixture(&fixture_path)
            .with_input("function_signature", signature)
            .with_input("function_params", r#"[
                "1000000",
                "900000",
                ["0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8", "0x0000000000000000000000000000000000000000"],
                "0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
                "1234567890"
            ]"#);
        
        let result = harness.execute_runbook()
            .expect("Failed to encode complex function selector");
        
        assert!(result.success, "Complex function selector encoding should succeed");
        
        let selector = result.outputs.get("selector")
            .and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None
            })
            .expect("Should have selector output");
        
        // Selector should be 4 bytes (8 hex chars + 0x prefix)
        assert_eq!(selector.len(), 10, "Selector should be 10 characters (0x + 8 hex)");
        
        println!("✅ Complex selector test passed: {}", selector);
    }
    
    #[test]
    fn test_encode_function_with_no_params() {
        println!("🔍 Testing function selector for parameterless function");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/function_selector_test.tx");
        
        let harness = ProjectTestHarness::from_fixture(&fixture_path)
            .with_input("function_signature", "totalSupply()")
            .with_input("function_params", r#"[]"#);
        
        let result = harness.execute_runbook()
            .expect("Failed to encode parameterless function");
        
        assert!(result.success, "Parameterless function encoding should succeed");
        
        // The selector for totalSupply() should be 0x18160ddd
        let selector = result.outputs.get("selector")
            .and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None
            })
            .expect("Should have selector output");
        
        assert!(selector.starts_with("0x18160ddd"), 
            "totalSupply selector should be 0x18160ddd, got {}", selector);
        
        println!("✅ Parameterless function test passed: {}", selector);
    }
}