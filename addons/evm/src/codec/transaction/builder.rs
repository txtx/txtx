use super::types::{CommonTransactionFields, FilledCommonTransactionFields, TransactionType};
use super::legacy::build_unsigned_legacy_transaction_v2;
use super::eip1559::build_unsigned_eip1559_transaction_v2;
use super::cost::set_gas_limit_v2;

use crate::commands::actions::get_expected_address;
use crate::errors::{EvmError, EvmResult, TransactionError, CodecError, TransactionContext};
use crate::rpc::EvmRpc;

use alloy::network::TransactionBuilder;
use alloy::rpc::types::TransactionRequest;
use error_stack::{Report, ResultExt};
use txtx_addon_kit::types::stores::ValueStore;

// New error-stack version
pub async fn build_unsigned_transaction_v2(
    rpc: EvmRpc,
    args: &ValueStore,
    fields: CommonTransactionFields,
) -> EvmResult<(TransactionRequest, i128, String)> {
    // Parse and validate the from address
    let from = get_expected_address(&fields.from)
        .attach_printable("Parsing 'from' address for transaction")?;
    
    // Parse and validate the to address if present
    let to = if let Some(to_value) = fields.to.clone() {
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

    let filled_fields = FilledCommonTransactionFields {
        to,
        from,
        nonce,
        chain_id: fields.chain_id,
        amount: fields.amount,
        gas_limit: fields.gas_limit,
        input: fields.input.clone(),
        deploy_code: fields.deploy_code.clone(),
    };

    let mut tx = match fields.tx_type {
        TransactionType::Legacy => {
            build_unsigned_legacy_transaction_v2(&rpc, args, &filled_fields)
                .await
                .attach(tx_context.clone())
                .change_context(EvmError::Transaction(TransactionError::InvalidType(
                    "Failed to build legacy transaction".to_string()
                )))?
        }
        TransactionType::EIP2930 => {
            println!("Unsupported tx type EIP2930 was used. Defaulting to EIP1559 tx");
            build_unsigned_eip1559_transaction_v2(&rpc, args, &filled_fields)
                .await
                .attach(tx_context.clone())
                .change_context(EvmError::Transaction(TransactionError::InvalidType(
                    "Failed to build EIP-2930 transaction".to_string()
                )))?
        }
        TransactionType::EIP1559 => {
            build_unsigned_eip1559_transaction_v2(&rpc, args, &filled_fields)
                .await
                .attach(tx_context.clone())
                .change_context(EvmError::Transaction(TransactionError::InvalidType(
                    "Failed to build EIP-1559 transaction".to_string()
                )))?
        }
        TransactionType::EIP4844 => {
            return Err(Report::new(EvmError::Transaction(
                TransactionError::InvalidType(format!("Transaction type EIP-4844 not yet supported"))
            )))
            .attach(tx_context);
        }
    };

    // set gas limit _after_ all other fields have been set to get an accurate estimate
    tx = set_gas_limit_v2(&rpc, tx, fields.gas_limit)
        .await
        .attach(tx_context.clone())?;

    let typed_transaction = tx.clone()
        .build_unsigned()
        .map_err(|e| Report::new(EvmError::Transaction(TransactionError::InvalidType(
            format!("Failed to build transaction: {}", e)
        ))))
        .attach(tx_context)?;
    
    let cost = super::cost::get_transaction_cost_v2(&typed_transaction, &rpc).await?;
    
    Ok((tx, cost.0, cost.1))
}

// Keep old version for compatibility
#[deprecated(note = "Use build_unsigned_transaction_v2 for better error handling")]
#[allow(dead_code)]
pub async fn build_unsigned_transaction(
    rpc: EvmRpc,
    args: &ValueStore,
    fields: CommonTransactionFields,
) -> Result<(TransactionRequest, i128, String), String> {
    // Use new version internally and convert error
    let (tx, cost, _cost_string) = build_unsigned_transaction_v2(rpc.clone(), args, fields)
        .await
        .map_err(|e| e.to_string())?;

    // Try to simulate the transaction, but provide a valid empty result on failure
    let sim = match rpc.call(&tx, false).await {
        Ok(result) => result,
        Err(e) => {
            // Log the error but return valid empty hex
            eprintln!("Warning: Transaction simulation failed: {}", e);
            "0x00".into() // Return valid hex that represents empty/zero result
        }
    };
    Ok((tx, cost, sim))
}