use crate::errors::{EvmError, EvmResult, TransactionError, CodecError};
use crate::rpc::EvmRpc;

use alloy::consensus::{Transaction, TypedTransaction};
use alloy::primitives::utils::format_units;
use alloy::rpc::types::TransactionRequest;
use error_stack::{Report, ResultExt};

#[deprecated(note = "Use set_gas_limit_v2 for better error handling")]
#[allow(dead_code)]
pub async fn set_gas_limit(
    rpc: &EvmRpc,
    mut tx: TransactionRequest,
    gas_limit: Option<u64>,
) -> Result<TransactionRequest, String> {
    if let Some(gas_limit) = gas_limit {
        tx.gas = Some(gas_limit.into());
    } else {
        let call_res = rpc.call(&tx, false).await;

        let gas_limit = rpc.estimate_gas(&tx).await.map_err(|estimate_err| match call_res {
            Ok(res) => format!(
                "failed to estimate gas: {};\nsimulation results: {}",
                estimate_err.to_string(),
                res
            ),
            Err(e) => format!(
                "failed to estimate gas: {};\nfailed to simulate transaction: {}",
                estimate_err.to_string(),
                e.to_string()
            ),
        })?;
        tx.gas = Some(gas_limit.into());
    }
    Ok(tx)
}

pub async fn set_gas_limit_v2(
    rpc: &EvmRpc,
    mut tx: TransactionRequest,
    gas_limit: Option<u64>,
) -> EvmResult<TransactionRequest> {
    if let Some(gas_limit) = gas_limit {
        tx.gas = Some(gas_limit.into());
    } else {
        let call_res = rpc.call(&tx, false).await;

        let gas_limit = rpc.estimate_gas(&tx)
            .await
            .map_err(|estimate_err| match call_res {
                Ok(res) => {
                    estimate_err
                        .attach_printable(format!("Simulation result: {}", res))
                }
                Err(e) => {
                    estimate_err
                        .attach_printable(format!("Failed to simulate transaction: {}", e))
                }
            })
            .attach_printable("Gas estimation failed")?;
        
        // Add 10% buffer for safety
        let buffered_gas = gas_limit.saturating_mul(110).saturating_div(100);
        tx.gas = Some(buffered_gas.into());
    }
    Ok(tx)
}

#[deprecated(note = "Use get_transaction_cost_v2 for better error handling")]
#[allow(dead_code)]
pub async fn get_transaction_cost(
    transaction: &TypedTransaction,
    rpc: &EvmRpc,
) -> Result<i128, String> {
    let effective_gas_price = match &transaction {
        TypedTransaction::Legacy(tx) => tx.gas_price,
        TypedTransaction::Eip2930(tx) => tx.gas_price,
        TypedTransaction::Eip1559(tx) => {
            let base_fee = rpc.get_base_fee_per_gas().await.map_err(|e| e.to_string())?;
            tx.effective_gas_price(Some(base_fee as u64))
        }
        TypedTransaction::Eip4844(_tx) => unimplemented!("EIP-4844 is not supported"),
        TypedTransaction::Eip7702(_tx) => unimplemented!("EIP-7702 is not supported"),
    };
    let gas_limit = transaction.gas_limit();
    let cost: i128 = effective_gas_price as i128 * gas_limit as i128;
    Ok(cost)
}

pub async fn get_transaction_cost_v2(
    typed_transaction: &TypedTransaction,
    rpc: &EvmRpc,
) -> EvmResult<(i128, String)> {
    let effective_gas_price = match typed_transaction {
        TypedTransaction::Legacy(tx) => tx.gas_price,
        TypedTransaction::Eip2930(tx) => tx.gas_price,
        TypedTransaction::Eip1559(tx) => {
            let base_fee = rpc.get_base_fee_per_gas()
                .await
                .attach_printable("Fetching base fee for cost calculation")?;
            tx.effective_gas_price(Some(base_fee as u64))
        }
        TypedTransaction::Eip4844(_) => {
            return Err(Report::new(EvmError::Transaction(
                TransactionError::InvalidType("EIP-4844 not supported".to_string())
            )))
        }
        TypedTransaction::Eip7702(_) => {
            return Err(Report::new(EvmError::Transaction(
                TransactionError::InvalidType("EIP-7702 not supported".to_string())
            )))
        }
    };
    
    let gas_limit = typed_transaction.gas_limit();
    let amount = typed_transaction.value();
    let gas_cost = (effective_gas_price as i128) * (gas_limit as i128);
    let total_cost = gas_cost + amount.to::<i128>();
    
    let cost_string = format_units(total_cost as u128, 18)
        .map_err(|e| Report::new(EvmError::Codec(CodecError::InvalidType {
            expected: "wei amount".to_string(),
            received: e.to_string(),
        })))
        .attach_printable("Formatting transaction cost")?;
    
    Ok((total_cost, format!("{} ETH", cost_string)))
}

#[deprecated(note = "Use format_transaction_cost for better error handling")]
pub fn format_transaction_cost(cost: i128) -> EvmResult<String> {
    format_units(cost, "wei")
        .map_err(|e| Report::new(EvmError::Codec(CodecError::InvalidType {
            expected: "valid cost value".to_string(),
            received: format!("{}: {}", cost, e),
        })))
        .attach_printable(format!("Formatting transaction cost: {} wei", cost))
}