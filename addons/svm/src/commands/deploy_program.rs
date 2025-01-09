use std::collections::HashMap;
use std::str::FromStr;
use std::vec;

use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentLevel;
use solana_sdk::pubkey::Pubkey;
use txtx_addon_kit::channel;
use txtx_addon_kit::constants::SIGNED_TRANSACTION_BYTES;
use txtx_addon_kit::types::commands::{
    return_synchronous_result, CommandExecutionFutureResult, CommandExecutionResult,
    CommandImplementation, CommandSpecification, PreCommandSpecification,
};
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::types::frontend::{
    BlockEvent, ProgressBarStatus, ProgressBarStatusColor, StatusUpdater,
};
use txtx_addon_kit::types::signers::{
    SignerActionsFutureResult, SignerInstance, SignerSignFutureResult, SignersState,
};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::{RunbookSupervisionContext, Type, Value};
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::uuid::Uuid;

use crate::codec::anchor::AnchorProgramArtifacts;
use crate::codec::send_transaction::send_transaction_background_task;
use crate::codec::{KeypairOrTxSigner, UpgradeableProgramDeployer};
use crate::constants::{
    AUTO_EXTEND, CHECKED_PUBLIC_KEY, COMMITMENT_LEVEL, DO_AWAIT_CONFIRMATION, IS_DEPLOYMENT,
    KEYPAIR, PROGRAM, PROGRAM_DEPLOYMENT_KEYPAIR, RPC_API_URL, SIGNATURE, TRANSACTION_BYTES,
};
use crate::typing::{SvmValue, ANCHOR_PROGRAM_ARTIFACTS};

use super::get_signers_did;
use super::sign_transaction::SignTransaction;

