//! Centralized error handling for the EVM addon using error-stack
//! 
//! This module provides rich error context and stack traces while maintaining
//! compatibility with the existing Diagnostic system.

use error_stack::{Report, Context};
use std::fmt;
use txtx_addon_kit::types::diagnostics::Diagnostic;
use alloy::primitives::Address;

/// Root error type for all EVM operations
#[derive(Debug, Clone)]
pub enum EvmError {
    /// Transaction-related errors
    Transaction(TransactionError),
    /// RPC communication errors  
    Rpc(RpcError),
    /// Smart contract interaction errors
    Contract(ContractError),
    /// Contract verification errors
    Verification(VerificationError),
    /// ABI encoding/decoding errors
    Codec(CodecError),
    /// Signer-related errors
    Signer(SignerError),
    /// Configuration errors
    Config(ConfigError),
}

impl fmt::Display for EvmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transaction(e) => write!(f, "Transaction error: {}", e),
            Self::Rpc(e) => write!(f, "RPC error: {}", e),
            Self::Contract(e) => write!(f, "Contract error: {}", e),
            Self::Verification(e) => write!(f, "Verification error: {}", e),
            Self::Codec(e) => write!(f, "Codec error: {}", e),
            Self::Signer(e) => write!(f, "Signer error: {}", e),
            Self::Config(e) => write!(f, "Configuration error: {}", e),
        }
    }
}

impl Context for EvmError {}

/// Transaction-specific errors
#[derive(Debug, Clone)]
pub enum TransactionError {
    InvalidType(String),
    InsufficientFunds { required: u128, available: u128 },
    InvalidNonce { expected: u64, provided: u64 },
    GasEstimationFailed,
    SigningFailed,
    BroadcastFailed,
    InvalidRecipient(String),
}

impl fmt::Display for TransactionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidType(t) => write!(f, "Invalid transaction type: {}", t),
            Self::InsufficientFunds { required, available } => {
                write!(f, "Insufficient funds: required {}, available {}", required, available)
            }
            Self::InvalidNonce { expected, provided } => {
                write!(f, "Invalid nonce: expected {}, provided {}", expected, provided)
            }
            Self::GasEstimationFailed => write!(f, "Failed to estimate gas"),
            Self::SigningFailed => write!(f, "Failed to sign transaction"),
            Self::BroadcastFailed => write!(f, "Failed to broadcast transaction"),
            Self::InvalidRecipient(addr) => write!(f, "Invalid recipient address: {}", addr),
        }
    }
}

impl Context for TransactionError {}

/// RPC communication errors
#[derive(Debug, Clone)]
pub enum RpcError {
    ConnectionFailed(String),
    RequestTimeout,
    InvalidResponse(String),
    NodeError(String),
    RateLimited,
}

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConnectionFailed(url) => write!(f, "Failed to connect to RPC endpoint: {}", url),
            Self::RequestTimeout => write!(f, "RPC request timed out"),
            Self::InvalidResponse(msg) => write!(f, "Invalid RPC response: {}", msg),
            Self::NodeError(msg) => write!(f, "RPC node error: {}", msg),
            Self::RateLimited => write!(f, "RPC rate limit exceeded"),
        }
    }
}

impl Context for RpcError {}

/// Smart contract errors
#[derive(Debug, Clone)]
pub enum ContractError {
    NotDeployed(Address),
    InvalidAbi(String),
    FunctionNotFound(String),
    InvalidArguments(String),
    ExecutionReverted(String),
    DeploymentFailed(String),
}

impl fmt::Display for ContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotDeployed(addr) => write!(f, "Contract not deployed at address: {}", addr),
            Self::InvalidAbi(msg) => write!(f, "Invalid contract ABI: {}", msg),
            Self::FunctionNotFound(name) => write!(f, "Function '{}' not found in ABI", name),
            Self::InvalidArguments(msg) => write!(f, "Invalid function arguments: {}", msg),
            Self::ExecutionReverted(msg) => write!(f, "Contract execution reverted: {}", msg),
            Self::DeploymentFailed(msg) => write!(f, "Contract deployment failed: {}", msg),
        }
    }
}

