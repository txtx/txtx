use std::collections::HashMap;
use std::str::FromStr;
use std::vec;

use solana_client::rpc_client::RpcClient;
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
use txtx_addon_kit::types::types::{RunbookSupervisionContext, Type, Value};
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::uuid::Uuid;

use crate::codec::anchor::AnchorProgramArtifacts;
use crate::codec::{KeypairOrTxSigner, UpgradeableProgramDeployer};
use crate::commands::send_transaction::SendTransaction;
use crate::constants::{AUTO_EXTEND, PROGRAM_DEPLOYMENT_KEYPAIR, RPC_API_URL, TRANSACTION_BYTES};
use crate::typing::ANCHOR_PROGRAM_ARTIFACTS;

use super::get_signers_did;
use super::sign_transaction::SignTransaction;

lazy_static! {
    pub static ref DEPLOY_PROGRAM: PreCommandSpecification = define_command! {
        DeployProgram => {
            name: "Deploy SVM Program",
            matcher: "deploy_program",
            documentation: "`svm::deploy_program` is coming soon",
            implements_signing_capability: true,
            implements_background_task_capability: true,
            inputs: [
                description: {
                    documentation: "Description of the program",
                    typing: Type::string(),
                    optional: true,
                    tainting: false,
                    internal: false
                },
                program: {
                    documentation: "Coming soon",
                    typing: ANCHOR_PROGRAM_ARTIFACTS.clone(),
                    optional: false,
                    tainting: true,
                    internal: false
                },
                signers: {
                    documentation: "A reference to a signer construct, which will be used to pay for the deployment.",
                    typing: Type::string(),
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
                },
                auto_extend: {
                    documentation: "Whether to auto extend the program account for program upgrades. Defaults to `true`.",
                    typing: Type::bool(),
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

        let program_artifacts_map = match args.get_expected_object("program") {
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
        let keypair = program_artifacts_map.get("keypair").unwrap();
        signer_state.insert_scoped_value(
            &construct_did.to_string(),
            PROGRAM_DEPLOYMENT_KEYPAIR,
            keypair.clone(),
        );

        let payer_pubkey_str =
            signer_state.get_expected_string("checked_public_key").map_err(|e| {
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
        signers: SignersState,
    ) -> SignerSignFutureResult {
        let progress_tx = progress_tx.clone();
        // let signer_did = get_payer_did(args).unwrap();
        let args = args.clone();
        let signers_instances = signers_instances.clone();
        let construct_did = construct_did.clone();
        let spec = spec.clone();
        let progress_tx = progress_tx.clone();

        let mut args = args.clone();
        let future = async move {
            // args.insert(SIGNERS, Value::array(vec![Value::string(signer_did.to_string())]));
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
            if let Some(transactions_values) = signed_transaction_value.as_array() {
                for transaction_value in transactions_values.iter() {
                    let transaction_bytes =
                        transaction_value.expect_buffer_bytes_result().map_err(|e| {
                            (signers.clone(), signer_state.clone(), diagnosed_error!("{}", e))
                        })?;
                    let transaction: Transaction = serde_json::from_slice(&transaction_bytes)
                        .map_err(|e| {
                            (
                                signers.clone(),
                                signer_state.clone(),
                                diagnosed_error!("invalid signed transaction: {}", e),
                            )
                        })?;
                    let _ = transaction.verify_and_hash_message().map_err(|e| {
                        (
                            signers.clone(),
                            signer_state.clone(),
                            diagnosed_error!("failed to verify signed transaction: {}", e),
                        )
                    })?;
                }
            } else {
                let transaction_bytes =
                    signed_transaction_value.expect_buffer_bytes_result().map_err(|e| {
                        (signers.clone(), signer_state.clone(), diagnosed_error!("{}", e))
                    })?;
                let transaction: Transaction =
                    serde_json::from_slice(&transaction_bytes).map_err(|e| {
                        (
                            signers.clone(),
                            signer_state.clone(),
                            diagnosed_error!("invalid signed transaction: {}", e),
                        )
                    })?;
                let _ = transaction.verify_and_hash_message().map_err(|e| {
                    (
                        signers.clone(),
                        signer_state.clone(),
                        diagnosed_error!("failed to verify signed transaction: {}", e),
                    )
                })?;
            }
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
