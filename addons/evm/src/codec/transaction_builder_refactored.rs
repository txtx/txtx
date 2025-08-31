//! Refactored transaction building module using error-stack
//! This demonstrates how error-stack provides better error context

use crate::errors::{
    EvmError, EvmResult, TransactionError, RpcError, CodecError,
    TransactionContext, RpcContext, IntoEvmError
};
use crate::commands::actions::get_expected_address;
use crate::constants::{GAS_PRICE, MAX_FEE_PER_GAS, MAX_PRIORITY_FEE_PER_GAS};
use crate::rpc::EvmRpc;
use crate::codec::{CommonTransactionFields, TransactionType};
use alloy::primitives::Address;
use alloy::rpc::types::TransactionRequest;
use error_stack::{Report, ResultExt};
use txtx_addon_kit::types::stores::ValueStore;

/// Build an unsigned transaction with rich error context
pub async fn build_unsigned_transaction_v2(
    rpc: EvmRpc,
    args: &ValueStore,
    fields: CommonTransactionFields,
) -> EvmResult<(TransactionRequest, i128, String)> {
    // Parse and validate the from address
    let from = get_expected_address(&fields.from)
        .attach_printable("Parsing 'from' address for transaction")?;
    
    // Parse and validate the to address if present
    let to = if let Some(to_value) = fields.to {
        Some(
            get_expected_address(&to_value)
                .attach_printable("Parsing 'to' address for transaction")?
        )
    } else {
        None
    };

    // Get nonce with RPC context
    let nonce = match fields.nonce {
        Some(nonce) => nonce,
        None => {
            rpc.get_nonce(&from)
                .await
                .map_err(|e| {
                    Report::new(EvmError::Rpc(RpcError::NodeError(e.to_string())))
                })
                .attach(RpcContext {
                    endpoint: rpc.get_endpoint(),
                    method: "eth_getTransactionCount".to_string(),
                    params: Some(format!("[\"{:?}\", \"pending\"]", from)),
                })
                .attach_printable(format!("Fetching nonce for address {}", from))?
        }
    };

    // Build transaction context for error reporting
    let tx_context = TransactionContext {
        tx_hash: None,
        from: Some(from),
        to,
        value: Some(fields.amount as u128),
        gas_limit: fields.gas_limit,
        chain_id: fields.chain_id,
    };

    // Build the appropriate transaction type
    let (tx_request, cost_estimate, cost_string) = match fields.tx_type {
        TransactionType::Legacy => {
            build_legacy_transaction_v2(
                rpc.clone(),
                args,
                from,
                to,
                nonce,
                fields.chain_id,
                fields.amount,
                fields.gas_limit,
                fields.input,
                fields.deploy_code,
            )
            .await
            .attach(tx_context.clone())
            .change_context(EvmError::Transaction(TransactionError::InvalidType(
                "Failed to build legacy transaction".to_string()
            )))?
        }
        TransactionType::EIP1559 => {
            build_eip1559_transaction_v2(
                rpc.clone(),
                args,
                from,
                to,
                nonce,
                fields.chain_id,
                fields.amount,
                fields.gas_limit,
                fields.input,
                fields.deploy_code,
            )
            .await
            .attach(tx_context.clone())
            .change_context(EvmError::Transaction(TransactionError::InvalidType(
                "Failed to build EIP-1559 transaction".to_string()
            )))?
        }
        TransactionType::EIP2930 | TransactionType::EIP4844 => {
            return Err(Report::new(EvmError::Transaction(
                TransactionError::InvalidType(format!("Transaction type {:?} not yet supported", fields.tx_type))
            )))
            .attach(tx_context);
        }
    };

    // Validate the transaction has sufficient funds
    validate_transaction_balance(&tx_request, cost_estimate, &rpc, &from)
        .await
        .attach(tx_context)?;

    Ok((tx_request, cost_estimate, cost_string))
}

