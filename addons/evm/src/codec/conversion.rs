// Type conversion utilities for EVM codec

use alloy::consensus::{SignableTransaction, TypedTransaction};
use alloy::hex::FromHex;
use alloy::primitives::Address;
use alloy::rpc::types::TransactionRequest;
use error_stack::{Report, ResultExt};

use crate::errors::{EvmError, EvmResult, CodecError};

/// Convert a string to an Ethereum address
/// Handles both with and without 0x prefix
/// Also handles 32-byte padded addresses
pub fn string_to_address(address_str: String) -> EvmResult<Address> {
    let mut address_str = address_str.replace("0x", "");
    
    // Hack: we're assuming that if the address is 32 bytes, 
    // it's a sol value that's padded with 0s, so we trim them
    if address_str.len() == 64 {
        let split_pos = address_str.char_indices()
            .nth_back(39)
            .ok_or_else(|| Report::new(EvmError::Codec(
                CodecError::InvalidAddress(format!("Invalid padded address format: {}", address_str))
            )))?
            .0;
        address_str = address_str[split_pos..].to_owned();
    }
    
    let address = Address::from_hex(&address_str)
        .map_err(|e| Report::new(EvmError::Codec(
            CodecError::InvalidAddress(format!("{}: {}", address_str, e))
        )))
        .attach_printable(format!("Parsing address: {}", address_str))?;
    Ok(address)
}

/// Get the bytes of a transaction request for serialization
pub fn get_typed_transaction_bytes(tx: &TransactionRequest) -> EvmResult<Vec<u8>> {
    serde_json::to_vec(&tx)
        .map_err(|e| Report::new(EvmError::Codec(
            CodecError::SerializationFailed(format!("Transaction serialization failed: {}", e))
        )))
        .attach_printable("Serializing transaction request to bytes")
}

/// Get the bytes of a typed transaction for signing
pub fn typed_transaction_bytes(typed_transaction: &TypedTransaction) -> Vec<u8> {
    let mut bytes = vec![];
    match typed_transaction {
        TypedTransaction::Legacy(tx) => tx.encode_for_signing(&mut bytes),
        TypedTransaction::Eip2930(tx) => tx.encode_for_signing(&mut bytes),
        TypedTransaction::Eip1559(tx) => tx.encode_for_signing(&mut bytes),
        TypedTransaction::Eip4844(tx) => tx.encode_for_signing(&mut bytes),
        TypedTransaction::Eip7702(tx) => tx.encode_for_signing(&mut bytes),
    }
    bytes
}