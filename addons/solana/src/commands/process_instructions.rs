use std::collections::HashMap;
use std::str::FromStr;

use solana_sdk::instruction::{AccountMeta, Instruction};
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

use crate::codec::encode_contract_call;
use crate::commands::send_transaction::SendTransaction;
use crate::constants::TRANSACTION_MESSAGE_BYTES;
use crate::typing::SOLANA_ACCOUNT;

use super::get_signers_did;
use super::sign_transaction::SignTransaction;

lazy_static! {
    pub static ref PROCESS_INSTRUCTIONS: PreCommandSpecification = define_command! {
        ProcessInstructions => {
            name: "Process Solana Instructions",
            matcher: "process_instructions",
            documentation: "The `process_instructions` action encode instructions, build, sign and broadcast a transaction",
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
                            typing: Type::array(Type::Addon(SOLANA_ACCOUNT.into())), // TODO - should be an object
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
                }
            ],
            outputs: [

            ],
            example: txtx_addon_kit::indoc! {r#"
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
        mut signers: SignersState,
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

        for instruction_data in instructions_data.iter() {
            let program_id = instruction_data.get("program_id").unwrap().expect_string();
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
                        pubkey: Pubkey::try_from(txtx_addon_kit::hex::decode(pubkey).unwrap())
                            .unwrap(),
                        is_signer,
                        is_writable,
                    }
                })
                .collect::<Vec<_>>();
            let data = instruction_data.get("data").unwrap().expect_buffer_bytes();
            let instruction =
                Instruction { program_id: Pubkey::from_str(program_id).unwrap(), accounts, data };
            instructions.push(instruction);
        }
        let bytes = encode_contract_call(&instructions).unwrap();
        let mut args = args.clone();
        args.insert(TRANSACTION_MESSAGE_BYTES, bytes);

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
            let transaction: Transaction =
                bincode::deserialize(&transaction_bytes.expect_buffer_bytes()).unwrap();
            let res = transaction.verify_and_hash_message().unwrap();
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
