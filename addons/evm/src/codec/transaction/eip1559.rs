use super::types::FilledCommonTransactionFields;
use crate::constants::{MAX_FEE_PER_GAS, MAX_PRIORITY_FEE_PER_GAS};
use crate::errors::{EvmError, EvmResult, CodecError};
use crate::rpc::EvmRpc;

use alloy::network::TransactionBuilder;
use alloy::primitives::TxKind;
use alloy::rpc::types::TransactionRequest;
use error_stack::{Report, ResultExt};
use txtx_addon_kit::types::stores::ValueStore;

#[deprecated(note = "Use build_unsigned_transaction_v2 instead")]
#[allow(dead_code)]
pub async fn build_unsigned_eip1559_transaction(
    rpc: &EvmRpc,
    args: &ValueStore,
    fields: &FilledCommonTransactionFields,
) -> Result<TransactionRequest, String> {
    let max_fee_per_gas = args.get_value(MAX_FEE_PER_GAS).map(|v| v.expect_uint()).transpose()?;
    let max_priority_fee_per_gas =
        args.get_value(MAX_PRIORITY_FEE_PER_GAS).map(|v| v.expect_uint()).transpose()?;

    let (max_fee_per_gas, max_priority_fee_per_gas) =
        if max_fee_per_gas.is_none() || max_priority_fee_per_gas.is_none() {
            let fees = rpc.estimate_eip1559_fees().await.map_err(|e| e.to_string())?;

            (
                max_fee_per_gas.and_then(|f| Some(f as u128)).unwrap_or(fees.max_fee_per_gas),
                max_priority_fee_per_gas
                    .and_then(|f| Some(f as u128))
                    .unwrap_or(fees.max_priority_fee_per_gas),
            )
        } else {
            (max_fee_per_gas.unwrap() as u128, max_priority_fee_per_gas.unwrap() as u128)
        };

    let mut tx = TransactionRequest::default()
        .with_from(fields.from)
        .with_value(alloy::primitives::U256::from(fields.amount))
        .with_nonce(fields.nonce)
        .with_chain_id(fields.chain_id)
        .max_fee_per_gas(max_fee_per_gas)
        .with_max_priority_fee_per_gas(max_priority_fee_per_gas);

    if let Some(to) = fields.to {
        tx = tx.with_to(to);
    }
    if let Some(input) = &fields.input {
        tx = tx.with_input(input.clone());
    }
    if let Some(code) = &fields.deploy_code {
        tx = tx.with_deploy_code(code.clone()).with_kind(TxKind::Create);
    }

    Ok(tx)
}

pub async fn build_unsigned_eip1559_transaction_v2(
    rpc: &EvmRpc,
    args: &ValueStore,
    fields: &FilledCommonTransactionFields,
) -> EvmResult<TransactionRequest> {
    let mut tx = TransactionRequest::default()
        .from(fields.from)
        .nonce(fields.nonce)
        .with_chain_id(fields.chain_id)
        .value(alloy::primitives::U256::from(fields.amount));

    // Set recipient or deployment data
    if let Some(to_addr) = fields.to {
        tx = tx.to(to_addr);
        if let Some(data) = &fields.input {
            tx = tx.input(data.clone().into());
        }
    } else if let Some(code) = &fields.deploy_code {
        tx = tx.input(code.clone().into());
    }

    // Get fee parameters
    let max_fee = if let Some(fee) = args.get_value(MAX_FEE_PER_GAS) {
        fee.as_integer()
            .and_then(|i| if i >= 0 { Some(i as u128) } else { None })
            .ok_or_else(|| Report::new(EvmError::Codec(CodecError::InvalidType {
                expected: "u128".to_string(),
                received: format!("{:?}", fee),
            })))
            .attach_printable("Converting max fee per gas")?
    } else {
        let base_fee = rpc.get_base_fee_per_gas()
            .await
            .attach_printable("Fetching current base fee")?;
        // Standard formula: base_fee * 2 + priority_fee
        base_fee * 2
    };

    let max_priority = if let Some(fee) = args.get_value(MAX_PRIORITY_FEE_PER_GAS) {
        fee.as_integer()
            .and_then(|i| if i >= 0 { Some(i as u128) } else { None })
            .ok_or_else(|| Report::new(EvmError::Codec(CodecError::InvalidType {
                expected: "u128".to_string(),
                received: format!("{:?}", fee),
            })))
            .attach_printable("Converting max priority fee")?
    } else {
        // Default priority fee
        2_000_000_000 // 2 gwei
    };

    tx = tx
        .max_fee_per_gas(max_fee)
        .max_priority_fee_per_gas(max_priority);

    Ok(tx)
}