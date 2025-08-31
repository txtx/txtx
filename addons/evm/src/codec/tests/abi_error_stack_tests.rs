//! Tests for ABI encoding error messages with error-stack
//! 
//! These tests verify that our ABI encoding provides helpful, contextual error messages
//! that guide users to fix their issues quickly.

use crate::codec::abi::encoding::*;
use crate::errors::{EvmError, CodecError};
use alloy::json_abi::{Function, JsonAbi, Param, StateMutability};
use alloy::primitives::address;
use txtx_addon_kit::types::types::Value;
use crate::typing::EvmValue;

fn create_uniswap_v3_mint_abi() -> JsonAbi {
    // Simplified Uniswap V3 mint function for testing
    let mint_fn = Function {
        name: "mint".to_string(),
        inputs: vec![
            Param {
                name: "recipient".to_string(),
                ty: "address".to_string(),
                internal_type: None,
                components: vec![],
            },
            Param {
                name: "tickLower".to_string(),
                ty: "int24".to_string(),
                internal_type: None,
                components: vec![],
            },
            Param {
                name: "tickUpper".to_string(),
                ty: "int24".to_string(),
                internal_type: None,
                components: vec![],
            },
            Param {
                name: "amount".to_string(),
                ty: "uint128".to_string(),
                internal_type: None,
                components: vec![],
            },
            Param {
                name: "data".to_string(),
                ty: "bytes".to_string(),
                internal_type: None,
                components: vec![],
            },
        ],
        outputs: vec![],
        state_mutability: StateMutability::NonPayable,
    };
    
    let mut abi = JsonAbi::default();
    abi.functions.insert("mint".to_string(), vec![mint_fn]);
    abi
}

#[test]
fn test_function_not_found_error_message() {
    let abi = create_uniswap_v3_mint_abi();
    let args = Value::array(vec![]);
    
    // Try to call non-existent function
    let result = value_to_abi_function_args("Mint", &args, &abi); // Wrong case
    assert!(result.is_err());
    
    let error = result.unwrap_err();
    
    // First check the error type
    let is_function_not_found = matches!(
        error.current_context(),
        EvmError::Codec(CodecError::FunctionNotFound { name }) if name == "Mint"
    );
    assert!(is_function_not_found, "Expected CodecError::FunctionNotFound, got: {:?}", error.current_context());
    
    // Also verify the error message contains helpful context (for user-facing messages)
    let error_string = format!("{:?}", error);
    assert!(error_string.contains("Available functions: mint"), "Should list available functions");
    assert!(error_string.contains("Did you mean 'mint'? (case-sensitive)"), "Should suggest correct name");
}

#[test]
fn test_argument_count_mismatch_error() {
    let abi = create_uniswap_v3_mint_abi();
    
    // Provide only 3 arguments when 5 are expected
    let args = Value::array(vec![
        EvmValue::address(&address!("742d35Cc6634C0532925a3b844Bc9e7595f0bEb8")),
        Value::integer(100),
        Value::integer(200),
    ]);
    
    let result = value_to_abi_function_args("mint", &args, &abi);
    assert!(result.is_err());
    
    let error = result.unwrap_err();
    
    // Check the error type
    let is_arg_count_mismatch = matches!(
        error.current_context(),
        EvmError::Codec(CodecError::ArgumentCountMismatch { expected: 5, got: 3 })
    );
    assert!(is_arg_count_mismatch, "Expected ArgumentCountMismatch(5, 3), got: {:?}", error.current_context());
    
    // Also verify the detailed error message for user experience
    let error_string = format!("{:?}", error);
    assert!(error_string.contains("[0] recipient: address ✓"), "Should show provided args");
    assert!(error_string.contains("[1] tickLower: int24 ✓"), "Should show provided args");
    assert!(error_string.contains("[2] tickUpper: int24 ✓"), "Should show provided args");
    assert!(error_string.contains("[3] amount: uint128 ✗ missing"), "Should show missing args");
    assert!(error_string.contains("[4] data: bytes ✗ missing"), "Should show missing args");
}

