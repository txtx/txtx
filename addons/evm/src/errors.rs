use error_stack::{Context, Report};
use std::fmt;
use txtx_addon_kit::types::errors::{ErrorAttachments, TxtxError};

/// EVM-specific error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvmError {
    /// Invalid Ethereum address format
    InvalidAddress,
    /// Transaction execution failed
    TransactionFailed,
    /// Contract deployment failed
    ContractDeploymentFailed,
    /// Insufficient funds for operation
    InsufficientFunds,
    /// Contract call failed
    ContractCallFailed,
    /// ABI encoding/decoding error
    AbiError,
    /// RPC communication error
    RpcError,
    /// Gas estimation failed
    GasEstimationFailed,
    /// Invalid chain ID
    InvalidChainId,
    /// Signer error
    SignerError,
}

impl fmt::Display for EvmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvmError::InvalidAddress => write!(f, "Invalid Ethereum address"),
            EvmError::TransactionFailed => write!(f, "Transaction execution failed"),
            EvmError::ContractDeploymentFailed => write!(f, "Contract deployment failed"),
            EvmError::InsufficientFunds => write!(f, "Insufficient funds for operation"),
            EvmError::ContractCallFailed => write!(f, "Contract call failed"),
            EvmError::AbiError => write!(f, "ABI encoding/decoding error"),
            EvmError::RpcError => write!(f, "RPC communication error"),
            EvmError::GasEstimationFailed => write!(f, "Gas estimation failed"),
            EvmError::InvalidChainId => write!(f, "Invalid chain ID"),
            EvmError::SignerError => write!(f, "Signer operation failed"),
        }
    }
}

impl Context for EvmError {}

/// Account balance information for insufficient funds errors
#[derive(Debug, Clone)]
pub struct AccountBalance {
    pub address: String,
    pub balance: String,
    pub required: String,
}

impl fmt::Display for AccountBalance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Account {} has {} but needs {}", self.address, self.balance, self.required)
    }
}

/// Transaction details for debugging
#[derive(Debug, Clone)]
pub struct TransactionInfo {
    pub from: String,
    pub to: Option<String>,
    pub value: String,
    pub gas_limit: Option<String>,
    pub gas_price: Option<String>,
    pub nonce: Option<u64>,
}

impl fmt::Display for TransactionInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Transaction from {}", self.from)?;
        if let Some(to) = &self.to {
            write!(f, " to {}", to)?;
        }
        write!(f, " value: {}", self.value)?;
        if let Some(gas) = &self.gas_limit {
            write!(f, " gas: {}", gas)?;
        }
        Ok(())
    }
}

/// Contract information for deployment/interaction errors
#[derive(Debug, Clone)]
pub struct ContractInfo {
    pub name: String,
    pub address: Option<String>,
    pub method: Option<String>,
}

impl fmt::Display for ContractInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Contract: {}", self.name)?;
        if let Some(addr) = &self.address {
            write!(f, " at {}", addr)?;
        }
        if let Some(method) = &self.method {
            write!(f, " method: {}", method)?;
        }
        Ok(())
    }
}

/// Helper functions for EVM-specific error creation
pub trait EvmErrorExt {
    /// Attach account balance information
    fn with_balance_info(
        self,
        address: impl Into<String>,
        balance: impl Into<String>,
        required: impl Into<String>,
    ) -> Self;

    /// Attach transaction information
    fn with_transaction_info(self, tx: TransactionInfo) -> Self;

    /// Attach contract information
    fn with_contract_info(
        self,
        name: impl Into<String>,
        address: Option<String>,
        method: Option<String>,
    ) -> Self;
}

impl<T> EvmErrorExt for Result<T, Report<EvmError>> {
    fn with_balance_info(
        self,
        address: impl Into<String>,
        balance: impl Into<String>,
        required: impl Into<String>,
    ) -> Self {
        self.map_err(|e| {
            e.attach(AccountBalance {
                address: address.into(),
                balance: balance.into(),
                required: required.into(),
            })
        })
    }

    fn with_transaction_info(self, tx: TransactionInfo) -> Self {
        self.map_err(|e| e.attach(tx))
    }

    fn with_contract_info(
        self,
        name: impl Into<String>,
        address: Option<String>,
        method: Option<String>,
    ) -> Self {
        self.map_err(|e| e.attach(ContractInfo { name: name.into(), address, method }))
    }
}

/// Helper macro for creating EVM errors
#[macro_export]
macro_rules! evm_error {
    ($error:expr, $($arg:tt)*) => {{
        use $crate::errors::EvmError;
        error_stack::Report::new($error)
            .attach_printable(format!($($arg)*))
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use error_stack::ResultExt;
    use txtx_addon_kit::types::errors::ErrorDocumentation;

    #[test]
    fn test_evm_error_with_balance() {
        let error =
            Report::new(EvmError::InsufficientFunds).attach_printable("Cannot deploy contract");

        let result: Result<(), Report<EvmError>> = Err(error);
        let error = result
            .with_balance_info("0x742d35Cc6634C0532925a3b844Bc9e7595f89590", "0.5 ETH", "1.2 ETH")
            .unwrap_err();

        let balance = error.downcast_ref::<AccountBalance>().unwrap();
        assert_eq!(balance.balance, "0.5 ETH");
        assert_eq!(balance.required, "1.2 ETH");
    }

    #[test]
    fn test_evm_error_with_contract_info() {
        let error = evm_error!(EvmError::ContractCallFailed, "Method 'transfer' reverted");

        let result: Result<(), Report<EvmError>> = Err(error);
        let error = result
            .with_contract_info(
                "ERC20Token",
                Some("0x1234567890123456789012345678901234567890".to_string()),
                Some("transfer".to_string()),
            )
            .unwrap_err();

        let contract = error.downcast_ref::<ContractInfo>().unwrap();
        assert_eq!(contract.name, "ERC20Token");
        assert_eq!(contract.method.as_ref().unwrap(), "transfer");
    }

    #[test]
    fn test_error_chain_with_context() {
        // Simulate a nested error scenario
        fn parse_address(addr: &str) -> Result<String, Report<EvmError>> {
            if !addr.starts_with("0x") || addr.len() != 42 {
                return Err(Report::new(EvmError::InvalidAddress)
                    .attach_printable(format!("Invalid address format: {}", addr)));
            }
            Ok(addr.to_string())
        }

        fn deploy_contract(addr: &str) -> Result<String, Report<EvmError>> {
            parse_address(addr)
                .change_context(EvmError::ContractDeploymentFailed)
                .attach_printable("Failed to validate deployer address")?;

            // Simulate deployment failure
            Err(Report::new(EvmError::InsufficientFunds))
                .with_balance_info(addr, "0.1 ETH", "0.5 ETH")
        }

        let result = deploy_contract("invalid");
        assert!(result.is_err());

        let error = result.unwrap_err();
        // Should have the deployment context
        let error_string = format!("{:?}", error);
        assert!(error_string.contains("Contract deployment failed"));
        assert!(error_string.contains("Invalid address format"));
    }
}