/// Build a legacy transaction with error context
async fn build_legacy_transaction_v2(
    rpc: EvmRpc,
    args: &ValueStore,
    from: Address,
    to: Option<Address>,
    nonce: u64,
    chain_id: u64,
    amount: u64,
    gas_limit: Option<u64>,
    input: Option<Vec<u8>>,
    deploy_code: Option<Vec<u8>>,
) -> EvmResult<(TransactionRequest, i128, String)> {
    let mut tx = TransactionRequest::default()
        .from(from)
        .nonce(nonce)
        .chain_id(chain_id)
        .value(alloy::primitives::U256::from(amount));

    // Set recipient or deployment data
    if let Some(to_addr) = to {
        tx = tx.to(to_addr);
        if let Some(data) = input {
            tx = tx.input(data.into());
        }
    } else if let Some(code) = deploy_code {
        tx = tx.input(code.into());
    }

    // Get gas price from args or RPC
    let gas_price = if let Some(price) = args.get_value(GAS_PRICE) {
        price.try_into()
            .map_err(|_| Report::new(EvmError::Codec(CodecError::InvalidType {
                expected: "u128".to_string(),
                received: "unknown".to_string(),
            })))
            .attach_printable("Converting gas price from configuration")?
    } else {
        rpc.get_gas_price()
            .await
            .map_err(|e| Report::new(EvmError::Rpc(RpcError::NodeError(e.to_string()))))
            .attach(RpcContext {
                endpoint: rpc.get_endpoint(),
                method: "eth_gasPrice".to_string(),
                params: None,
            })?
    };

    tx = tx.gas_price(gas_price);

    // Estimate gas if not provided
    let gas_limit = match gas_limit {
        Some(limit) => limit,
        None => estimate_gas_limit(&rpc, &tx).await?
    };
    
    tx = tx.gas(gas_limit);

    // Calculate cost
    let cost = (gas_price * gas_limit as u128) + amount as i128;
    let cost_string = format_wei_to_ether(cost)?;

    Ok((tx, cost, cost_string))
}

/// Build an EIP-1559 transaction with error context
async fn build_eip1559_transaction_v2(
    rpc: EvmRpc,
    args: &ValueStore,
    from: Address,
    to: Option<Address>,
    nonce: u64,
    chain_id: u64,
    amount: u64,
    gas_limit: Option<u64>,
    input: Option<Vec<u8>>,
    deploy_code: Option<Vec<u8>>,
) -> EvmResult<(TransactionRequest, i128, String)> {
    let mut tx = TransactionRequest::default()
        .from(from)
        .nonce(nonce)
        .chain_id(chain_id)
        .value(alloy::primitives::U256::from(amount));

    // Set recipient or deployment data
    if let Some(to_addr) = to {
        tx = tx.to(to_addr);
        if let Some(data) = input {
            tx = tx.input(data.into());
        }
    } else if let Some(code) = deploy_code {
        tx = tx.input(code.into());
    }

    // Get fee parameters
    let (max_fee, max_priority_fee) = get_eip1559_fees(&rpc, args).await?;
    
    tx = tx
        .max_fee_per_gas(max_fee)
        .max_priority_fee_per_gas(max_priority_fee);

    // Estimate gas if not provided  
    let gas_limit = match gas_limit {
        Some(limit) => limit,
        None => estimate_gas_limit(&rpc, &tx).await?
    };
    
    tx = tx.gas(gas_limit);

    // Calculate cost (using max fee for worst case)
    let cost = (max_fee * gas_limit as u128) + amount as i128;
    let cost_string = format_wei_to_ether(cost)?;

    Ok((tx, cost, cost_string))
}

/// Helper to get EIP-1559 fee parameters
async fn get_eip1559_fees(
    rpc: &EvmRpc,
    args: &ValueStore,
) -> EvmResult<(u128, u128)> {
    // Get max fee per gas
    let max_fee = if let Some(fee) = args.get_value(MAX_FEE_PER_GAS) {
        fee.try_into()
            .map_err(|_| Report::new(EvmError::Codec(CodecError::InvalidType {
                expected: "u128".to_string(),
                received: "unknown".to_string(),
            })))
            .attach_printable("Converting max fee per gas")?
    } else {
        let base_fee = rpc.get_base_fee()
            .await
            .map_err(|e| Report::new(EvmError::Rpc(RpcError::NodeError(e.to_string()))))
            .attach_printable("Fetching current base fee")?;
        
        // Standard formula: base_fee * 2 + priority_fee
        base_fee * 2
    };

    // Get max priority fee
    let max_priority = if let Some(fee) = args.get_value(MAX_PRIORITY_FEE_PER_GAS) {
        fee.try_into()
            .map_err(|_| Report::new(EvmError::Codec(CodecError::InvalidType {
                expected: "u128".to_string(),
                received: "unknown".to_string(),
            })))
            .attach_printable("Converting max priority fee")?
    } else {
        rpc.get_max_priority_fee()
            .await
            .map_err(|e| Report::new(EvmError::Rpc(RpcError::NodeError(e.to_string()))))
            .attach_printable("Fetching suggested priority fee")?
    };

    Ok((max_fee, max_priority))
}

