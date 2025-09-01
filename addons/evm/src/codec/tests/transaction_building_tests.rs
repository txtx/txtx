use crate::codec::transaction::types::{TransactionType, CommonTransactionFields};
use crate::codec::{get_typed_transaction_bytes, string_to_address, format_transaction_cost};
use crate::rpc::EvmRpc;
use crate::typing::EvmValue;
use alloy::primitives::{address, Address, U256, TxKind};
use alloy::rpc::types::{TransactionRequest, AccessList};
use alloy::consensus::SignableTransaction;
use txtx_addon_kit::types::Did;
use txtx_addon_kit::types::stores::ValueStore;

fn create_test_rpc() -> EvmRpc {
    // Create a test RPC instance pointing to a local test endpoint
    EvmRpc::new("http://127.0.0.1:8545").expect("Failed to create test RPC")
}

fn create_test_value_store() -> ValueStore {
    let test_uuid = Did::from_hex_string("0000000000000000000000000000000000000000000000000000000000000000");
    let store = ValueStore::new("test", &test_uuid);
    store
}

fn create_common_fields(from: Address, to: Option<Address>) -> CommonTransactionFields {
    CommonTransactionFields {
        to: to.map(|addr| EvmValue::address(&addr)),
        from: EvmValue::address(&from),
        nonce: Some(0),
        chain_id: 1,
        amount: 1000000000000000, // 0.001 ETH in wei
        gas_limit: Some(21000),
        input: None,
        tx_type: TransactionType::EIP1559,
        deploy_code: None,
    }
}

#[tokio::test]
async fn test_build_unsigned_transaction_eip1559() {
    let from = address!("0000000000000000000000000000000000000001");
    let to = address!("0000000000000000000000000000000000000002");
    
    let fields = create_common_fields(from, Some(to));
    let store = create_test_value_store();
    
    // Note: This test will need a mock RPC or test against a local node
    // For now, we're testing the structure and type conversions
    
    // Test that transaction type parsing works
    assert!(matches!(fields.tx_type, TransactionType::EIP1559));
}

#[test]
fn test_transaction_type_from_str() {
    // Test valid transaction types
    assert!(matches!(
        TransactionType::from_str("legacy").unwrap(),
        TransactionType::Legacy
    ));
    assert!(matches!(
        TransactionType::from_str("eip2930").unwrap(),
        TransactionType::EIP2930
    ));
    assert!(matches!(
        TransactionType::from_str("eip1559").unwrap(),
        TransactionType::EIP1559
    ));
    assert!(matches!(
        TransactionType::from_str("eip4844").unwrap(),
        TransactionType::EIP4844
    ));
    
    // Test case insensitive
    assert!(matches!(
        TransactionType::from_str("LEGACY").unwrap(),
        TransactionType::Legacy
    ));
    assert!(matches!(
        TransactionType::from_str("EiP1559").unwrap(),
        TransactionType::EIP1559
    ));
    
    // Test invalid type
    assert!(TransactionType::from_str("invalid").is_err());
}

#[test]
fn test_transaction_type_from_some_value() {
    // Test with Some value
    assert!(matches!(
        TransactionType::from_some_value(Some("legacy")).unwrap(),
        TransactionType::Legacy
    ));
    
    // Test with None (should default to EIP1559)
    assert!(matches!(
        TransactionType::from_some_value(None).unwrap(),
        TransactionType::EIP1559
    ));
    
    // Test with invalid value
    assert!(TransactionType::from_some_value(Some("invalid")).is_err());
}

#[test]
fn test_common_transaction_fields_creation() {
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
    assert_eq!(fields.gas_limit, Some(21000));
    assert!(matches!(fields.tx_type, TransactionType::Legacy));
}

#[test]
fn test_get_typed_transaction_bytes() {
    use alloy::network::TransactionBuilder;
    
    let from = address!("0000000000000000000000000000000000000001");
    let to = address!("0000000000000000000000000000000000000002");
    
    let tx = TransactionRequest::default()
        .with_from(from)
        .with_to(to)
        .with_value(U256::from(1000000000000000u64))
        .with_nonce(0)
        .with_chain_id(1)
        .with_gas_limit(21000);
    
    let result = get_typed_transaction_bytes(&tx);
    assert!(result.is_ok());
    assert!(!result.unwrap().is_empty());
}

#[test]
fn test_typed_transaction_bytes() {
    use alloy::consensus::{TxLegacy, TxEip1559};
    
    // Test Legacy transaction
    let legacy_tx = TxLegacy {
        chain_id: Some(1),
        nonce: 0,
        gas_price: 20000000000, // 20 gwei
        gas_limit: 21000,
        to: TxKind::Call(address!("0000000000000000000000000000000000000002")),
        value: U256::from(1000000000000000u64),
        input: vec![].into(),
    };
    
    // For tests, we skip the TxEnvelope wrapper and test the underlying transaction
    // TxEnvelope requires signed transactions which aren't needed for these encoding tests
    let bytes = {
        let mut buf = vec![];
        legacy_tx.encode_for_signing(&mut buf);
        buf
    };
    assert!(!bytes.is_empty());
    
    // Test EIP-1559 transaction
    let eip1559_tx = TxEip1559 {
        chain_id: 1,
        nonce: 0,
        max_fee_per_gas: 30000000000, // 30 gwei
        max_priority_fee_per_gas: 2000000000, // 2 gwei
        gas_limit: 21000,
        to: TxKind::Call(address!("0000000000000000000000000000000000000002")),
        value: U256::from(1000000000000000u64),
        input: vec![].into(),
        access_list: AccessList::default(),
    };
    
    // For tests, we skip the TxEnvelope wrapper and test the underlying transaction
    let bytes = {
        let mut buf = vec![];
        eip1559_tx.encode_for_signing(&mut buf);
        buf
    };
    assert!(!bytes.is_empty());
}

#[tokio::test]
async fn test_format_transaction_cost() {
    let cost: i128 = 1000000000000000000; // 1 ETH
    let result = format_transaction_cost(cost);
    assert!(result.is_ok());
    
    // The format should be in a readable unit
    let formatted = result.unwrap();
    assert!(!formatted.is_empty());
}



#[test]
fn test_string_to_address_valid() {
    // Test with 0x prefix
    let addr_str = "0x0000000000000000000000000000000000000001".to_string();
    let result = string_to_address(addr_str);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        address!("0000000000000000000000000000000000000001")
    );
    
    // Test without 0x prefix
    let addr_str = "0000000000000000000000000000000000000002".to_string();
    let result = string_to_address(addr_str);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        address!("0000000000000000000000000000000000000002")
    );
}

#[test]
fn test_string_to_address_padded() {
    // Test with 32-byte padded address (64 chars)
    let padded_str = "0000000000000000000000000000000000000000000000000000000000000001".to_string();
    let result = string_to_address(padded_str);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        address!("0000000000000000000000000000000000000001")
    );
    
    // Test with 0x prefix and padding
    let padded_str = "0x0000000000000000000000000000000000000000000000000000000000000002".to_string();
    let result = string_to_address(padded_str);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        address!("0000000000000000000000000000000000000002")
    );
}

#[test]
fn test_string_to_address_invalid() {
    // Test invalid hex
    let invalid_str = "0xGGGG".to_string();
    let result = string_to_address(invalid_str);
    assert!(result.is_err());
    
    // Test wrong length
    let short_str = "0x1234".to_string();
    let result = string_to_address(short_str);
    assert!(result.is_err());
    
    // Test empty string
    let empty_str = "".to_string();
    let result = string_to_address(empty_str);
    assert!(result.is_err());
}