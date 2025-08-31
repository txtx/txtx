use super::types::FilledCommonTransactionFields;
use crate::constants::GAS_PRICE;
use crate::errors::{EvmError, EvmResult, CodecError};
use crate::rpc::EvmRpc;

use alloy::network::TransactionBuilder;
use alloy::primitives::TxKind;
use alloy::rpc::types::TransactionRequest;
use error_stack::{Report, ResultExt};
use txtx_addon_kit::types::stores::ValueStore;

#[deprecated(note = "Use build_unsigned_transaction_v2 instead")]
#[allow(dead_code)]
pub async fn build_unsigned_legacy_transaction(
    rpc: &EvmRpc,
    args: &ValueStore,
    fields: &FilledCommonTransactionFields,
) -> Result<TransactionRequest, String> {
    let gas_price = args.get_value(GAS_PRICE).map(|v| v.expect_uint()).transpose()?;

    let gas_price = match gas_price {
        Some(gas_price) => gas_price as u128,
        None => rpc.get_gas_price().await.map_err(|e| e.to_string())?,
    };
    let mut tx = TransactionRequest::default()
        .with_from(fields.from)
        .with_value(alloy::primitives::U256::from(fields.amount))
        .with_nonce(fields.nonce)
        .with_chain_id(fields.chain_id)
        .with_gas_price(gas_price);

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

pub async fn build_unsigned_legacy_transaction_v2(
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

    // Get gas price from args or RPC
    let gas_price = if let Some(price) = args.get_value(GAS_PRICE) {
        price.as_integer()
            .and_then(|i| if i >= 0 { Some(i as u128) } else { None })
            .ok_or_else(|| Report::new(EvmError::Codec(CodecError::InvalidType {
                expected: "u128".to_string(),
                received: format!("{:?}", price),
            })))
            .attach_printable("Converting gas price from configuration")?
    } else {
        rpc.get_gas_price()
            .await
            .attach_printable("Fetching current gas price from network")?
    };

    tx.gas_price = Some(gas_price);
    Ok(tx)
}