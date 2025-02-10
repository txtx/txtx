use std::collections::HashMap;
use std::str::FromStr;
use std::vec;

use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use txtx_addon_kit::channel;
use txtx_addon_kit::constants::{
    DESCRIPTION, NESTED_CONSTRUCT_COUNT, NESTED_CONSTRUCT_DID, NESTED_CONSTRUCT_INDEX,
    SIGNED_TRANSACTION_BYTES,
};
use txtx_addon_kit::indexmap::IndexMap;
use txtx_addon_kit::types::commands::{
    CommandExecutionFutureResult, CommandExecutionResult, CommandImplementation,
    CommandSpecification, PreCommandSpecification,
};
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::types::frontend::{Actions, BlockEvent, StatusUpdater};
use txtx_addon_kit::types::signers::{
    return_synchronous, PrepareSignedNestedExecutionResult, SignerActionsFutureResult,
    SignerInstance, SignerSignFutureResult, SignersState,
};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::{ObjectType, RunbookSupervisionContext, Type, Value};
use txtx_addon_kit::types::{ConstructDid, Did};
use txtx_addon_kit::uuid::Uuid;

use crate::codec::anchor::AnchorProgramArtifacts;
use crate::codec::send_transaction::send_transaction_background_task;
use crate::codec::{DeploymentTransaction, DeploymentTransactionType, UpgradeableProgramDeployer};
use crate::constants::{
    AUTHORITY, AUTO_EXTEND, CHECKED_PUBLIC_KEY, COMMITMENT_LEVEL, DO_AWAIT_CONFIRMATION,
    FORMATTED_TRANSACTION, IS_DEPLOYMENT, KEYPAIR, PAYER, PROGRAM, PROGRAM_DEPLOYMENT_KEYPAIR,
    PROGRAM_ID, RPC_API_URL, SIGNATURE, SIGNATURES, SIGNERS, TRANSACTION_BYTES,
};
use crate::typing::{SvmValue, ANCHOR_PROGRAM_ARTIFACTS, DEPLOYMENT_TRANSACTION_SIGNATURES};

use super::get_custom_signer_did;
use super::sign_transaction::SignTransaction;