lazy_static! {
    pub static ref DEPLOY_PROGRAM: PreCommandSpecification = define_command! {
        DeployProgram => {
            name: "Deploy SVM Program",
            matcher: "deploy_program",
            documentation: "`svm::deploy_program` deploys an anchor program to the specified SVM-compatible network.",
            implements_signing_capability: true,
            implements_background_task_capability: true,
            inputs: [
                description: {
                    documentation: "A description of the deployment action.",
                    typing: Type::string(),
                    optional: true,
                    tainting: false,
                    internal: false,
                    sensitive: false
                },
                program: {
                    documentation: "The Solana program artifacts to deploy.",
                    typing: ANCHOR_PROGRAM_ARTIFACTS.clone(),
                    optional: false,
                    tainting: true,
                    internal: false,
                    sensitive: false
                },
                signers: {
                    documentation: "A set of references to a signer construct, which will be used to sign the transaction.",
                    typing: Type::string(),
                    optional: false,
                    tainting: true,
                    internal: false,
                    sensitive: false
                },
                commitment_level: {
                    documentation: "The commitment level expected for considering this action as done ('processed', 'confirmed', 'finalized'). The default is 'confirmed'.",
                    typing: Type::string(),
                    optional: true,
                    tainting: false,
                    internal: false,
                    sensitive: false
                },
                auto_extend: {
                    documentation: "Whether to auto extend the program account for program upgrades. Defaults to `true`.",
                    typing: Type::bool(),
                    optional: true,
                    tainting: false,
                    internal: false,
                    sensitive: false
                }
            ],
            outputs: [
                signature: {
                    documentation: "The transaction computed signature.",
                    typing: Type::string()
                }
            ],
            example: txtx_addon_kit::indoc! {r#"
                action "deploy" "svm::deploy_program" {
                    description = "Deploy hello world program"
                    program = svm::get_program_from_anchor_project("hello_world") 
                    signers = [signer.deployer]
                }
            "#},
      }
    };
}

pub struct DeployProgram;
impl CommandImplementation for DeployProgram {
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
        let signer_did = signers_did.first().unwrap();
        let mut signer_state = signers.pop_signer_state(&signer_did).unwrap();

        let program_artifacts_map = match args.get_expected_object(PROGRAM) {
            Ok(a) => a,
            Err(e) => return Err((signers, signer_state, e)),
        };
        let program_artifacts = match AnchorProgramArtifacts::from_map(&program_artifacts_map) {
            Ok(a) => a,
            Err(e) => return Err((signers, signer_state, diagnosed_error!("{}", e))),
        };

        let rpc_api_url = args
            .get_expected_string(RPC_API_URL)
            .map_err(|e| (signers.clone(), signer_state.clone(), e))?
            .to_string();

        let rpc_client = RpcClient::new(rpc_api_url.clone());

        let auto_extend = args.get_bool(AUTO_EXTEND);

        // safe unwrap because AnchorProgramArtifacts::from_map already checked for the key
        let keypair = program_artifacts_map.get(KEYPAIR).unwrap();
        signer_state.insert_scoped_value(
            &construct_did.to_string(),
            PROGRAM_DEPLOYMENT_KEYPAIR,
            keypair.clone(),
        );

        let payer_pubkey_str =
            signer_state.get_expected_string(CHECKED_PUBLIC_KEY).map_err(|e| {
                (
                    signers.clone(),
                    signer_state.clone(),
                    diagnosed_error!("failed to get signer pubkey: {}", e),
                )
            })?;
        let payer_pubkey = Pubkey::from_str(payer_pubkey_str).map_err(|e| {
            (
                signers.clone(),
                signer_state.clone(),
                diagnosed_error!("failed to get payer pubkey: {}", e),
            )
        })?;

        let deployer = UpgradeableProgramDeployer::new(
            program_artifacts.keypair,
            KeypairOrTxSigner::TxSigner(payer_pubkey.clone()),
            &program_artifacts.bin,
            &payer_pubkey,
            rpc_client,
            None,
            None,
            auto_extend,
        );
        let transactions = deployer.get_transactions().map_err(|e| {
            (
                signers.clone(),
                signer_state.clone(),
                diagnosed_error!("failed to get deploy transactions: {}", e),
            )
        })?;

        let mut args = args.clone();
        args.insert(TRANSACTION_BYTES, Value::array(transactions));
        // args.insert(SIGNERS, Value::array(vec![Value::string(signer_did.to_string())]));
        signers.push_signer_state(signer_state);

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
        mut signers: SignersState,
    ) -> SignerSignFutureResult {
        let signers_did = get_signers_did(args).unwrap();
        let first_signer_did = signers_did.first().unwrap();

        let first_signer_state = signers.pop_signer_state(&first_signer_did).unwrap();
        let payload = first_signer_state
            .get_scoped_value(&construct_did.to_string(), TRANSACTION_BYTES)
            .unwrap()
            .clone();
        signers.push_signer_state(first_signer_state);

        let progress_tx = progress_tx.clone();
        // let signer_did = get_payer_did(args).unwrap();
        let args = args.clone();
        let signers_instances = signers_instances.clone();
        let construct_did = construct_did.clone();
        let spec = spec.clone();
        let progress_tx = progress_tx.clone();

        let mut args = args.clone();
        let future = async move {
            let (signers, signer_state, res) = if let Some(payloads) = payload.as_array() {
                let mut signers_ref = signers;
                let mut last_signer_state_ref = None;
                let mut signatures = vec![];
                let mut signed_transactions_bytes = vec![];

                args.insert(IS_DEPLOYMENT, Value::bool(true));
                let mut status_updater =
                    StatusUpdater::new(&Uuid::new_v4(), &construct_did, &progress_tx);
                for (i, transaction) in payloads.iter().enumerate() {
                    status_updater.propagate_pending_status(&format!(
                        "Sending transaction {}/{}",
                        i + 1,
                        payloads.len()
                    ));
                    let (do_await_confirmation, commitment) = if i == 0 {
                        (true, CommitmentLevel::Processed)
                    } else if i == payloads.len() - 2 {
                        (true, CommitmentLevel::Processed)
                    } else if i == payloads.len() - 1 {
                        status_updater.propagate_status(ProgressBarStatus::new_msg(
                            ProgressBarStatusColor::Green,
                            "Complete",
                            "All program data written to buffer",
                        ));
                        (true, CommitmentLevel::Processed)
                    } else {
                        (false, CommitmentLevel::Processed)
                    };
                    args.insert(COMMITMENT_LEVEL, Value::string(commitment.to_string()));
                    args.insert(DO_AWAIT_CONFIRMATION, Value::bool(do_await_confirmation));

                    if let Some(last_signer_state) = last_signer_state_ref {
                        signers_ref.push_signer_state(last_signer_state);
                    }
                    args.insert(TRANSACTION_BYTES, transaction.clone());
                    let run_signing_future = SignTransaction::run_signed_execution(
                        &construct_did,
                        &spec,
                        &args,
                        &progress_tx,
                        &signers_instances,
                        signers_ref,
                    );
                    let (signers, signer_state, res_signing) = match run_signing_future {
                        Ok(future) => match future.await {
                            Ok(res) => res,
                            Err(err) => return Err(err),
                        },
                        Err(err) => return Err(err),
                    };

                    if i == 0 {
                        status_updater.propagate_status(ProgressBarStatus::new_msg(
                            ProgressBarStatusColor::Green,
                            "Complete",
                            "Program buffer creation complete",
                        ));
                    } else if i == payloads.len() - 1 {
                        status_updater.propagate_status(ProgressBarStatus::new_msg(
                            ProgressBarStatusColor::Green,
                            "Complete",
                            "Program created",
                        ));
                    }

                    signers_ref = signers;

                    let signed_transaction_value =
                        res_signing.outputs.get(SIGNED_TRANSACTION_BYTES).unwrap();
                    let signature = res_signing.outputs.get(SIGNATURE).unwrap();
                    signed_transactions_bytes.push(signed_transaction_value.clone());
                    signatures.push(signature.clone());

                    last_signer_state_ref = Some(signer_state);
                }

                let signed_transactions_bytes = Value::array(signed_transactions_bytes);
                let signatures = Value::array(signatures);
                args.insert(SIGNED_TRANSACTION_BYTES, signed_transactions_bytes.clone());
                let mut result = CommandExecutionResult::new();
                result.outputs.insert(SIGNED_TRANSACTION_BYTES.into(), signed_transactions_bytes);
                result.outputs.insert(SIGNATURE.into(), signatures);
                result.outputs.insert(IS_DEPLOYMENT.into(), Value::bool(true));
                (signers_ref, last_signer_state_ref.unwrap(), result)
            } else {
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
                let signed_transaction_value =
                    res_signing.outputs.get(SIGNED_TRANSACTION_BYTES).unwrap();
                args.insert(SIGNED_TRANSACTION_BYTES, signed_transaction_value.clone());
                verify_signature(signed_transaction_value.clone())
                    .map_err(|e| (signers.clone(), signer_state.clone(), e))?;
                (signers, signer_state, res_signing)
            };

            Ok((signers, signer_state, res))
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
        if values.get_bool(IS_DEPLOYMENT).unwrap_or(false) {
            return return_synchronous_result(Ok(CommandExecutionResult::new()));
        } else {
            send_transaction_background_task(
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
}

fn verify_signature(signed_transaction_value: Value) -> Result<(), Diagnostic> {
    let transaction = SvmValue::to_transaction(&signed_transaction_value)
        .map_err(|e| diagnosed_error!("invalid signed transaction: {}", e))?;
    let _ = transaction
        .verify_and_hash_message()
        .map_err(|e| diagnosed_error!("failed to verify signed transaction: {}", e))?;
    Ok(())
}