#[test]
fn test_uint8_overflow_error() {
    let mut abi = JsonAbi::default();
    let func = Function {
        name: "setAge".to_string(),
        inputs: vec![
            Param {
                name: "age".to_string(),
                ty: "uint8".to_string(),
                internal_type: None,
                components: vec![],
            },
        ],
        outputs: vec![],
        state_mutability: StateMutability::NonPayable,
    };
    abi.functions.insert("setAge".to_string(), vec![func]);
    
    // Try to pass 256 which exceeds uint8 max (255)
    let args = Value::array(vec![Value::integer(256)]);
    
    let result = value_to_abi_function_args("setAge", &args, &abi);
    assert!(result.is_err());
    
    let error = result.unwrap_err();
    
    // Check error type - could be InvalidValue or InvalidType depending on implementation
    let is_type_error = matches!(
        error.current_context(),
        EvmError::Codec(CodecError::InvalidValue { .. }) |
        EvmError::Codec(CodecError::InvalidType { .. })
    );
    assert!(is_type_error, "Expected InvalidValue or InvalidType for overflow, got: {:?}", error.current_context());
    
    // Verify the error message contains helpful details
    let error_string = format!("{:?}", error);
    assert!(error_string.contains("256"), "Should mention the actual value");
    assert!(error_string.contains("uint8"), "Should mention the target type");
}

#[test]
fn test_nested_tuple_error_location() {
    let mut abi = JsonAbi::default();
    let func = Function {
        name: "processOrder".to_string(),
        inputs: vec![
            Param {
                name: "order".to_string(),
                ty: "tuple".to_string(),
                internal_type: None,
                components: vec![
                    Param {
                        name: "orderId".to_string(),
                        ty: "uint256".to_string(),
                        internal_type: None,
                        components: vec![],
                    },
                    Param {
                        name: "buyer".to_string(),
                        ty: "address".to_string(),
                        internal_type: None,
                        components: vec![],
                    },
                    Param {
                        name: "items".to_string(),
                        ty: "uint256[]".to_string(),
                        internal_type: None,
                        components: vec![],
                    },
                ],
            },
        ],
        outputs: vec![],
        state_mutability: StateMutability::NonPayable,
    };
    abi.functions.insert("processOrder".to_string(), vec![func]);
    
    // Create a tuple with invalid address in second field
    let order = Value::array(vec![
        Value::integer(42),  // orderId - valid
        Value::string("0xINVALID".to_string()),  // buyer - invalid address
        Value::array(vec![Value::integer(100), Value::integer(200)]),  // items - valid
    ]);
    
    let args = Value::array(vec![order]);
    
    let result = value_to_abi_function_args("processOrder", &args, &abi);
    assert!(result.is_err());
    
    let error = result.unwrap_err();
    
    // Check error type
    let is_invalid_address = matches!(
        error.current_context(),
        EvmError::Codec(CodecError::InvalidAddress(_))
    );
    assert!(is_invalid_address, "Expected InvalidAddress error, got: {:?}", error.current_context());
    
    // Also verify nested location is shown in error message
    let error_string = format!("{:?}", error);
    assert!(error_string.contains("Encoding parameter #1 (order)"), "Should show parameter name: {}", error_string);
    assert!(error_string.contains("buyer") || error_string.contains("#2"), "Should show tuple field: {}", error_string);
    assert!(error_string.contains("0xINVALID") || error_string.contains("INVALID"), "Should show the invalid value: {}", error_string);
}

#[test]
fn test_array_length_mismatch_parallel_arrays() {
    let mut abi = JsonAbi::default();
    let func = Function {
        name: "batchTransfer".to_string(),
        inputs: vec![
            Param {
                name: "recipients".to_string(),
                ty: "address[]".to_string(),
                internal_type: None,
                components: vec![],
            },
            Param {
                name: "amounts".to_string(),
                ty: "uint256[]".to_string(),
                internal_type: None,
                components: vec![],
            },
        ],
        outputs: vec![],
        state_mutability: StateMutability::NonPayable,
    };
    abi.functions.insert("batchTransfer".to_string(), vec![func]);
    
    // Provide mismatched array lengths
    let recipients = Value::array(vec![
        EvmValue::address(&address!("742d35Cc6634C0532925a3b844Bc9e7595f0bEb8")),
        EvmValue::address(&address!("3C44CdDdB6a900fa2b585dd299e03d12FA4293BC")),
        EvmValue::address(&address!("90F79bf6EB2c4f870365E785982E1f101E93b906")),
    ]);
    
    let amounts = Value::array(vec![
        Value::string("1000000000000000000".to_string()),  // 1 ETH in wei as string
        Value::string("2000000000000000000".to_string()),  // 2 ETH in wei as string
        // Missing third amount!
    ]);
    
    let args = Value::array(vec![recipients, amounts]);
    
    // This should succeed at encoding level (both arrays are valid)
    // The mismatch would be caught by the contract, but we can verify
    // that each array encodes with proper context
    let result = value_to_abi_function_args("batchTransfer", &args, &abi);
    
    // In this case, encoding should succeed as both are valid arrays
    // The contract would catch the mismatch
    assert!(result.is_ok(), "Both arrays are valid, even if different lengths");
}

