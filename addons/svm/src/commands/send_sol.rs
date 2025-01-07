use std::collections::HashMap;
use std::str::FromStr;

use solana_client::rpc_client::RpcClient;
use solana_sdk::message::Message;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction;
use txtx_addon_kit::channel;
use txtx_addon_kit::constants::SIGNED_TRANSACTION_BYTES;
use txtx_addon_kit::helpers::build_diag_context_fn;
use txtx_addon_kit::types::commands::{
    CommandExecutionFutureResult, CommandImplementation, CommandSpecification,
    PreCommandSpecification,
};
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::types::frontend::BlockEvent;
use txtx_addon_kit::types::signers::{
    SignerActionsFutureResult, SignerInstance, SignerSignFutureResult, SignersState,
};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::{RunbookSupervisionContext, Type};
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::uuid::Uuid;

use crate::commands::send_transaction::SendTransaction;
use crate::constants::{
    AMOUNT, CHECKED_PUBLIC_KEY, NAMESPACE, RECIPIENT, RPC_API_URL, TRANSACTION_BYTES,
};
use crate::typing::SvmValue;

use super::get_signer_did;
use super::sign_transaction::SignTransaction;

lazy_static! {
    pub static ref SEND_SOL: PreCommandSpecification = define_command! {
        SendSol => {
            name: "Send SOL",
            matcher: "send_sol",
            documentation: "The `svm::send_sol` action encodes a transaction which sends SOL, signs it, and broadcasts it to the network.",
            implements_signing_capability: true,
            implements_background_task_capability: true,
            inputs: [
                description: {
                    documentation: "A description of the transaction.",
                    typing: Type::string(),
                    optional: true,
                    tainting: false,
                    internal: false
                },
                amount: {
                    documentation: "The amount, in lamports, to send.",
                    typing: Type::integer(),
                    optional: false,
                    tainting: false,
                    internal: false
                },
                recipient: {
                    documentation: "The address of the recipient.",
                    typing: Type::string(),
                    optional: false,
                    tainting: true,
                    internal: false
                },
                signer: {
                    documentation: "A reference to a signer construct, which will be used to sign the transaction.",
                    typing: Type::array(Type::string()),
                    optional: false,
                    tainting: true,
                    internal: false
                },
                commitment_level: {
                    documentation: "The commitment level expected for considering this action as done ('processed', 'confirmed', 'finalized'). The default is 'confirmed'.",
                    typing: Type::string(),
                    optional: true,
                    tainting: false,
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
                }
            ],
            outputs: [
                signature: {
                    documentation: "The transaction computed signature.",
                    typing: Type::string()
                }
            ],
            example: txtx_addon_kit::indoc! {
                r#"action "send_sol" "svm::send_sol" {
                    description = "Send some SOL"
                    amount = evm::sol_to_lamports(1)
                    signers = [signer.caller]
                    recipient = "zbBjhHwuqyKMmz8ber5oUtJJ3ZV4B6ePmANfGyKzVGV"
                }"#
            },
      }
    };
}