impl Context for ContractError {}

/// Verification errors
#[derive(Debug, Clone)]
pub enum VerificationError {
    ProviderUnavailable(String),
    InvalidSourceCode,
    CompilationMismatch,
    AlreadyVerified,
    VerificationTimeout,
    ApiError(String),
}

impl fmt::Display for VerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProviderUnavailable(p) => write!(f, "Verification provider unavailable: {}", p),
            Self::InvalidSourceCode => write!(f, "Invalid source code for verification"),
            Self::CompilationMismatch => write!(f, "Compiled bytecode doesn't match on-chain code"),
            Self::AlreadyVerified => write!(f, "Contract already verified"),
            Self::VerificationTimeout => write!(f, "Verification request timed out"),
            Self::ApiError(msg) => write!(f, "Verification API error: {}", msg),
        }
    }
}

impl Context for VerificationError {}

/// Codec/encoding errors
#[derive(Debug, Clone)]
pub enum CodecError {
    InvalidHex(String),
    InvalidAddress(String),
    AbiEncodingFailed(String),
    AbiDecodingFailed(String),
    InvalidType { expected: String, received: String },
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHex(s) => write!(f, "Invalid hex string: {}", s),
            Self::InvalidAddress(s) => write!(f, "Invalid address: {}", s),
            Self::AbiEncodingFailed(msg) => write!(f, "ABI encoding failed: {}", msg),
            Self::AbiDecodingFailed(msg) => write!(f, "ABI decoding failed: {}", msg),
            Self::InvalidType { expected, received } => {
                write!(f, "Type mismatch: expected {}, received {}", expected, received)
            }
        }
    }
}

impl Context for CodecError {}

/// Signer errors
#[derive(Debug, Clone)]
pub enum SignerError {
    KeyNotFound,
    InvalidPrivateKey,
    InvalidMnemonic,
    DerivationFailed,
    SignatureFailed,
    Locked,
}

impl fmt::Display for SignerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KeyNotFound => write!(f, "Signer key not found"),
            Self::InvalidPrivateKey => write!(f, "Invalid private key"),
            Self::InvalidMnemonic => write!(f, "Invalid mnemonic phrase"),
            Self::DerivationFailed => write!(f, "Key derivation failed"),
            Self::SignatureFailed => write!(f, "Failed to create signature"),
            Self::Locked => write!(f, "Signer is locked"),
        }
    }
}

impl Context for SignerError {}

/// Configuration errors
#[derive(Debug, Clone)]
pub enum ConfigError {
    MissingField(String),
    InvalidValue { field: String, value: String },
    FileNotFound(String),
    ParseError(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingField(field) => write!(f, "Missing required field: {}", field),
            Self::InvalidValue { field, value } => {
                write!(f, "Invalid value '{}' for field '{}'", value, field)
            }
            Self::FileNotFound(path) => write!(f, "Configuration file not found: {}", path),
            Self::ParseError(msg) => write!(f, "Failed to parse configuration: {}", msg),
        }
    }
}

impl Context for ConfigError {}

/// Context attachments for rich error information
#[derive(Debug, Clone)]
pub struct TransactionContext {
    pub tx_hash: Option<String>,
    pub from: Option<Address>,
    pub to: Option<Address>,
    pub value: Option<u128>,
    pub gas_limit: Option<u64>,
    pub chain_id: u64,
}