/// Estimate gas limit for a transaction
async fn estimate_gas_limit(
    rpc: &EvmRpc,
    tx: &TransactionRequest,
) -> EvmResult<u64> {
    rpc.estimate_gas(tx)
        .await
        .map_err(|e| Report::new(EvmError::Transaction(TransactionError::GasEstimationFailed)))
        .attach(RpcContext {
            endpoint: rpc.get_endpoint(),
            method: "eth_estimateGas".to_string(),
            params: Some(format!("{:?}", tx)),
        })
        .attach_printable("Estimating gas for transaction")
        .map(|gas| {
            // Add 10% buffer for safety
            gas * 110 / 100
        })
}

/// Validate transaction sender has sufficient balance
async fn validate_transaction_balance(
    tx: &TransactionRequest,
    cost: i128,
    rpc: &EvmRpc,
    from: &Address,
) -> EvmResult<()> {
    let balance = rpc.get_balance(from)
        .await
        .map_err(|e| Report::new(EvmError::Rpc(RpcError::NodeError(e.to_string()))))
        .attach(RpcContext {
            endpoint: rpc.get_endpoint(),
            method: "eth_getBalance".to_string(),
            params: Some(format!("[\"{:?}\", \"latest\"]", from)),
        })
        .attach_printable(format!("Checking balance for address {}", from))?;

    if balance < cost as u128 {
        return Err(Report::new(EvmError::Transaction(
            TransactionError::InsufficientFunds {
                required: cost as u128,
                available: balance,
            }
        )))
        .attach_printable(format!(
            "Account {} has insufficient funds. Required: {} wei, Available: {} wei",
            from, cost, balance
        ));
    }

    Ok(())
}

/// Format wei amount to ether string
fn format_wei_to_ether(wei: i128) -> EvmResult<String> {
    use alloy::primitives::utils::format_units;
    
    format_units(wei as u128, 18)
        .map_err(|e| Report::new(EvmError::Codec(
            CodecError::InvalidType {
                expected: "wei amount".to_string(),
                received: e.to_string(),
            }
        )))
        .attach_printable("Formatting wei to ether")
        .map(|s| format!("{} ETH", s))
}

// Extension trait to get RPC endpoint (mock for demonstration)
trait RpcExt {
    fn get_endpoint(&self) -> String;
    async fn get_base_fee(&self) -> Result<u128, String>;
    async fn get_max_priority_fee(&self) -> Result<u128, String>;
    async fn get_balance(&self, address: &Address) -> Result<u128, String>;
    async fn estimate_gas(&self, tx: &TransactionRequest) -> Result<u64, String>;
}

impl RpcExt for EvmRpc {
    fn get_endpoint(&self) -> String {
        // This would be implemented properly in the actual RPC module
        "http://localhost:8545".to_string()
    }
    
    async fn get_base_fee(&self) -> Result<u128, String> {
        // Placeholder - would call actual RPC
        Ok(20_000_000_000) // 20 gwei
    }
    
    async fn get_max_priority_fee(&self) -> Result<u128, String> {
        // Placeholder - would call actual RPC
        Ok(2_000_000_000) // 2 gwei
    }
    
    async fn get_balance(&self, _address: &Address) -> Result<u128, String> {
        // Placeholder - would call actual RPC
        Ok(1_000_000_000_000_000_000) // 1 ETH
    }
    
    async fn estimate_gas(&self, _tx: &TransactionRequest) -> Result<u64, String> {
        // Placeholder - would call actual RPC
        Ok(21000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use txtx_addon_kit::types::types::Value;

    #[tokio::test]
    async fn test_transaction_with_insufficient_funds() {
        // This test demonstrates how error-stack provides rich context
        // when a transaction fails due to insufficient funds
        
        let rpc = EvmRpc::new("http://localhost:8545".to_string(), None);
        let mut args = ValueStore::new();
        
        let fields = CommonTransactionFields {
            to: Some(Value::string("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb")),
            from: Value::string("0x0000000000000000000000000000000000000001"),
            nonce: Some(0),
            chain_id: 1,
            amount: 10_000_000_000_000_000_000, // 10 ETH (more than balance)
            gas_limit: Some(21000),
            input: None,
            tx_type: TransactionType::EIP1559,
            deploy_code: None,
        };

        let result = build_unsigned_transaction_v2(rpc, &args, fields).await;
        
        assert!(result.is_err());
        
        // The error would contain rich context:
        // - Root cause: InsufficientFunds
        // - RPC context: balance check details
        // - Transaction context: from, to, amount, etc.
        // - Attachments: human-readable messages at each level
    }
}
