use std::collections::HashMap;
use std::str::FromStr;

use solana_client::rpc_client::RpcClient;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::message::Message;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction;
use txtx_addon_kit::channel;
use txtx_addon_kit::constants::SIGNED_TRANSACTION_BYTES;
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
use txtx_addon_kit::types::types::{ObjectProperty, RunbookSupervisionContext, Type};
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::uuid::Uuid;

use crate::commands::send_transaction::SendTransaction;
use crate::constants::{PROGRAM_ID, RPC_API_URL, TRANSACTION_BYTES};
use crate::typing::{SvmValue, SVM_ACCOUNT};

use super::get_signers_did;
use super::sign_transaction::SignTransaction;

lazy_static! {
    pub static ref PROCESS_INSTRUCTIONS: PreCommandSpecification = define_command! {
        ProcessInstructions => {
            name: "Process SVM Instructions",
            matcher: "process_instructions",
            documentation: "The `svm::process_instructions` action encodes instructions that are added to a transaction that is signed and broadcasted to the network.",
            implements_signing_capability: true,
            implements_background_task_capability: true,
            inputs: [
                description: {
                    documentation: "Description of the transaction",
                    typing: Type::string(),
                    optional: true,
                    tainting: false,
                    internal: false
                },
                instruction: {
                    documentation: "Instructions to process",
                    typing: Type::map(vec![
                        ObjectProperty {
                            name: "description".into(),
                            documentation: "Description of the instruction".into(),
                            typing: Type::string(),
                            optional: true,
                            tainting: false,
                            internal: false
                        },
                        ObjectProperty {
                            name: "program_id".into(),
                            documentation: "Specifies the program being invoked".into(),
                            typing: Type::string(),
                            optional: false,
                            tainting: true,
                            internal: false
                        },
                        ObjectProperty {
                            name: "accounts".into(),
                            documentation: "Lists every account the instruction reads from or writes to, including other programs".into(),
                            typing: Type::array(Type::Addon(SVM_ACCOUNT.into())), // TODO - should be an object
                            optional: false,
                            tainting: true,
                            internal: false
                        },
                        ObjectProperty {
                            name: "data".into(),
                            documentation: "A byte array that specifies which instruction handler on the program to invoke, plus any additional data required by the instruction handler (function arguments)".into(),
                            typing: Type::buffer(),
                            optional: true,
                            tainting: true,
                            internal: false
                        }
                    ]),
                    optional: false,
                    tainting: true,
                    internal: false
                },
                signers: {
                    documentation: "Set of references to a signer construct, which will be used to sign the transaction.",
                    typing: Type::array(Type::string()),
                    optional: false,
                    tainting: true,
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

            action "program_call" "svm::process_instructions" {
                description = "Invoke instructions"
                instruction {
                    program_id = variable.program
                    accounts = [svm::account(signer.caller.address, true, true)]
                    data = svm::get_instruction_data_from_idl(variable.program.idl, "my_instruction", ["arg1", "arg2"])
                }
                signers = [signer.caller]
            }
    "#},
      }
    };
}

pub struct ProcessInstructions;
impl CommandImplementation for ProcessInstructions {
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
        let signers_did = get_signers_did(args).unwrap();

        // TODO: revisit pattern and leverage `check_instantiability` instead`.
        let mut instructions = vec![];
        let instructions_data = args
            .get_expected_array("instruction")
            .unwrap()
            .iter()
            .map(|i| i.expect_object())
            .collect::<Vec<_>>();
        let rpc_api_url = args.get_expected_string(RPC_API_URL).unwrap().to_string();

        for instruction_data in instructions_data.iter() {
            let program_id = instruction_data.get(PROGRAM_ID).unwrap().expect_string();
            let accounts = instruction_data
                .get("accounts")
                .unwrap()
                .expect_array()
                .iter()
                .map(|a| {
                    let account = a.expect_object();
                    let pubkey = account.get("public_key").unwrap().expect_string();
                    let is_signer = account.get("is_signer").unwrap().expect_bool();
                    let is_writable = account.get("is_writable").unwrap().expect_bool();
                    AccountMeta {
                        pubkey: Pubkey::try_from(pubkey).unwrap(),
                        is_signer,
                        is_writable,
                    }
                })
                .collect::<Vec<_>>();
            let data = instruction_data.get("data").unwrap().expect_buffer_bytes();
            let program_id = Pubkey::from_str(program_id).unwrap();
            let instruction = Instruction { program_id, accounts, data };
            instructions.push(instruction);
        }

        let mut message = Message::new(&instructions, None);
        let client = RpcClient::new(rpc_api_url);
        message.recent_blockhash = client.get_latest_blockhash().unwrap();
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
            let (signers, signer_state, mut res_signing) = match run_signing_future {
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

            let _ = transaction.verify_and_hash_message().unwrap();
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
