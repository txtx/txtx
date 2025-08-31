use serde::{Deserialize, Serialize};
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::types::types::Value;

#[macro_use]
use txtx_addon_kit;

/// Ethereum transaction types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransactionType {
    Legacy,
    EIP2930,
    EIP1559,
    EIP4844,
}

impl TransactionType {
    pub fn from_some_value(input: Option<&str>) -> Result<Self, Diagnostic> {
        input
            .and_then(|t| Some(TransactionType::from_str(t)))
            .unwrap_or(Ok(TransactionType::EIP1559))
    }
    
    pub fn from_str(input: &str) -> Result<Self, Diagnostic> {
        match input.to_ascii_lowercase().as_ref() {
            "legacy" => Ok(TransactionType::Legacy),
            "eip2930" => Ok(TransactionType::EIP2930),
            "eip1559" => Ok(TransactionType::EIP1559),
            "eip4844" => Ok(TransactionType::EIP4844),
            other => Err(diagnosed_error!("invalid Ethereum Transaction type: {}", other)),
        }
    }
}

/// Common fields for all transaction types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommonTransactionFields {
    pub to: Option<Value>,
    pub from: Value,
    pub nonce: Option<u64>,
    pub chain_id: u64,
    pub amount: u64,
    pub gas_limit: Option<u64>,
    pub input: Option<Vec<u8>>,
    pub tx_type: TransactionType,
    pub deploy_code: Option<Vec<u8>>,
}

/// Internal structure for filled transaction fields
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct FilledCommonTransactionFields {
    pub to: Option<alloy::primitives::Address>,
    pub from: alloy::primitives::Address,
    pub nonce: u64,
    pub chain_id: u64,
    pub amount: u64,
    pub gas_limit: Option<u64>,
    pub input: Option<Vec<u8>>,
    pub deploy_code: Option<Vec<u8>>,
}