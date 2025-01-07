use std::sync::Arc;

use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::commitment_config::{CommitmentConfig, CommitmentLevel};
use solana_sdk::transaction::Transaction;
use txtx_addon_kit::channel;
use txtx_addon_kit::constants::SIGNED_TRANSACTION_BYTES;
use txtx_addon_kit::types::commands::CommandExecutionResult;
use txtx_addon_kit::types::commands::{
    CommandExecutionFutureResult, CommandImplementation, CommandSpecification,
    PreCommandSpecification,
};
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::types::frontend::{
    Actions, BlockEvent, ProgressBarStatus, ProgressBarStatusColor, StatusUpdater,
};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::{RunbookSupervisionContext, Type, Value};
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::uuid::Uuid;

use crate::constants::{COMMITMENT_LEVEL, RPC_API_URL, SIGNATURE};
use crate::typing::SVM_INSTRUCTION;

lazy_static! {
    pub static ref SEND_TRANSACTION: PreCommandSpecification = define_command! {
        SendTransaction => {
            name: "Send SVM Transaction",
            matcher: "send_transaction",
            documentation: "The `svm::send_transaction` action encodes a transaction, signs the transaction using an in-browser signer, and broadcasts the signed transaction to the network.",
            implements_signing_capability: false,
            implements_background_task_capability: true,
            inputs: [
                description: {
                    documentation: "Description of the transaction",
                    typing: Type::string(),
                    optional: true,
                    tainting: false,
                    internal: false
                },
                instructions: {
                    documentation: "The address and identifier of the contract to invoke.",
                    typing: Type::array(Type::addon(SVM_INSTRUCTION.into())),
                    optional: false,
                    tainting: true,
                    internal: false
                },
                rpc_api_url: {
                    documentation: "The URL to use when making API requests.",
                    typing: Type::string(),
                    optional: true,
                    tainting: false,
                    internal: false
                },
                rpc_api_auth_token: {
                    documentation: "The HTTP authentication token to include in the headers when making API requests.",
                    typing: Type::string(),
                    optional: true,
                    tainting: false,
                    internal: false,
                    sensitive: true
                },
                commitment_level: {
                    documentation: "The commitment level expected for considering this action as done ('processed', 'confirmed', 'finalized'). Default to 'confirmed'.",
                    typing: Type::string(),
                    optional: true,
                    tainting: false,
                    internal: false
                }
            ],
            outputs: [
                signature: {
                    documentation: "The transaction computed signature.",
                    typing: Type::string()
                }
            ],
            example: txtx_addon_kit::indoc! {r#"
            // Coming soon
            "#},
      }
    };
}

pub struct SendTransaction;
impl CommandImplementation for SendTransaction {
    fn check_instantiability(
        _ctx: &CommandSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn check_executability(
        _construct_id: &ConstructDid,
        _instance_name: &str,
        _spec: &CommandSpecification,
        _values: &ValueStore,
        _supervision_context: &RunbookSupervisionContext,
    ) -> Result<Actions, Diagnostic> {
        Ok(Actions::none())
    }

    #[cfg(not(feature = "wasm"))]
    fn run_execution(
        _construct_id: &ConstructDid,
        _spec: &CommandSpecification,
        _values: &ValueStore,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
    ) -> CommandExecutionFutureResult {
        let future = async move {
            let result = CommandExecutionResult::new();
            Ok(result)
        };

        Ok(Box::pin(future))
    }

    fn build_background_task(
        construct_did: &ConstructDid,
        _spec: &CommandSpecification,
        inputs: &ValueStore,
        outputs: &ValueStore,
        progress_tx: &channel::Sender<BlockEvent>,
        background_tasks_uuid: &Uuid,
        _supervision_context: &RunbookSupervisionContext,
    ) -> CommandExecutionFutureResult {
        let rpc_api_url = inputs.get_expected_string(RPC_API_URL).unwrap().to_string();
        let commitment_level =
            inputs.get_expected_string(COMMITMENT_LEVEL).unwrap_or("confirmed").to_string();

        let construct_did = construct_did.clone();
        let outputs = outputs.clone();
        let progress_tx = progress_tx.clone();
        let background_tasks_uuid = background_tasks_uuid.clone();

        let future = async move {
            let signed_transaction_value = outputs.get_value(SIGNED_TRANSACTION_BYTES).unwrap();
            let commitment_config = CommitmentConfig {
                commitment: match commitment_level.as_str() {
                    "processed" => CommitmentLevel::Processed,
                    "confirmed" => CommitmentLevel::Confirmed,
                    "finalized" => CommitmentLevel::Finalized,
                    _ => CommitmentLevel::Processed,
                },
            };
            let client =
                Arc::new(RpcClient::new_with_commitment(rpc_api_url.clone(), commitment_config));

            // let mut config = RpcSendTransactionConfig::default();
            // config.preflight_commitment = match commitment_level.as_str() {
            //     "processed" => Some(CommitmentLevel::Processed),
            //     "confirmed" => Some(CommitmentLevel::Confirmed),
            //     "finalized" => Some(CommitmentLevel::Finalized),
            //     _ => Some(CommitmentLevel::Confirmed),
            // };
            let mut status_updater =
                StatusUpdater::new(&background_tasks_uuid, &construct_did, &progress_tx);

            let mut result = CommandExecutionResult::new();

            let transaction_bytes = signed_transaction_value
                .expect_buffer_bytes_result()
                .map_err(|e| diagnosed_error!("{}", e))?;
            let signature =
                send_transaction(client.clone(), true, &transaction_bytes).map_err(|diag| {
                    status_updater.propagate_status(ProgressBarStatus::new_err(
                        "Failed",
                        "Failed to broadcast transaction",
                        &diag,
                    ));
                    diag
                })?;
            result.outputs.insert(SIGNATURE.into(), Value::string(signature.clone()));

            status_updater.propagate_status(ProgressBarStatus::new_msg(
                ProgressBarStatusColor::Green,
                "Complete",
                &format!("Transaction {} broadcasting complete", signature),
            ));
            Ok(result)
        };

        Ok(Box::pin(future))
    }
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
