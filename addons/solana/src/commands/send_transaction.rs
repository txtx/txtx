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
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::{RunbookSupervisionContext, Type, Value};
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::uuid::Uuid;

use crate::constants::RPC_API_URL;
use crate::typing::SOLANA_INSTRUCTION;

lazy_static! {
    pub static ref SEND_TRANSACTION: PreCommandSpecification = define_command! {
        SendTransaction => {
            name: "Send Solana Transaction",
            matcher: "send_transaction",
            documentation: "The `send_transaction` action encodes a transaction, signs the transaction using an in-browser signer, and broadcasts the signed transaction to the network.",
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
                    typing: Type::array(Type::addon(SOLANA_INSTRUCTION.into())),
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
                    internal: false
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
        spec: &CommandSpecification,
        inputs: &ValueStore,
        outputs: &ValueStore,
        progress_tx: &channel::Sender<BlockEvent>,
        background_tasks_uuid: &Uuid,
        supervision_context: &RunbookSupervisionContext,
    ) -> CommandExecutionFutureResult {
        let rpc_api_url = inputs.get_expected_string(RPC_API_URL).unwrap().to_string();
        let commitment_level = inputs.get_expected_string("commitment_level").unwrap_or("confirmed").to_string();
        let transaction_bytes =
            outputs.get_expected_buffer_bytes(SIGNED_TRANSACTION_BYTES).unwrap();
        let transaction: Transaction = bincode::deserialize(&transaction_bytes).unwrap();

        let future = async move {
            let client = RpcClient::new(rpc_api_url);

            let mut config = RpcSendTransactionConfig::default();
            config.preflight_commitment = match commitment_level.as_str() {
                "processed" => Some(CommitmentLevel::Processed),
                "confirmed" => Some(CommitmentLevel::Confirmed),
                "finalized" => Some(CommitmentLevel::Finalized),
                _ => None
            };

            let res = match client.send_transaction_with_config(&transaction, config) {
                Ok(res) => res,
                Err(e) => {
                    return Err(diagnosed_error!("unable to send transaction ({})", e.to_string()))
                }
            };

            let mut result = CommandExecutionResult::new();
            result.outputs.insert("signature".into(), Value::string(res.to_string()));
            Ok(result)
        };

        Ok(Box::pin(future))
    }
}
