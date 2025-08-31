/// Basic tests that establish the foundation for codec module testing
/// These tests focus on the core functionality without complex dependencies

use crate::codec::transaction::types::{TransactionType, CommonTransactionFields};
use crate::codec::transaction::cost::format_transaction_cost;
use crate::codec::conversion::string_to_address;
use crate::codec::abi::types::value_to_sol_value;
use crate::codec::abi::decoding::sol_value_to_value;
use crate::typing::EvmValue;
use alloy::dyn_abi::DynSolValue;
use alloy::primitives::{Address, U256};
use txtx_addon_kit::types::types::Value;

#[test]
fn test_transaction_type_parsing() {
    // Test valid transaction types
    assert!(matches!(
        TransactionType::from_str("legacy").unwrap(),
        TransactionType::Legacy
    ));
    assert!(matches!(
        TransactionType::from_str("eip1559").unwrap(),
        TransactionType::EIP1559
    ));
    
    // Test case insensitive
    assert!(matches!(
        TransactionType::from_str("LEGACY").unwrap(),
        TransactionType::Legacy
    ));
    
    // Test invalid type
    assert!(TransactionType::from_str("invalid").is_err());
    
    // Test from_some_value with None (defaults to EIP1559)
    assert!(matches!(
        TransactionType::from_some_value(None).unwrap(),
        TransactionType::EIP1559
    ));
}

#[test]
fn test_string_to_address_basic() {
    // Test with 0x prefix
    let addr_str = "0x0000000000000000000000000000000000000001".to_string();
    let result = string_to_address(addr_str);
    assert!(result.is_ok());
    
    // Test without 0x prefix
    let addr_str = "0000000000000000000000000000000000000002".to_string();
    let result = string_to_address(addr_str);
    assert!(result.is_ok());
    
    // Test invalid hex
    let invalid_str = "0xGGGG".to_string();
    let result = string_to_address(invalid_str);
    assert!(result.is_err());
}

#[test]
fn test_format_transaction_cost_basic() {
    // Test formatting 1 ETH
    let cost: i128 = 1_000_000_000_000_000_000;
    let result = format_transaction_cost(cost);
    assert!(result.is_ok());
    
    // Test formatting 0 wei
    let cost: i128 = 0;
    let result = format_transaction_cost(cost);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "0.0");
}

#[test]
fn test_common_transaction_fields_structure() {
    use alloy::primitives::address;
    
    let from = address!("0000000000000000000000000000000000000001");
    let to = address!("0000000000000000000000000000000000000002");
    
    let fields = CommonTransactionFields {
        to: Some(EvmValue::address(&to)),
        from: EvmValue::address(&from),
        nonce: Some(42),
        chain_id: 1,
        amount: 1000000000000000000, // 1 ETH
        gas_limit: Some(21000),
        input: Some(vec![0x01, 0x02, 0x03]),
        tx_type: TransactionType::Legacy,
        deploy_code: None,
    };
    
    assert_eq!(fields.nonce, Some(42));
    assert_eq!(fields.chain_id, 1);
    assert_eq!(fields.amount, 1000000000000000000);
    assert!(matches!(fields.tx_type, TransactionType::Legacy));
}

#[test]
fn test_value_to_sol_value_basic() {
    use txtx_addon_kit::types::types::Value;
    use alloy::primitives::U256;
    
    // Test bool conversion
    let value = Value::bool(true);
    let sol_value = value_to_sol_value(&value);
    assert!(sol_value.is_ok());
    match sol_value.unwrap() {
        DynSolValue::Bool(b) => assert!(b),
        _ => panic!("Expected bool"),
    }
    
    // Test integer conversion
    let value = Value::integer(42);
    let sol_value = value_to_sol_value(&value);
    assert!(sol_value.is_ok());
    match sol_value.unwrap() {
        DynSolValue::Uint(val, bits) => {
            assert_eq!(val, U256::from(42));
            assert_eq!(bits, 256);
        }
        _ => panic!("Expected uint256"),
    }
    
    // Test string conversion
    let value = Value::string("test".to_string());
    let sol_value = value_to_sol_value(&value);
    assert!(sol_value.is_ok());
    match sol_value.unwrap() {
        DynSolValue::String(s) => assert_eq!(s, "test"),
        _ => panic!("Expected string"),
    }
}

#[test]
fn test_sol_value_to_value_basic() {
    use txtx_addon_kit::types::types::Value;
    use alloy::primitives::U256;
    
    // Test bool
    let sol_bool = DynSolValue::Bool(false);
    let value = sol_value_to_value(&sol_bool);
    assert!(value.is_ok());
    assert_eq!(value.unwrap(), Value::bool(false));
    
    // Test uint256 (small value)
    let sol_uint = DynSolValue::Uint(U256::from(12345), 256);
    let value = sol_value_to_value(&sol_uint);
    assert!(value.is_ok());
    assert_eq!(value.unwrap(), Value::integer(12345));
    
    // Test string
    let sol_string = DynSolValue::String("Hello".to_string());
    let value = sol_value_to_value(&sol_string);
    assert!(value.is_ok());
    assert_eq!(value.unwrap(), Value::string("Hello".to_string()));
}