pub struct SendSol;
impl CommandImplementation for SendSol {
    fn check_instantiability(
        _ctx: &CommandSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    fn check_signed_executability(
        construct_did: &ConstructDid,
        instance_name: &str,
        spec: &CommandSpecification,
        args: &ValueStore,
        supervision_context: &RunbookSupervisionContext,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        signers: SignersState,
    ) -> SignerActionsFutureResult {
        let to_diag_with_ctx = build_diag_context_fn(
            instance_name.to_string(),
            format!("{}::{}", NAMESPACE, spec.matcher),
        );

        let signer_did = get_signer_did(args).unwrap();
        let signer_state = signers.get_signer_state(&signer_did).unwrap();

        let amount = args
            .get_expected_uint(AMOUNT)
            .map_err(|e| (signers.clone(), signer_state.clone(), to_diag_with_ctx(e.message)))?;

        let recipient =
            Pubkey::from_str(args.get_expected_string(RECIPIENT).map_err(|e| {
                (signers.clone(), signer_state.clone(), to_diag_with_ctx(e.message))
            })?)
            .map_err(|e| {
                (
                    signers.clone(),
                    signer_state.clone(),
                    to_diag_with_ctx(format!("invalid recipient: {}", e.to_string())),
                )
            })?;

        let rpc_api_url = args
            .get_expected_string(RPC_API_URL)
            .map_err(|e| (signers.clone(), signer_state.clone(), to_diag_with_ctx(e.message)))?
            .to_string();

        let signer_pubkey =
            Pubkey::from_str(signer_state.get_expected_string(CHECKED_PUBLIC_KEY).map_err(
                |e| (signers.clone(), signer_state.clone(), to_diag_with_ctx(e.to_string())),
            )?)
            .map_err(|e| {
                (
                    signers.clone(),
                    signer_state.clone(),
                    to_diag_with_ctx(format!("invalid signer pubkey: {}", e.to_string())),
                )
            })?;

        let instruction =
            solana_sdk::system_instruction::transfer(&signer_pubkey, &recipient, amount);

        let mut message = Message::new(&vec![instruction], None);
        let client = RpcClient::new(rpc_api_url);
        message.recent_blockhash = client.get_latest_blockhash().map_err(|e| {
            (
                signers.clone(),
                signer_state.clone(),
                to_diag_with_ctx(format!("failed to retrieve latest blockhash: {}", e.to_string())),
            )
        })?;
        let transaction = Transaction::new_unsigned(message);

        let transaction_bytes = serde_json::to_vec(&transaction).unwrap();

        let mut args = args.clone();
        args.insert(TRANSACTION_BYTES, SvmValue::message(transaction_bytes));

        SignTransaction::check_signed_executability(
            construct_did,
            instance_name,
            spec,
            &args,
            supervision_context,
            signers_instances,
            signers,
        )
    }

    fn run_signed_execution(
        construct_did: &ConstructDid,
        spec: &CommandSpecification,
        args: &ValueStore,
        progress_tx: &channel::Sender<BlockEvent>,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        signers: SignersState,
    ) -> SignerSignFutureResult {
        let progress_tx = progress_tx.clone();
        let args = args.clone();
        let signers_instances = signers_instances.clone();
        let construct_did = construct_did.clone();
        let spec = spec.clone();
        let progress_tx = progress_tx.clone();

        let mut args = args.clone();
        let future = async move {
            let run_signing_future = SignTransaction::run_signed_execution(
                &construct_did,
                &spec,
                &args,
                &progress_tx,
                &signers_instances,
                signers,
            );
            let (signers, signer_state, res_signing) = match run_signing_future {
                Ok(future) => match future.await {
                    Ok(res) => res,
                    Err(err) => return Err(err),
                },
                Err(err) => return Err(err),
            };

            let transaction_bytes = res_signing.outputs.get(SIGNED_TRANSACTION_BYTES).unwrap();
            args.insert(SIGNED_TRANSACTION_BYTES, transaction_bytes.clone());
            let transaction_bytes = transaction_bytes
                .expect_buffer_bytes_result()
                .map_err(|e| (signers.clone(), signer_state.clone(), diagnosed_error!("{}", e)))?;
            let transaction: Transaction =
                serde_json::from_slice(&transaction_bytes).map_err(|e| {
                    (
                        signers.clone(),
                        signer_state.clone(),
                        diagnosed_error!("failed to serialize transaction bytes: {}", e),
                    )
                })?;

            let _ = transaction.verify_and_hash_message().map_err(|e| {
                (
                    signers.clone(),
                    signer_state.clone(),
                    diagnosed_error!("failed to verify transaction message: {}", e),
                )
            })?;
            Ok((signers, signer_state, res_signing))
        };
        Ok(Box::pin(future))
    }

    fn build_background_task(
        construct_did: &ConstructDid,
        spec: &CommandSpecification,
        values: &ValueStore,
        outputs: &ValueStore,
        progress_tx: &channel::Sender<BlockEvent>,
        background_tasks_uuid: &Uuid,
        supervision_context: &RunbookSupervisionContext,
    ) -> CommandExecutionFutureResult {
        SendTransaction::build_background_task(
            &construct_did,
            &spec,
            &values,
            &outputs,
            &progress_tx,
            &background_tasks_uuid,
            &supervision_context,
        )
    }
}
