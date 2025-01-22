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
use crate::codec::UpgradeableProgramDeployer;
use crate::constants::{
    AUTHORITY, AUTO_EXTEND, CHECKED_PUBLIC_KEY, COMMITMENT_LEVEL, DO_AWAIT_CONFIRMATION,
    IS_DEPLOYMENT, KEYPAIR, PAYER, PROGRAM, PROGRAM_DEPLOYMENT_KEYPAIR, PROGRAM_ID, RPC_API_URL,
    SIGNATURE, TRANSACTION_BYTES,
};
use crate::typing::{SvmValue, ANCHOR_PROGRAM_ARTIFACTS};

use super::get_custom_signer_did;
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
                payer: {
                    documentation: "A reference to a signer construct, which will be used to sign transactions that pay for the program deployment. If omitted, the `authority` will be used.",
                    typing: Type::string(),
                    optional: true,
                    tainting: false,
                    internal: false,
                    sensitive: false
                },
                authority: {
                    documentation: "A reference to a signer construct, which will be the final authority for the deployed program.",
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
        let (
            (_authority_signer_did, mut authority_signer_state),
            (_payer_signer_did, mut payer_signer_state),
        ) = pop_deployment_signers(args, &mut signers);

        let program_artifacts_map = match args.get_expected_object(PROGRAM) {
            Ok(a) => a,
            Err(e) => return Err((signers, authority_signer_state, e)),
        };
        let program_artifacts = match AnchorProgramArtifacts::from_map(&program_artifacts_map) {
            Ok(a) => a,
            Err(e) => return Err((signers, authority_signer_state, diagnosed_error!("{}", e))),
        };

        let rpc_api_url = args
            .get_expected_string(RPC_API_URL)
            .map_err(|e| (signers.clone(), authority_signer_state.clone(), e))?
            .to_string();

        let rpc_client = RpcClient::new(rpc_api_url.clone());

        let auto_extend = args.get_bool(AUTO_EXTEND);

        // safe unwrap because AnchorProgramArtifacts::from_map already checked for the key
        let keypair = program_artifacts_map.get(KEYPAIR).unwrap();

        insert_to_payer_or_authority(
            &mut payer_signer_state,
            &mut authority_signer_state,
            |signer_state| {
                signer_state.insert_scoped_value(
                    &construct_did.to_string(),
                    PROGRAM_DEPLOYMENT_KEYPAIR,
                    keypair.clone(),
                );
            },
        );

        let authority_pubkey = {
            let authority_pubkey_str =
                authority_signer_state.get_expected_string(CHECKED_PUBLIC_KEY).map_err(|e| {
                    (
                        signers.clone(),
                        authority_signer_state.clone(),
                        diagnosed_error!("failed to get authority pubkey: {}", e),
                    )
                })?;

            Pubkey::from_str(authority_pubkey_str).map_err(|e| {
                (
                    signers.clone(),
                    authority_signer_state.clone(),
                    diagnosed_error!("invalid authority pubkey: {}", e),
                )
            })?
        };

        let payer_pubkey = {
            let payer_pubkey_str = get_from_payer_or_authority(
                &payer_signer_state,
                &authority_signer_state,
                |signer_state| signer_state.get_expected_string(CHECKED_PUBLIC_KEY),
            )
            .map_err(|e| {
                (
                    signers.clone(),
                    authority_signer_state.clone(),
                    diagnosed_error!("failed to get payer pubkey: {}", e),
                )
            })?;

            Pubkey::from_str(payer_pubkey_str).map_err(|e| {
                (
                    signers.clone(),
                    authority_signer_state.clone(),
                    diagnosed_error!("invalid payer pubkey: {}", e),
                )
            })?
        };

        let transactions = match authority_signer_state
            .get_scoped_value(&construct_did.to_string(), "deployment_transactions")
        {
            Some(transactions) => transactions.clone(),
            None => {
                let temp_authority_keypair = match authority_signer_state
                    .get_scoped_value(&construct_did.to_string(), "temp_authority_keypair")
                {
                    Some(kp) => SvmValue::to_keypair(kp).map_err(|e| {
                        (
                            signers.clone(),
                            authority_signer_state.clone(),
                            diagnosed_error!("failed to create temp authority keypair: {}", e),
                        )
                    })?,
                    None => {
                        let temp_authority_keypair =
                            UpgradeableProgramDeployer::create_temp_authority();
                        authority_signer_state.insert_scoped_value(
                            &construct_did.to_string(),
                            "temp_authority_keypair",
                            SvmValue::keypair(temp_authority_keypair.to_bytes().to_vec()),
                        );
                        temp_authority_keypair
                    }
                };

                let mut deployer = UpgradeableProgramDeployer::new(
                    program_artifacts.keypair,
                    &authority_pubkey,
                    temp_authority_keypair,
                    &program_artifacts.bin,
                    &payer_pubkey,
                    rpc_client,
                    None,
                    None,
                    auto_extend,
                )
                .map_err(|e| {
                    (
                        signers.clone(),
                        authority_signer_state.clone(),
                        diagnosed_error!("failed to initialize deployment: {}", e),
                    )
                })?;

                authority_signer_state.insert_scoped_value(
                    &construct_did.to_string(),
                    PROGRAM_ID,
                    SvmValue::pubkey(deployer.program_pubkey.to_bytes().to_vec()),
                );
                authority_signer_state.insert_scoped_value(
                    &construct_did.to_string(),
                    "is_program_upgrade",
                    Value::bool(deployer.is_program_upgrade),
                );

                let transactions = deployer.get_transactions().map_err(|e| {
                    (
                        signers.clone(),
                        authority_signer_state.clone(),
                        diagnosed_error!("failed to get deploy transactions: {}", e),
                    )
                })?;

                authority_signer_state.insert_scoped_value(
                    &construct_did.to_string(),
                    "deployment_transactions",
                    Value::array(transactions.clone()),
                );
                Value::array(transactions)
            }
        };

        let mut args = args.clone();
        args.insert(IS_DEPLOYMENT, Value::bool(true));
        args.insert(TRANSACTION_BYTES, transactions);

        signers.push_signer_state(authority_signer_state);
        if let Some(payer_signer_state) = payer_signer_state {
            signers.push_signer_state(payer_signer_state);
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
        let authority_signer_did = get_custom_signer_did(args, AUTHORITY).unwrap();
        let authority_signer_state =
            signers.get_signer_state(&authority_signer_did).unwrap().clone();
        let payload = authority_signer_state
            .get_scoped_value(&construct_did.to_string(), "deployment_transactions")
            .unwrap()
            .clone();
        let program_id = SvmValue::to_pubkey(
            authority_signer_state
                .get_scoped_value(&construct_did.to_string(), PROGRAM_ID)
                .unwrap(),
        )
        .unwrap();
        let is_program_upgrade = authority_signer_state
            .get_scoped_bool(&construct_did.to_string(), "is_program_upgrade")
            .unwrap();

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

                for (i, mut transaction) in payloads.clone().into_iter().enumerate() {
                    status_updater.propagate_pending_status(&format!(
                        "Sending transaction {}/{}",
                        i + 1,
                        payloads.len()
                    ));

                    let (do_await_confirmation, commitment) =
                    // the first transaction is the temp authority creation
                    if i == 0 {
                        (true, CommitmentLevel::Processed)
                    }
                    // the second transaction creates the buffer
                    else if i == 1 {
                        (true, CommitmentLevel::Processed)
                    }
                    else if i == payloads.len() - 4 {
                        (true, CommitmentLevel::Processed)
                    }
                    // the third-to-last transaction transfers authority of the buffer to the final authority
                    else if i == payloads.len() - 3 {
                        (true, CommitmentLevel::Processed)
                    }
                    // the second-to-last transaction deploys/upgrades the program
                     else if i == payloads.len() - 2 {
                        (true, CommitmentLevel::Processed)
                    }
                    // the last transaction closes the temp authority
                    else if i == payloads.len() - 1 {
                        transaction =
                            UpgradeableProgramDeployer::get_close_temp_authority_transaction(
                                &transaction,
                            )
                            .map_err(|e| {
                                (
                                    signers_ref.clone(),
                                    authority_signer_state.clone(),
                                    diagnosed_error!("failed to close temp authority: {}", e),
                                )
                            })?;
                        (true, CommitmentLevel::Processed)
                    }
                    // all other transactions are for writing to the buffer
                    else {
                        (false, CommitmentLevel::Processed)
                    };

                    let (_, expected_signer_did) =
                        UpgradeableProgramDeployer::get_signer_did_from_transaction_value(
                            &transaction,
                            &args,
                        )
                        .unwrap();

                    if let Some(expected_signer_did) = expected_signer_did {
                        args.insert(
                            "expected_signer",
                            Value::string(expected_signer_did.to_string()),
                        );
                    }
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
                            "Account Created",
                            "Temp account created to write to buffer",
                        ));
                    } else if i == 1 {
                        status_updater.propagate_status(ProgressBarStatus::new_msg(
                            ProgressBarStatusColor::Green,
                            "Account Created",
                            "Program buffer creation complete",
                        ));
                    } else if i == payloads.len() - 2 {
                        status_updater.propagate_status(ProgressBarStatus::new_msg(
                            ProgressBarStatusColor::Green,
                            "Program Created",
                            &format!(
                                "Program {} has been {}",
                                program_id,
                                if is_program_upgrade { "upgraded" } else { "deployed" }
                            ),
                        ));
                    } else if i == payloads.len() - 1 {
                        status_updater.propagate_status(ProgressBarStatus::new_msg(
                            ProgressBarStatusColor::Green,
                            "Complete",
                            "Temp account closed and leftover funds returned to payer",
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

fn insert_to_payer_or_authority<'a>(
    payer_signer_state: &'a mut Option<ValueStore>,
    authority_signer_state: &'a mut ValueStore,
    setter: impl Fn(&'a mut ValueStore),
) {
    if let Some(payer_signer_state) = payer_signer_state {
        setter(payer_signer_state)
    } else {
        setter(authority_signer_state)
    }
}

fn get_from_payer_or_authority<'a, ReturnType>(
    payer_signer_state: &'a Option<ValueStore>,
    authority_signer_state: &'a ValueStore,
    getter: impl Fn(&'a ValueStore) -> ReturnType,
) -> ReturnType {
    if let Some(payer_signer_state) = payer_signer_state {
        getter(payer_signer_state)
    } else {
        getter(authority_signer_state)
    }
}

pub fn pop_deployment_signers<'a>(
    values: &ValueStore,
    signers: &mut SignersState,
) -> ((ConstructDid, ValueStore), (ConstructDid, Option<ValueStore>)) {
    let authority_signer_did = get_custom_signer_did(values, AUTHORITY).unwrap();
    let payer_signer_did =
        get_custom_signer_did(values, PAYER).unwrap_or(authority_signer_did.clone());
    let authority_signer_state = signers.pop_signer_state(&authority_signer_did).unwrap();
    let payer_signer_state = if payer_signer_did.eq(&authority_signer_did) {
        None
    } else {
        Some(signers.pop_signer_state(&payer_signer_did).unwrap())
    };

    ((authority_signer_did, authority_signer_state), (payer_signer_did, payer_signer_state))
}

pub fn get_deployment_signers<'a>(
    values: &ValueStore,
    signers: &SignersState,
) -> ((ConstructDid, ValueStore), (ConstructDid, Option<ValueStore>)) {
    let authority_signer_did = get_custom_signer_did(values, AUTHORITY).unwrap();
    let payer_signer_did =
        get_custom_signer_did(values, PAYER).unwrap_or(authority_signer_did.clone());
    let authority_signer_state = signers.get_signer_state(&authority_signer_did).unwrap();
    let payer_signer_state = if payer_signer_did.eq(&authority_signer_did) {
        None
    } else {
        Some(signers.get_signer_state(&payer_signer_did).unwrap())
    };

    (
        (authority_signer_did, authority_signer_state.clone()),
        (payer_signer_did, payer_signer_state.cloned()),
    )
}