#[derive(Debug, Clone)]
pub struct RpcContext {
    pub endpoint: String,
    pub method: String,
    pub params: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ContractContext {
    pub address: Address,
    pub function: Option<String>,
    pub args: Option<String>,
}

/// Wrapper type to enable conversion to Diagnostic
pub struct EvmErrorReport(pub Report<EvmError>);

impl From<Report<EvmError>> for EvmErrorReport {
    fn from(report: Report<EvmError>) -> Self {
        EvmErrorReport(report)
    }
}

impl From<EvmErrorReport> for Diagnostic {
    fn from(wrapper: EvmErrorReport) -> Self {
        let report = wrapper.0;
        
        // Build the error message chain
        let error_chain = format!("{:?}", report);
        
        // Extract main error message
        let main_message = report.to_string();
        
        // Create diagnostic with full context
        let mut diagnostic = Diagnostic::error_from_string(main_message);
        
        // Add the full error chain as documentation for debugging
        diagnostic.documentation = Some(format!("Full error context:\n{}", error_chain));
        
        diagnostic
    }
}

/// Convert a Report<EvmError> to Diagnostic
pub fn report_to_diagnostic(report: Report<EvmError>) -> Diagnostic {
    EvmErrorReport(report).into()
}

/// Helper trait for converting existing errors to error-stack
pub trait IntoEvmError {
    fn into_evm_error(self) -> Report<EvmError>;
}

impl IntoEvmError for String {
    fn into_evm_error(self) -> Report<EvmError> {
        Report::new(EvmError::Config(ConfigError::ParseError(self)))
    }
}

impl IntoEvmError for Diagnostic {
    fn into_evm_error(self) -> Report<EvmError> {
        Report::new(EvmError::Config(ConfigError::ParseError(self.message)))
    }
}

/// Convenience type alias for EVM results
pub type EvmResult<T> = error_stack::Result<T, EvmError>;

/// Helper macros for attaching context
#[macro_export]
macro_rules! attach_tx_context {
    ($result:expr, $tx_hash:expr, $from:expr, $to:expr) => {
        $result.attach($crate::errors::TransactionContext {
            tx_hash: Some($tx_hash.to_string()),
            from: Some($from),
            to: $to,
            value: None,
            gas_limit: None,
            chain_id: 0,
        })
    };
}

#[macro_export]
macro_rules! attach_rpc_context {
    ($result:expr, $endpoint:expr, $method:expr) => {
        $result.attach($crate::errors::RpcContext {
            endpoint: $endpoint.to_string(),
            method: $method.to_string(),
            params: None,
        })
    };
}

#[macro_export]
macro_rules! attach_contract_context {
    ($result:expr, $address:expr, $function:expr) => {
        $result.attach($crate::errors::ContractContext {
            address: $address,
            function: Some($function.to_string()),
            args: None,
        })
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use error_stack::ResultExt;

    #[test]
    fn test_error_chain_with_context() {
        fn inner_operation() -> EvmResult<()> {
            Err(Report::new(EvmError::Rpc(RpcError::ConnectionFailed(
                "http://localhost:8545".to_string()
            ))))
        }

        fn middle_operation() -> EvmResult<()> {
            inner_operation()
                .attach(RpcContext {
                    endpoint: "http://localhost:8545".to_string(),
                    method: "eth_getBalance".to_string(),
                    params: Some("[\"0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb\", \"latest\"]".to_string()),
                })
                .change_context(EvmError::Transaction(TransactionError::InsufficientFunds {
                    required: 1000,
                    available: 0,
                }))
        }

        fn outer_operation() -> EvmResult<()> {
            middle_operation()
                .attach_printable("Attempting to send transaction")
                .attach(TransactionContext {
                    tx_hash: None,
                    from: None,
                    to: None,
                    value: Some(1000),
                    gas_limit: Some(21000),
                    chain_id: 1,
                })
        }

        let result = outer_operation();
        assert!(result.is_err());
        
        // Convert to diagnostic to verify compatibility
        let diagnostic = report_to_diagnostic(result.unwrap_err());
        assert!(diagnostic.message.contains("Transaction error"));
        assert!(diagnostic.documentation.is_some());
    }

    #[test]
    fn test_diagnostic_conversion() {
        let report = Report::new(EvmError::Contract(ContractError::FunctionNotFound(
            "transfer".to_string()
        )))
        .attach_printable("While calling ERC20 contract")
        .attach(ContractContext {
            address: Address::ZERO,
            function: Some("transfer".to_string()),
            args: Some("(address,uint256)".to_string()),
        });

        let diagnostic = report_to_diagnostic(report);
        assert!(diagnostic.message.contains("Function 'transfer' not found"));
        assert!(diagnostic.documentation.is_some());
    }
}
