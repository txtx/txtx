use crate::codec::display::{format_transaction_for_display, format_access_list_for_display};
use alloy::consensus::{TxLegacy, TxEip1559, TxEip2930, TypedTransaction};
use alloy::primitives::{address, B256, U256, TxKind};
use alloy::rpc::types::{AccessListItem, AccessList};
use txtx_addon_kit::types::types::Value;

#[test]
fn test_format_transaction_for_display_legacy() {
    let legacy_tx = TxLegacy {
        chain_id: Some(1),
        nonce: 42,
        gas_price: 20_000_000_000, // 20 gwei
        gas_limit: 21000,
        to: TxKind::Call(address!("0000000000000000000000000000000000000002")),
        value: U256::from(1_000_000_000_000_000_000u128), // 1 ETH
        input: vec![0x12, 0x34].into(),
    };
    
    let typed_tx = TypedTransaction::Legacy(legacy_tx);
    let display_value = format_transaction_for_display(&typed_tx);
    
    let obj = display_value.as_object().unwrap();
    
    // Check required fields
    assert!(obj.contains_key("kind"));
    assert!(obj.contains_key("nonce"));
    assert!(obj.contains_key("gas_limit"));
    assert!(obj.contains_key("input"));
    assert!(obj.contains_key("value"));
    assert!(obj.contains_key("type"));
    assert!(obj.contains_key("chain_id"));
    assert!(obj.contains_key("gas_price"));
    
    // Verify values
    assert_eq!(obj.get("nonce").unwrap(), &Value::integer(42));
    assert_eq!(obj.get("gas_limit").unwrap(), &Value::integer(21000));
    assert_eq!(obj.get("chain_id").unwrap(), &Value::integer(1));
    assert_eq!(obj.get("gas_price").unwrap(), &Value::integer(20_000_000_000));
    assert_eq!(obj.get("type").unwrap(), &Value::string("Legacy".to_string())); // Legacy type
}

#[test]
fn test_format_transaction_for_display_eip1559() {
    let eip1559_tx = TxEip1559 {
        chain_id: 1,
        nonce: 10,
        max_fee_per_gas: 30_000_000_000, // 30 gwei
        max_priority_fee_per_gas: 2_000_000_000, // 2 gwei
        gas_limit: 50000,
        to: TxKind::Call(address!("0000000000000000000000000000000000000003")),
        value: U256::from(500_000_000_000_000_000u128), // 0.5 ETH
        input: vec![].into(),
        access_list: AccessList::default(),
    };
    
    let typed_tx = TypedTransaction::Eip1559(eip1559_tx);
    let display_value = format_transaction_for_display(&typed_tx);
    
    let obj = display_value.as_object().unwrap();
    
    // Check EIP-1559 specific fields
    assert!(obj.contains_key("max_fee_per_gas"));
    assert!(obj.contains_key("max_priority_fee_per_gas"));
    assert!(obj.contains_key("access_list"));
    
    // Verify values
    assert_eq!(obj.get("nonce").unwrap(), &Value::integer(10));
    assert_eq!(obj.get("gas_limit").unwrap(), &Value::integer(50000));
    assert_eq!(obj.get("max_fee_per_gas").unwrap(), &Value::integer(30_000_000_000));
    assert_eq!(obj.get("max_priority_fee_per_gas").unwrap(), &Value::integer(2_000_000_000));
    assert_eq!(obj.get("type").unwrap(), &Value::string("EIP-1559".to_string())); // EIP-1559 type
}

#[test]
fn test_format_transaction_for_display_create() {
    // Test contract creation (no 'to' address)
    let create_tx = TxLegacy {
        chain_id: Some(1),
        nonce: 0,
        gas_price: 20_000_000_000,
        gas_limit: 200000,
        to: TxKind::Create, // Contract creation
        value: U256::from(0),
        input: vec![0xFF; 100].into(), // Deployment bytecode
    };
    
    let typed_tx = TypedTransaction::Legacy(create_tx);
    let display_value = format_transaction_for_display(&typed_tx);
    
    let obj = display_value.as_object().unwrap();
    
    // For contract creation, 'kind' should be "create"
    assert_eq!(obj.get("kind").unwrap(), &Value::string("create".to_string()));
}