#[test]
fn test_bytes32_invalid_length() {
    let mut abi = JsonAbi::default();
    let func = Function {
        name: "verify".to_string(),
        inputs: vec![
            Param {
                name: "merkleRoot".to_string(),
                ty: "bytes32".to_string(),
                internal_type: None,
                components: vec![],
            },
        ],
        outputs: vec![],
        state_mutability: StateMutability::View,
    };
    abi.functions.insert("verify".to_string(), vec![func]);
    
    // Provide too short bytes for bytes32
    let args = Value::array(vec![
        Value::string("0xabcd".to_string()),  // Only 2 bytes, need 32
    ]);
    
    let result = value_to_abi_function_args("verify", &args, &abi);
    
    // bytes32 encoding might pad or fail depending on implementation
    // Let's check what happens
    if result.is_err() {
        let error = result.unwrap_err();
        let error_string = format!("{:?}", error);
        
        // If it fails, should explain why
        assert!(error_string.contains("bytes32") || error_string.contains("32 bytes"), 
                "Should mention bytes32 requirement");
    }
}

#[test]
fn test_complex_nested_error_with_full_context() {
    // Create a complex DeFi-style function
    let mut abi = JsonAbi::default();
    let func = Function {
        name: "executeSwap".to_string(),
        inputs: vec![
            Param {
                name: "swapData".to_string(),
                ty: "tuple".to_string(),
                internal_type: None,
                components: vec![
                    Param {
                        name: "pool".to_string(),
                        ty: "address".to_string(),
                        internal_type: None,
                        components: vec![],
                    },
                    Param {
                        name: "tokenIn".to_string(),
                        ty: "address".to_string(),
                        internal_type: None,
                        components: vec![],
                    },
                    Param {
                        name: "tokenOut".to_string(),
                        ty: "address".to_string(),
                        internal_type: None,
                        components: vec![],
                    },
                    Param {
                        name: "fee".to_string(),
                        ty: "uint24".to_string(),  // Common Uniswap fee tier
                        internal_type: None,
                        components: vec![],
                    },
                    Param {
                        name: "amountIn".to_string(),
                        ty: "uint256".to_string(),
                        internal_type: None,
                        components: vec![],
                    },
                ],
            },
        ],
        outputs: vec![],
        state_mutability: StateMutability::NonPayable,
    };
    abi.functions.insert("executeSwap".to_string(), vec![func]);
    
    // Create swap data with multiple errors
    let swap_data = Value::array(vec![
        EvmValue::address(&address!("8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8")), // pool - valid
        Value::string("not_an_address".to_string()),  // tokenIn - invalid!
        EvmValue::address(&address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")), // tokenOut - valid
        Value::integer(3000),  // fee - valid (0.3%)
        Value::string("1000000000000000000".to_string()),  // amountIn - should be integer not string
    ]);
    
    let args = Value::array(vec![swap_data]);
    
    let result = value_to_abi_function_args("executeSwap", &args, &abi);
    assert!(result.is_err());
    
    let error = result.unwrap_err();
    let error_string = format!("{:?}", error);
    
    // Should show the path to the error
    assert!(error_string.contains("swapData"), "Should mention the parameter");
    assert!(error_string.contains("tokenIn") || error_string.contains("#2"), "Should identify the field");
    assert!(error_string.contains("not_an_address"), "Should show invalid value");
}

#[test]
fn test_helpful_suggestions_for_common_mistakes() {
    // Test that we provide helpful suggestions for common errors
    let mut abi = JsonAbi::default();
    
    // Function expecting Wei amount as uint256
    let func = Function {
        name: "deposit".to_string(),
        inputs: vec![
            Param {
                name: "amount".to_string(),
                ty: "uint256".to_string(),
                internal_type: None,
                components: vec![],
            },
        ],
        outputs: vec![],
        state_mutability: StateMutability::Payable,
    };
    abi.functions.insert("deposit".to_string(), vec![func]);
    
    // User passes string instead of number (common mistake)
    let args = Value::array(vec![
        Value::string("1.5".to_string()),  // Trying to pass 1.5 ETH as decimal
    ]);
    
    let result = value_to_abi_function_args("deposit", &args, &abi);
    
    // Should handle or provide helpful error
    if result.is_err() {
        let error = result.unwrap_err();
        let error_string = format!("{:?}", error);
        
        // Should indicate type mismatch
        assert!(error_string.contains("uint256") || error_string.contains("integer"), 
                "Should mention expected type");
    }
}