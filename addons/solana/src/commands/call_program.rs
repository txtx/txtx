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
use txtx_addon_kit::types::types::{RunbookSupervisionContext, Type};
use txtx_addon_kit::types::{ConstructDid, ValueStore};
use txtx_addon_kit::uuid::Uuid;
use txtx_addon_kit::AddonDefaults;

use crate::codec::encode_contract_call;
use crate::commands::get_signer_did;
use crate::constants::{CHAIN_ID, PROGRAM_ID, TRANSACTION_MESSAGE_BYTES};

use super::sign_transaction::SignSolanaTransaction;

lazy_static! {
    pub static ref SEND_PROGRAM_CALL: PreCommandSpecification = define_command! {
        SendContractCall => {
            name: "Send Contract Call Transaction",
            matcher: "call_program",
            documentation: "The `call_program` action encodes a program call transaction, signs the transaction using an in-browser signer, and broadcasts the signed transaction to the network.",
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
                program_id: {
                    documentation: "The program identifier to invoke.",
                    typing: Type::array(Type::addon("")),
                    optional: false,
                    tainting: true,
                    internal: false
                },
                chain_id: {
                    documentation: "The network id used to validate the transaction version.",
                    typing: Type::string(),
                    optional: true,
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
                confirmations: {
                    documentation: "Once the transaction is included on a block, the number of blocks to await before the transaction is considered successful and Runbook execution continues.",
                    typing: Type::integer(),
                    optional: true,
                    tainting: false,
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

pub struct SendContractCall;
impl CommandImplementation for SendContractCall {
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
        defaults: &AddonDefaults,
        supervision_context: &RunbookSupervisionContext,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        mut signers: SignersState,
    ) -> SignerActionsFutureResult {
        let signer_did = get_signer_did(args).unwrap();
        let signer_state = signers.pop_signer_state(&signer_did).unwrap();
        // Extract network_id
        let chain_id: String = match args.get_defaulting_string(CHAIN_ID, defaults) {
            Ok(value) => value,
            Err(diag) => return Err((signers, signer_state, diag)),
        };
        let instructions = args
            .get_expected_array("instructions")
            .unwrap()
            .iter()
            .map(|i| i.expect_buffer_bytes())
            .collect::<Vec<Vec<u8>>>();

        let bytes = encode_contract_call(&instructions)
            .map_err(|e| (signers.clone(), signer_state.clone(), diagnosed_error!("{e}")))?;
        signers.push_signer_state(signer_state);

        let mut args = args.clone();
        args.insert(TRANSACTION_MESSAGE_BYTES, bytes);

        SignSolanaTransaction::check_signed_executability(
            construct_did,
            instance_name,
            spec,
            &args,
            defaults,
            supervision_context,
            signers_instances,
            signers,
        )
    }

    fn run_signed_execution(
        construct_did: &ConstructDid,
        spec: &CommandSpecification,
        args: &ValueStore,
        defaults: &AddonDefaults,
        progress_tx: &channel::Sender<BlockEvent>,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        signers: SignersState,
    ) -> SignerSignFutureResult {
        let chain_id: String = args.get_defaulting_string(CHAIN_ID, defaults).unwrap();
        let contract_id_value = args.get_expected_value("contract_id").unwrap();
        let function_name = args.get_expected_string("function_name").unwrap();
        let function_args_values = args.get_expected_array("function_args").unwrap();

        let bytes = encode_contract_call(&vec![]).unwrap();
        let progress_tx = progress_tx.clone();
        let args = args.clone();
        let signers_instances = signers_instances.clone();
        let defaults = defaults.clone();
        let construct_did = construct_did.clone();
        let spec = spec.clone();
        let progress_tx = progress_tx.clone();

        let mut args = args.clone();
        args.insert(TRANSACTION_MESSAGE_BYTES, bytes);

        let future = async move {
            let run_signing_future = SignSolanaTransaction::run_signed_execution(
                &construct_did,
                &spec,
                &args,
                &defaults,
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
        defaults: &AddonDefaults,
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