lazy_static! {
    pub static ref DEPLOY_PROGRAM: PreCommandSpecification = {
        let mut command = define_command! {
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
                    signatures: {
                        documentation: "The computed transaction signatures, grouped by transaction type.",
                        typing: DEPLOYMENT_TRANSACTION_SIGNATURES.clone()
                    },
                    program_id: {
                        documentation: "The program ID of the deployed program.",
                        typing: Type::string()
                    }
                ],
                example: txtx_addon_kit::indoc! {r#"
                    action "deploy" "svm::deploy_program" {
                        description = "Deploy hello world program"
                        program = svm::get_program_from_anchor_project("hello_world") 
                        authority = signer.authority
                        payer = signer.payer  # Optional, defaults to authority
                    }
                "#},
            }
        };

        if let PreCommandSpecification::Atomic(ref mut spec) = command {
            spec.create_critical_output = Some("program_id".to_string());
        }
        command
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

    fn prepare_signed_nested_execution(
        construct_did: &ConstructDid,
        instance_name: &str,
        values: &ValueStore,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        mut signers: SignersState,
    ) -> PrepareSignedNestedExecutionResult {
        let (
            (authority_signer_did, mut authority_signer_state),
            (_payer_signer_did, mut payer_signer_state),
        ) = pop_deployment_signers(values, &mut signers);

        let program_artifacts_map = match values.get_expected_object(PROGRAM) {
            Ok(a) => a,
            Err(e) => return Err((signers, authority_signer_state, e)),
        };
        let program_artifacts = match AnchorProgramArtifacts::from_map(&program_artifacts_map) {
            Ok(a) => a,
            Err(e) => return Err((signers, authority_signer_state, diagnosed_error!("{}", e))),
        };

        let rpc_api_url = values
            .get_expected_string(RPC_API_URL)
            .map_err(|e| (signers.clone(), authority_signer_state.clone(), e))?
            .to_string();

        let rpc_client =
            RpcClient::new_with_commitment(rpc_api_url.clone(), CommitmentConfig::finalized());

        let auto_extend = values.get_bool(AUTO_EXTEND);

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
            let authority_pubkey_val =
                authority_signer_state.get_expected_value(CHECKED_PUBLIC_KEY).map_err(|e| {
                    (
                        signers.clone(),
                        authority_signer_state.clone(),
                        diagnosed_error!("failed to get authority pubkey: {}", e),
                    )
                })?;

            SvmValue::to_pubkey(&authority_pubkey_val).map_err(|e| {
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

        let (program_id, transactions) = match authority_signer_state
            .get_scoped_value(&construct_did.to_string(), "deployment_transactions")
        {
            Some(transactions) => {
                let program_id = authority_signer_state
                    .get_scoped_value(&construct_did.to_string(), PROGRAM_ID)
                    .unwrap();
                (program_id.clone(), transactions.clone())
            }
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
                    auto_extend,
                )
                .map_err(|e| {
                    (
                        signers.clone(),
                        authority_signer_state.clone(),
                        diagnosed_error!("failed to initialize deployment: {}", e),
                    )
                })?;

                let program_id = SvmValue::pubkey(deployer.program_pubkey.to_bytes().to_vec());
                authority_signer_state.insert_scoped_value(
                    &construct_did.to_string(),
                    PROGRAM_ID,
                    program_id.clone(),
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
                (program_id, Value::array(transactions))
            }
        };

        signers.push_signer_state(authority_signer_state);
        if let Some(payer_signer_state) = payer_signer_state {
            signers.push_signer_state(payer_signer_state);
        }
        let authority_signer_state =
            signers.get_signer_state(&authority_signer_did).unwrap().clone();

        let mut cursor = 0;
        let mut res = vec![];
        let transaction_array = transactions.as_array().unwrap();
        for transaction_value in transaction_array.iter() {
            let new_did = ConstructDid(Did::from_components(vec![
                construct_did.as_bytes(),
                cursor.to_string().as_bytes(),
            ]));
            let mut value_store =
                ValueStore::new(&format!("{}:{}", instance_name, cursor), &new_did.value());
            let deployment_transaction = DeploymentTransaction::from_value(transaction_value)
                .map_err(|e| (signers.clone(), authority_signer_state.clone(), e))?;

            value_store.insert(NESTED_CONSTRUCT_DID, Value::string(new_did.to_string()));

            value_store.insert_scoped_value(
                &new_did.to_string(),
                "deployment_transaction_type",
                Value::string(deployment_transaction.transaction_type.to_string()),
            );

            value_store.insert_scoped_value(&new_did.to_string(), PROGRAM_ID, program_id.clone());

            value_store.insert_scoped_value(
                &new_did.to_string(),
                TRANSACTION_BYTES,
                transaction_value.clone(),
            );
            value_store.insert_scoped_value(
                &new_did.to_string(),
                NESTED_CONSTRUCT_INDEX,
                Value::integer(cursor as i128),
            );
            value_store.insert_scoped_value(
                &new_did.to_string(),
                NESTED_CONSTRUCT_COUNT,
                Value::integer(transaction_array.len() as i128),
            );
            res.push((new_did, value_store));
            cursor += 1;
        }
        return_synchronous((signers, authority_signer_state.clone(), res))
    }

    fn check_signed_executability(
        construct_did: &ConstructDid,
        instance_name: &str,
        spec: &CommandSpecification,
        values: &ValueStore,
        supervision_context: &RunbookSupervisionContext,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        signers: SignersState,
    ) -> SignerActionsFutureResult {
        let nested_construct_did = values.get_expected_construct_did(NESTED_CONSTRUCT_DID).unwrap();

        let transaction =
            values.get_scoped_value(&nested_construct_did.to_string(), TRANSACTION_BYTES).unwrap();

        let authority_signer_did = get_custom_signer_did(&values, AUTHORITY).unwrap();
        let authority_signer_state =
            signers.get_signer_state(&authority_signer_did).unwrap().clone();

        let deployment_transaction =
            DeploymentTransaction::from_value(transaction).map_err(|e| {
                (
                    signers.clone(),
                    authority_signer_state.clone(),
                    diagnosed_error!("failed to get deployment transaction: {}", e),
                )
            })?;

        if let DeploymentTransactionType::SkipCloseTempAuthority =
            deployment_transaction.transaction_type
        {
            return return_synchronous((signers, authority_signer_state, Actions::none()));
        }

        let (authority_signer_did, payer_signer_did) = get_deployment_dids(&values);
        let signers_dids = deployment_transaction
            .get_signers_dids(authority_signer_did, payer_signer_did)
            .map_err(|e| {
                (
                    signers.clone(),
                    authority_signer_state.clone(),
                    diagnosed_error!("failed to get signers for deployment transaction: {}", e),
                )
            })?;

        // we only need to check signability if there are signers for this transaction
        if let Some(signers_dids) = signers_dids {
            let mut values = values.clone();
            let transaction_value = SvmValue::transaction(&deployment_transaction.transaction)
                .map_err(|e| {
                    (
                        signers.clone(),
                        authority_signer_state.clone(),
                        diagnosed_error!("failed to serialize deployment transaction: {}", e),
                    )
                })?;
            values.insert(TRANSACTION_BYTES, transaction_value);
            values.insert(IS_DEPLOYMENT, Value::bool(true));
            values.insert(
                SIGNERS,
                Value::array(signers_dids.iter().map(|d| Value::string(d.to_string())).collect()),
            );

            let formatted_transaction = deployment_transaction
                .get_formatted_transaction(signers_dids, &signers_instances)
                .map_err(|e| {
                    (
                        signers.clone(),
                        authority_signer_state.clone(),
                        diagnosed_error!("failed to get formatted transaction: {}", e),
                    )
                })?;
            if let Some((formatted_transaction, description)) = formatted_transaction {
                values.insert(FORMATTED_TRANSACTION, Value::string(formatted_transaction));
                values.insert(DESCRIPTION, Value::string(description));
            }
            return SignTransaction::check_signed_executability(
                construct_did,
                instance_name,
                spec,
                &values,
                supervision_context,
                signers_instances,
                signers,
            );
        } else {
            return return_synchronous((signers, authority_signer_state, Actions::none()));
        }
    }

    fn run_signed_execution(
        construct_did: &ConstructDid,
        spec: &CommandSpecification,
        values: &ValueStore,
        progress_tx: &channel::Sender<BlockEvent>,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        signers: SignersState,
    ) -> SignerSignFutureResult {
        let authority_signer_did = get_custom_signer_did(values, AUTHORITY).unwrap();
        let authority_signer_state =
            signers.get_signer_state(&authority_signer_did).unwrap().clone();
        let program_id_value = authority_signer_state
            .get_scoped_value(&construct_did.to_string(), PROGRAM_ID)
            .unwrap()
            .clone();

        let progress_tx = progress_tx.clone();
        let signers_instances = signers_instances.clone();
        let construct_did = construct_did.clone();
        let spec = spec.clone();
        let progress_tx = progress_tx.clone();
        let mut values = values.clone();

        let future = async move {
            let mut result = CommandExecutionResult::new();

            result.outputs.insert(PROGRAM_ID.to_string(), program_id_value.clone());

            let nested_construct_did =
                values.get_expected_construct_did(NESTED_CONSTRUCT_DID).unwrap();

            let transaction_value = values
                .get_scoped_value(&nested_construct_did.to_string(), TRANSACTION_BYTES)
                .unwrap()
                .clone();

            let deployment_transaction =
                DeploymentTransaction::from_value(&transaction_value).unwrap();

            if let DeploymentTransactionType::SkipCloseTempAuthority =
                deployment_transaction.transaction_type
            {
                return Ok((signers, authority_signer_state, result));
            }

            let (authority_signer_did, payer_signer_did) = get_deployment_dids(&values);
            let signers_dids = deployment_transaction
                .get_signers_dids(authority_signer_did, payer_signer_did)
                .unwrap();

            if let Some(signers_dids) = signers_dids {
                values.insert(
                    SIGNERS,
                    Value::array(
                        signers_dids.iter().map(|d| Value::string(d.to_string())).collect(),
                    ),
                );
            } else {
                let rpc_api_url = values.get_expected_string(RPC_API_URL).unwrap();
                let transaction = deployment_transaction
                    .sign_transaction_with_keypairs(rpc_api_url)
                    .map_err(|e| {
                        (
                            signers.clone(),
                            authority_signer_state.clone(),
                            diagnosed_error!("failed to sign transaction: {}", e),
                        )
                    })?;
                let transaction_value = SvmValue::transaction(&transaction).map_err(|e| {
                    (
                        signers.clone(),
                        authority_signer_state.clone(),
                        diagnosed_error!("failed to serialize signed transaction: {}", e),
                    )
                })?;

                result.outputs.insert(
                    format!("{}:{}", &nested_construct_did.to_string(), SIGNED_TRANSACTION_BYTES),
                    transaction_value.clone(),
                );

                return Ok((signers, authority_signer_state, result));
            }
            values.insert(IS_DEPLOYMENT, Value::bool(true));
            values.insert(TRANSACTION_BYTES, transaction_value.clone());

            let run_signing_future = SignTransaction::run_signed_execution(
                &construct_did,
                &spec,
                &values,
                &progress_tx,
                &signers_instances,
                signers,
            );
            let (signers, signer_state, mut signin_res) = match run_signing_future {
                Ok(future) => match future.await {
                    Ok(res) => res,
                    Err(err) => return Err(err),
                },
                Err(err) => return Err(err),
            };

            let signed_transaction_value =
                signin_res.outputs.remove(SIGNED_TRANSACTION_BYTES).unwrap();
            result.append(&mut signin_res);

            result.outputs.insert(
                format!("{}:{}", &nested_construct_did.to_string(), SIGNED_TRANSACTION_BYTES),
                signed_transaction_value,
            );

            return Ok((signers, signer_state, result));
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
        let construct_did = construct_did.clone();
        let spec = spec.clone();
        let mut inputs = inputs.clone();
        let outputs = outputs.clone();
        let progress_tx = progress_tx.clone();
        let background_tasks_uuid = background_tasks_uuid.clone();
        let supervision_context = supervision_context.clone();

        let future = async move {
            let nested_construct_did =
                inputs.get_expected_construct_did(NESTED_CONSTRUCT_DID).unwrap();

            let transaction_value = inputs
                .get_scoped_value(&nested_construct_did.to_string(), TRANSACTION_BYTES)
                .unwrap()
                .clone();
            let transaction_index = inputs
                .get_scoped_integer(&nested_construct_did.to_string(), NESTED_CONSTRUCT_INDEX)
                .unwrap();
            let transaction_count = inputs
                .get_scoped_integer(&nested_construct_did.to_string(), NESTED_CONSTRUCT_COUNT)
                .unwrap();

            let program_id = SvmValue::to_pubkey(&outputs.get_value(PROGRAM_ID).unwrap()).unwrap();
            let deployment_transaction =
                DeploymentTransaction::from_value(&transaction_value).unwrap();

            let mut status_updater = StatusUpdater::new_with_default_progress_index(
                &background_tasks_uuid,
                &construct_did,
                &progress_tx,
                transaction_index as usize,
            );

            deployment_transaction
                .pre_send_status_updates(
                    &mut status_updater,
                    transaction_index as usize,
                    transaction_count as usize,
                )
                .map_err(|e| e)?;
            match &deployment_transaction.transaction_type {
                DeploymentTransactionType::SkipCloseTempAuthority => {
                    return Ok(CommandExecutionResult::new());
                }
                _ => {}
            }

            let signed_transaction_value = inputs
                .get_scoped_value(&nested_construct_did.to_string(), SIGNED_TRANSACTION_BYTES)
                .unwrap()
                .clone();

            inputs.insert(IS_DEPLOYMENT, Value::bool(true));
            inputs.insert(SIGNED_TRANSACTION_BYTES, signed_transaction_value.clone());
            inputs.insert(
                COMMITMENT_LEVEL,
                Value::string(deployment_transaction.commitment_level.to_string()),
            );
            inputs.insert(
                DO_AWAIT_CONFIRMATION,
                Value::bool(deployment_transaction.do_await_confirmation),
            );

            let mut result = match send_transaction_background_task(
                &construct_did,
                &spec,
                &inputs,
                &outputs,
                &progress_tx,
                &background_tasks_uuid,
                &supervision_context,
            ) {
                Ok(res) => match res.await {
                    Ok(res) => res,
                    Err(e) => return Err(e),
                },
                Err(e) => return Err(e),
            };

            let signature = result.outputs.remove(SIGNATURE).unwrap();
            result
                .outputs
                .insert(format!("{}:{}", &nested_construct_did.to_string(), SIGNATURE), signature);

            deployment_transaction.post_send_status_updates(&mut status_updater, program_id);

            Ok(result)
        };
        Ok(Box::pin(future))
    }

    fn aggregate_nested_execution_results(
        _construct_did: &ConstructDid,
        nested_values: &Vec<(ConstructDid, ValueStore)>,
        nested_results: &Vec<CommandExecutionResult>,
    ) -> Result<CommandExecutionResult, Diagnostic> {
        let mut result = CommandExecutionResult::new();

        let mut signatures = IndexMap::new();
        let program_id = nested_values
            .first()
            .and_then(|(id, values)| values.get_scoped_value(&id.to_string(), PROGRAM_ID))
            .unwrap();
        for (res, (nested_construct_did, values)) in nested_results.iter().zip(nested_values) {
            let tx_type = values
                .get_scoped_value(&nested_construct_did.to_string(), "deployment_transaction_type")
                .unwrap()
                .as_string()
                .unwrap();
            let tx_type = DeploymentTransactionType::from_string(&tx_type);

            if let Some(signature) =
                res.outputs.get(&format!("{}:{}", &nested_construct_did.to_string(), SIGNATURE))
            {
                signatures
                    .entry(tx_type.to_string())
                    .or_insert_with(|| Vec::new())
                    .push(signature.clone());
            };
        }
        let object_type = ObjectType::from_map(
            signatures.into_iter().map(|(k, v)| (k, Value::array(v))).collect(),
        );

        result.outputs.insert(SIGNATURES.into(), object_type.to_value());
        result.outputs.insert(PROGRAM_ID.into(), program_id.clone());
        Ok(result)
    }
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

pub fn get_deployment_dids(values: &ValueStore) -> (ConstructDid, ConstructDid) {
    let authority_signer_did = get_custom_signer_did(values, AUTHORITY).unwrap();
    let payer_signer_did =
        get_custom_signer_did(values, PAYER).unwrap_or(authority_signer_did.clone());
    (authority_signer_did, payer_signer_did)
}