#[test]
fn test_format_transaction_for_display_eip2930() {
    let access_list_items = vec![
        AccessListItem {
            address: address!("0000000000000000000000000000000000000004"),
            storage_keys: vec![
                B256::from([1u8; 32]),
                B256::from([2u8; 32]),
            ],
        },
        AccessListItem {
            address: address!("0000000000000000000000000000000000000005"),
            storage_keys: vec![
                B256::from([3u8; 32]),
            ],
        },
    ];
    
    let eip2930_tx = TxEip2930 {
        chain_id: 1,
        nonce: 5,
        gas_price: 25_000_000_000,
        gas_limit: 30000,
        to: TxKind::Call(address!("0000000000000000000000000000000000000002")),
        value: U256::from(0),
        input: vec![].into(),
        access_list: AccessList::from(access_list_items),
    };
    
    let typed_tx = TypedTransaction::Eip2930(eip2930_tx);
    let display_value = format_transaction_for_display(&typed_tx);
    
    let obj = display_value.as_object().unwrap();
    
    // Check access list is included
    assert!(obj.contains_key("access_list"));
    let access_list = obj.get("access_list").unwrap();
    assert!(access_list.as_array().is_some());
    
    // Verify access list formatting
    let list = access_list.as_array().unwrap();
    assert_eq!(list.len(), 2);
}

#[test]
fn test_format_access_list_for_display() {
    let access_list_items = vec![
        AccessListItem {
            address: address!("0000000000000000000000000000000000000001"),
            storage_keys: vec![
                B256::from([0xAAu8; 32]),
                B256::from([0xBBu8; 32]),
            ],
        },
        AccessListItem {
            address: address!("0000000000000000000000000000000000000002"),
            storage_keys: vec![],
        },
    ];
    
    let access_list = AccessList::from(access_list_items);
    let formatted = format_access_list_for_display(&access_list);
    
    assert_eq!(formatted.len(), 2);
    
    // Check first item
    let first = formatted[0].as_object().unwrap();
    assert!(first.contains_key("address"));
    assert!(first.contains_key("storage_keys"));
    
    let storage_keys = first.get("storage_keys").unwrap().as_array().unwrap();
    assert_eq!(storage_keys.len(), 2);
    
    // Check second item (empty storage keys)
    let second = formatted[1].as_object().unwrap();
    let storage_keys = second.get("storage_keys").unwrap().as_array().unwrap();
    assert_eq!(storage_keys.len(), 0);
}

#[test]
fn test_format_access_list_empty() {
    let access_list = AccessList::default();
    let formatted = format_access_list_for_display(&access_list);
    
    assert_eq!(formatted.len(), 0);
}

#[test]
fn test_format_transaction_for_display_hex_encoding() {
    let input_data = vec![0x12, 0x34, 0xAB, 0xCD];
    
    let tx = TxLegacy {
        chain_id: Some(1),
        nonce: 0,
        gas_price: 20_000_000_000,
        gas_limit: 21000,
        to: TxKind::Call(address!("0000000000000000000000000000000000000002")),
        value: U256::from(0),
        input: input_data.clone().into(),
    };
    
    let typed_tx = TypedTransaction::Legacy(tx);
    let display_value = format_transaction_for_display(&typed_tx);
    
    let obj = display_value.as_object().unwrap();
    let input = obj.get("input").unwrap().as_string().unwrap();
    
    // Input should be hex encoded
    assert_eq!(input, "1234abcd");
}

// EIP-4844 test removed - Default not implemented for TxEip4844Variant

// EIP-7702 test removed - Default not implemented

#[test]
fn test_format_transaction_value_display() {
    // Test that value is formatted in ether units
    let tx = TxLegacy {
        chain_id: Some(1),
        nonce: 0,
        gas_price: 20_000_000_000,
        gas_limit: 21000,
        to: TxKind::Call(address!("0000000000000000000000000000000000000002")),
        value: U256::from(1_500_000_000_000_000_000u128), // 1.5 ETH
        input: vec![].into(),
    };
    
    let typed_tx = TypedTransaction::Legacy(tx);
    let display_value = format_transaction_for_display(&typed_tx);
    
    let obj = display_value.as_object().unwrap();
    let value = obj.get("value").unwrap().as_string().unwrap();
    
    // Value should be formatted as "1.5" (in ether)
    assert!(value.contains("1.5"));
}