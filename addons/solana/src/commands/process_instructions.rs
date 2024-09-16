use std::collections::HashMap;

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
use crate::constants::{CHAIN_ID, TRANSACTION_MESSAGE_BYTES};
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
                            typing: Type::array(Type::Addon(SOLANA_ACCOUNT.into())),
                            optional: false,
                            tainting: true,
                            internal: false
                        },
                        ObjectProperty {
                            name: "data".into(),
                            documentation: "A byte array that specifies which instruction handler on the program to invoke, plus any additional data required by the instruction handler (function arguments)".into(),
                            typing: Type::array(Type::Addon(SOLANA_ACCOUNT.into())),
                            optional: false,
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
        for signer_did in signers_did.iter() {
            let signer_state = signers.pop_signer_state(&signer_did).unwrap();
            // Extract network_id

            let instructions = args
                .get_expected_array("instruction")
                .unwrap()
                .iter()
                .map(|i| i.expect_buffer_bytes())
                .collect::<Vec<Vec<u8>>>();

            let bytes = encode_contract_call(&instructions)
                .map_err(|e| (signers.clone(), signer_state.clone(), diagnosed_error!("{e}")))?;

            let mut args = args.clone();
            args.insert(TRANSACTION_MESSAGE_BYTES, bytes);

            signers.push_signer_state(signer_state);
        }
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
        let chain_id: String = args.get_expected_string(CHAIN_ID).unwrap().into();
        let contract_id_value = args.get_expected_value("contract_id").unwrap();
        let function_name = args.get_expected_string("function_name").unwrap();
        let function_args_values = args.get_expected_array("function_args").unwrap();

        let bytes = encode_contract_call(&vec![]).unwrap();
        let progress_tx = progress_tx.clone();
        let args = args.clone();
        let signers_instances = signers_instances.clone();
        let construct_did = construct_did.clone();
        let spec = spec.clone();
        let progress_tx = progress_tx.clone();

        let mut args = args.clone();
        args.insert(TRANSACTION_MESSAGE_BYTES, bytes);

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

            args.insert(
                SIGNED_TRANSACTION_BYTES,
                res_signing.outputs.get(SIGNED_TRANSACTION_BYTES).unwrap().clone(),
            );
            // let mut res = match BroadcastStacksTransaction::run_execution(
            //     &construct_did,
            //     &spec,
            //     &args,
            //     &defaults,
            //     &progress_tx,
            // ) {
            //     Ok(future) => match future.await {
            //         Ok(res) => res,
            //         Err(diag) => return Err((signers, signer_state, diag)),
            //     },
            //     Err(data) => return Err((signers, signer_state, data)),
            // };

            // res_signing.append(&mut res);

            Ok((signers, signer_state, res_signing))
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
        // BroadcastStacksTransaction::build_background_task(
        //     &construct_did,
        //     &spec,
        //     &inputs,
        //     &outputs,
        //     &defaults,
        //     &progress_tx,
        //     &background_tasks_uuid,
        //     &supervision_context,
        // )
        unimplemented!()
    }
}
