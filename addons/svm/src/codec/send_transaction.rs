use std::sync::Arc;

use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::commitment_config::{CommitmentConfig, CommitmentLevel};
use solana_sdk::transaction::Transaction;
use txtx_addon_kit::channel;
use txtx_addon_kit::constants::SIGNED_TRANSACTION_BYTES;
use txtx_addon_kit::types::commands::CommandExecutionResult;
use txtx_addon_kit::types::commands::{CommandExecutionFutureResult, CommandSpecification};
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::types::frontend::{BlockEvent, ProgressBarStatus, StatusUpdater};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::{RunbookSupervisionContext, Value};
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::uuid::Uuid;

use crate::constants::{
    COMMITMENT_LEVEL, DO_AWAIT_CONFIRMATION, IS_DEPLOYMENT, RPC_API_URL, SIGNATURE,
};

pub fn send_transaction_background_task(
    construct_did: &ConstructDid,
    _spec: &CommandSpecification,
    inputs: &ValueStore,
    outputs: &ValueStore,
    progress_tx: &channel::Sender<BlockEvent>,
    background_tasks_uuid: &Uuid,
    _supervision_context: &RunbookSupervisionContext,
) -> CommandExecutionFutureResult {
    let construct_did = construct_did.clone();
    let outputs = outputs.clone();
    let inputs = inputs.clone();
    let progress_tx = progress_tx.clone();
    let background_tasks_uuid = background_tasks_uuid.clone();

    let future = async move {
        let rpc_api_url = inputs.get_expected_string(RPC_API_URL).unwrap().to_string();
        let commitment_level = inputs.get_expected_string(COMMITMENT_LEVEL).unwrap_or("confirmed");
        let do_await_confirmation = inputs.get_bool(DO_AWAIT_CONFIRMATION).unwrap_or(true);
        let is_deployment = inputs.get_bool(IS_DEPLOYMENT).unwrap_or(false);

        let signed_transaction_value = if is_deployment {
            inputs.get_value(SIGNED_TRANSACTION_BYTES).unwrap()
        } else {
            outputs.get_value(SIGNED_TRANSACTION_BYTES).unwrap()
        };

        let commitment_config = CommitmentConfig {
            commitment: match commitment_level {
                "processed" => CommitmentLevel::Processed,
                "confirmed" => CommitmentLevel::Confirmed,
                "finalized" => CommitmentLevel::Finalized,
                _ => CommitmentLevel::Processed,
            },
        };

        let client =
            Arc::new(RpcClient::new_with_commitment(rpc_api_url.clone(), commitment_config));

        let mut status_updater =
            StatusUpdater::new(&background_tasks_uuid, &construct_did, &progress_tx);

        let mut result = CommandExecutionResult::from_value_store(&outputs);

        let transaction_bytes = signed_transaction_value
            .expect_buffer_bytes_result()
            .map_err(|e| diagnosed_error!("{}", e))?;
        let signature = send_transaction(client.clone(), do_await_confirmation, &transaction_bytes)
            .map_err(|diag| {
                status_updater.propagate_status(ProgressBarStatus::new_err(
                    "Failed",
                    "Failed to broadcast transaction",
                    &diag,
                ));
                diag
            })?;
        result.outputs.insert(SIGNATURE.into(), Value::string(signature.clone()));

        Ok(result)
    };

    Ok(Box::pin(future))
}

pub fn send_transaction(
    rpc_client: Arc<RpcClient>,
    // rpc_config: &RpcSendTransactionConfig,
    do_await_confirmation: bool,
    transaction_bytes: &Vec<u8>,
) -> Result<String, Diagnostic> {
    let transaction: Transaction = serde_json::from_slice(&transaction_bytes).map_err(|e| {
        diagnosed_error!("unable to deserialize transaction from bytes ({})", e.to_string())
    })?;

    let signature = if do_await_confirmation {
        rpc_client.send_and_confirm_transaction(&transaction).map_err(|e| {
            diagnosed_error!("unable to send and confirm transaction ({})", e.to_string())
        })?
    } else {
        rpc_client
            .send_transaction_with_config(
                &transaction,
                RpcSendTransactionConfig {
                    skip_preflight: true,
                    preflight_commitment: None,
                    encoding: None,
                    max_retries: None,
                    min_context_slot: None,
                },
            )
            .map_err(|e| diagnosed_error!("unable to send transaction ({})", e.to_string()))?
    };

    Ok(signature.to_string())
}
