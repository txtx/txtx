use std::collections::HashMap;
use std::thread::sleep;

use solana_client::rpc_client::RpcClient;
use solana_sdk::transaction::Transaction;
use txtx_addon_kit::channel;
use txtx_addon_kit::constants::{DESCRIPTION, SIGNATURE_APPROVED};
use txtx_addon_kit::types::commands::CommandExecutionResult;
use txtx_addon_kit::types::frontend::{
    ActionItemRequest, ActionItemStatus, ReviewInputRequest, VerifyThirdPartySignatureRequest,
};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::signers::{
    return_synchronous_result, CheckSignabilityOk, SignerActionErr, SignerActionsFutureResult,
    SignerActivateFutureResult, SignerInstance, SignerSignFutureResult, SignersState,
};
use txtx_addon_kit::types::signers::{SignerImplementation, SignerSpecification};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::AuthorizationContext;
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::types::{
    commands::CommandSpecification,
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_network_svm_types::SVM_PUBKEY;

use crate::codec::squads::proposal::ProposalStatus;
use crate::codec::squads::SquadsMultisig;
use crate::codec::DeploymentTransaction;
use crate::commands::sign_transaction::{check_signed_executability, run_signed_execution};
use crate::constants::{
    ACTION_ITEM_CHECK_ADDRESS, ACTION_ITEM_PROVIDE_SIGNED_SQUAD_TRANSACTION, ADDRESS,
    CHECKED_ADDRESS, CHECKED_PUBLIC_KEY, FORMATTED_TRANSACTION, IS_DEPLOYMENT, IS_SIGNABLE,
    NAMESPACE, NETWORK_ID, PUBLIC_KEY, RPC_API_URL, SIGNERS, TRANSACTION_BYTES,
};
use crate::typing::SvmValue;
use crate::utils::build_transaction_from_svm_value;
use txtx_addon_kit::types::signers::return_synchronous_actions;
use txtx_addon_kit::types::types::RunbookSupervisionContext;

lazy_static! {
    pub static ref SVM_SQUADS: SignerSpecification = define_signer! {
        SvmSecretKey => {
            name: "Squads Signer",
            matcher: "squads",
            documentation:txtx_addon_kit::indoc! {r#"The `svm::squads` signer can be used to synchronously sign a transaction."#},
            inputs: [
                address: {
                    documentation: "The Squad multisig address.",
                    typing: Type::addon(SVM_PUBKEY),
                    optional: true,
                    tainting: true,
                    sensitive: false
                },
                create_key: {
                    documentation: "The create key used to derive the Squad multisig address.",
                    typing: Type::addon(SVM_PUBKEY),
                    optional: true,
                    tainting: true,
                    sensitive: false
                },
                vault_index: {
                    documentation: "The index of the vault to be created. If omitted, `0` will be used.",
                    typing: Type::integer(),
                    optional: true,
                    tainting: false,
                    sensitive: false
                },
                initiator: {
                    documentation: "A reference to a signer construct, which will be used to create the Squads Vault Transaction & Proposal. This signer must have the `Initiate` permission in the Squads Multisig.",
                    typing: Type::string(),
                    optional: false,
                    tainting: false,
                    sensitive: false
                },
                payer: {
                    documentation: "A reference to a signer construct, which will be used to pay for the Squads Vault Transaction & Proposal creation. If omitted, the `initiator` will be used.",
                    typing: Type::string(),
                    optional: true,
                    tainting: false,
                    sensitive: false
                },
                program_id: {
                    documentation: "The program ID of the Squad program. If omitted, the default program ID will be used.",
                    typing: Type::addon(SVM_PUBKEY),
                    optional: true,
                    tainting: false,
                    sensitive: false
                },
                squads_frontend_url: {
                    documentation: "The URL of the Squads frontend. If omitted, the default URL 'https://app.squads.so' will be used.",
                    typing: Type::string(),
                    optional: true,
                    tainting: false,
                    sensitive: false
                }
            ],
            outputs: [
                public_key: {
                    documentation: "The public key of the Squad multisig.",
                    typing: Type::string()
                },
                address: {
                    documentation: "The address of the Squad multisig.",
                    typing: Type::string()
                },
                vault_public_key: {
                    documentation: "The public key of the Squad vault for the provided vault index.",
                    typing: Type::string()
                },
                vault_address: {
                    documentation: "The address of the Squad vault for the provided vault index.",
                    typing: Type::string()
                }
            ],
            example: txtx_addon_kit::indoc! {r#"
                signer "initiator" "svm::web_wallet" {
                    expected_address = input.initiator_address
                }
                signer "deployer" "svm::squads" {
                    public_key = input.squad_public_key
                    initiator = signer.initiator
                }
            "#},
        }
    };
}

pub struct SvmSecretKey;
impl SignerImplementation for SvmSecretKey {
    fn check_instantiability(
        _ctx: &SignerSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    #[cfg(not(feature = "wasm"))]
    fn check_activability(
        construct_did: &ConstructDid,
        instance_name: &str,
        _spec: &SignerSpecification,
        values: &ValueStore,
        mut signer_state: ValueStore,
        mut signers: SignersState,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        supervision_context: &RunbookSupervisionContext,
        auth_ctx: &AuthorizationContext,
        is_balance_check_required: bool,
        is_public_key_required: bool,
    ) -> SignerActionsFutureResult {
        use txtx_addon_kit::types::signers::consolidate_signer_result;

        use crate::{codec::squads::SquadsMultisig, constants::RPC_API_URL};

        let mut consolidated_actions = Actions::none();

        if signer_state.get_value(CHECKED_PUBLIC_KEY).is_some() {
            return return_synchronous_actions(Ok((signers, signer_state, consolidated_actions)));
        }

        let ((initiator_did, initiator_instance), some_payer) =
            get_signer_states(values, signers_instances)
                .map_err(|e| (signers.clone(), signer_state.clone(), e))?;

        let rpc_api_url = values
            .get_expected_string(RPC_API_URL)
            .map_err(|e| (signers.clone(), signer_state.clone(), e))?;
        let client = RpcClient::new(rpc_api_url.to_string());

        let vault_index = values
            .get_u8("vault_index")
            .map_err(|e| {
                (
                    signers.clone(),
                    signer_state.clone(),
                    diagnosed_error!("invalid vault index: {e}"),
                )
            })?
            .unwrap_or(0);

        let squad_program_id = values
            .get_value("program_id")
            .map(|v| {
                SvmValue::to_pubkey(v).map_err(|e| {
                    (
                        signers.clone(),
                        signer_state.clone(),
                        diagnosed_error!("invalid Squad program ID: {e}"),
                    )
                })
            })
            .transpose()?;

        let squads_frontend_url = values.get_string("squads_frontend_url");

        let squad = if let Some(address) = values.get_value(ADDRESS) {
            let address = SvmValue::to_pubkey(address).map_err(|e| {
                (
                    signers.clone(),
                    signer_state.clone(),
                    diagnosed_error!("invalid Squad multisig address: {e}"),
                )
            })?;
            SquadsMultisig::from_multisig_pda(
                client,
                &address,
                vault_index,
                squad_program_id.as_ref(),
                squads_frontend_url,
            )
            .map_err(|e| (signers.clone(), signer_state.clone(), e))?
        } else if let Some(create_key) = values.get_value("create_key") {
            let create_key = SvmValue::to_pubkey(create_key).map_err(|e| {
                (
                    signers.clone(),
                    signer_state.clone(),
                    diagnosed_error!("invalid Squad multisig create key: {e}"),
                )
            })?;
            SquadsMultisig::from_create_key(
                client,
                &create_key,
                vault_index,
                squad_program_id.as_ref(),
                squads_frontend_url,
            )
            .map_err(|e| (signers.clone(), signer_state.clone(), e))?
        } else {
            return Err((
                signers,
                signer_state,
                diagnosed_error!("Either 'address' or 'create_key' must be provided"),
            ));
        };

        let pubkey = squad.multisig_pda;
        let vault_pubkey = squad.vault_pda;
        let pubkey_value = SvmValue::pubkey(pubkey.to_bytes().to_vec());
        let vault_pubkey_value = SvmValue::pubkey(vault_pubkey.to_bytes().to_vec());
        let pubkey_string_value = Value::string(pubkey.to_string());
        let vault_pubkey_string_value = Value::string(vault_pubkey.to_string());
        let multisig_value = squad.to_value();

        // if supervision_context.review_input_values {
        if let Ok(_) = values.get_expected_string(CHECKED_ADDRESS) {
            signer_state.insert(CHECKED_PUBLIC_KEY, vault_pubkey_value.clone());
            signer_state.insert(CHECKED_ADDRESS, vault_pubkey_string_value.clone());
            signer_state.insert("squads_multisig", multisig_value);
            signer_state.insert("initiator", Value::string(initiator_did.to_string()));
            if let Some((payer_did, _)) = &some_payer {
                signer_state.insert("payer", Value::string(payer_did.to_string()));
            }
        } else {
            consolidated_actions.push_sub_group(
                None,
                vec![ActionItemRequest::new(
                    &Some(construct_did.clone()),
                    &format!("Check {} vault expected address", instance_name),
                    None,
                    ActionItemStatus::Todo,
                    ReviewInputRequest::new("", &vault_pubkey_string_value).to_action_type(),
                    ACTION_ITEM_CHECK_ADDRESS,
                )],
            );
        }

        let values = values.clone();
        let signers_instances = signers_instances.clone();
        let supervision_context = supervision_context.clone();
        let auth_ctx = auth_ctx.clone();
        let future = async move {
            let initiator_signer_state = signers.pop_signer_state(&initiator_did).unwrap();
            let future = (initiator_instance.specification.check_activability)(
                &initiator_did,
                &initiator_instance.name,
                &initiator_instance.specification,
                &values,
                initiator_signer_state,
                signers,
                &signers_instances,
                &supervision_context,
                &auth_ctx,
                is_balance_check_required,
                is_public_key_required,
            )?;
            let (updated_signers, mut actions) = match future.await {
                Ok(res) => consolidate_signer_result(Ok(res), None).unwrap(),
                Err(e) => return Err(e),
            };
            signers = updated_signers;

            consolidated_actions.append(&mut actions);

            if let Some((payer_did, payer_instance)) = some_payer {
                let payer_signer_state = signers.pop_signer_state(&payer_did).unwrap();
                let future = (payer_instance.specification.check_activability)(
                    &payer_did,
                    &payer_instance.name,
                    &payer_instance.specification,
                    &values,
                    payer_signer_state,
                    signers,
                    &signers_instances,
                    &supervision_context,
                    &auth_ctx,
                    is_balance_check_required,
                    is_public_key_required,
                )?;
                let (updated_signers, mut actions) = match future.await {
                    Ok(res) => consolidate_signer_result(Ok(res), None).unwrap(),
                    Err(e) => return Err(e),
                };
                signers = updated_signers;
                consolidated_actions.append(&mut actions);
            }

            Ok((signers, signer_state, consolidated_actions))
        };
        Ok(Box::pin(future))
    }

    fn activate(
        _construct_did: &ConstructDid,
        _spec: &SignerSpecification,
        _values: &ValueStore,
        signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        _progress_tx: &channel::Sender<BlockEvent>,
    ) -> SignerActivateFutureResult {
        let mut result = CommandExecutionResult::new();
        let public_key = signer_state.get_value(CHECKED_PUBLIC_KEY).unwrap();
        let address = signer_state.get_value(CHECKED_ADDRESS).unwrap();
        let multisig =
            signer_state.get_value("squads_multisig").map(SquadsMultisig::from_value).unwrap();

        result.outputs.insert(
            "vault_public_key".into(),
            SvmValue::pubkey(multisig.vault_pda.to_bytes().to_vec()),
        );
        result
            .outputs
            .insert("vault_address".into(), Value::string(multisig.vault_pda.to_string()));
        result.outputs.insert(ADDRESS.into(), address.clone());
        result.outputs.insert(PUBLIC_KEY.into(), public_key.clone());

        return_synchronous_result(Ok((signers, signer_state, result)))
    }

    fn check_signability(
        construct_did: &ConstructDid,
        instance_name: &str,
        description: &Option<String>,
        payload: &Value,
        _spec: &SignerSpecification,
        values: &ValueStore,
        mut signer_state: ValueStore,
        mut signers: SignersState,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        supervision_context: &RunbookSupervisionContext,
    ) -> Result<CheckSignabilityOk, SignerActionErr> {
        signer_state.insert_scoped_value(
            &construct_did.to_string(),
            TRANSACTION_BYTES,
            payload.clone(),
        );

        let is_proposal_created = signer_state
            .get_scoped_value(&construct_did.to_string(), "proposal_created")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if is_proposal_created {
            let construct_did_str = &construct_did.to_string();
            let is_signature_complete = signer_state
                .get_scoped_value(&construct_did_str, "third_party_signature_complete")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if is_signature_complete {
                return Ok((signers, signer_state, Actions::none()));
            }

            let network_id = match values.get_expected_string(NETWORK_ID) {
                Ok(value) => value,
                Err(diag) => return Err((signers, signer_state, diag)),
            };
            let signable = signer_state
                .get_scoped_value(&construct_did_str, IS_SIGNABLE)
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let status = match signable {
                true => ActionItemStatus::Todo,
                false => ActionItemStatus::Blocked,
            };

            let multisig =
                signer_state.get_value("squads_multisig").map(SquadsMultisig::from_value).unwrap();

            let formatted_payload =
                signer_state.get_scoped_value(&construct_did_str, FORMATTED_TRANSACTION);

            let request = ActionItemRequest::new(
                &Some(construct_did.clone()),
                instance_name,
                Some(format!(
                    "Approve Squad proposal: '{}'",
                    description.clone().unwrap_or(instance_name.into())
                )),
                status,
                VerifyThirdPartySignatureRequest::new(
                    &signer_state.uuid,
                    &multisig.vault_transaction_url(),
                    &instance_name,
                    "Squads",
                    payload,
                    NAMESPACE,
                    &network_id,
                )
                .check_expectation_action_uuid(construct_did)
                .formatted_payload(formatted_payload)
                .to_action_type(),
                ACTION_ITEM_PROVIDE_SIGNED_SQUAD_TRANSACTION,
            );
            let actions = Actions::append_item(
                request,
                Some("Review and sign the transactions from the list below"),
                Some("Transaction Signing"),
            );
            return Ok((signers, signer_state, actions));
        } else {
            let rpc_api_url = values
                .get_expected_string(RPC_API_URL)
                .map_err(|e| (signers.clone(), signer_state.clone(), e))?
                .to_string();

            let rpc_client = RpcClient::new(rpc_api_url);

            let multisig =
                signer_state.get_value("squads_multisig").map(SquadsMultisig::from_value).unwrap();

            let ((initiator_did, _), some_payer) =
                get_signer_states(&signer_state, &signers_instances)
                    .map_err(|e| (signers.clone(), signer_state.clone(), e))?;

            let initiator_signer_state = signers.get_signer_state(&initiator_did).unwrap();

            let initiator_pubkey = initiator_signer_state
                .get_expected_value(CHECKED_PUBLIC_KEY)
                .and_then(|v| SvmValue::to_pubkey(v).map_err(Into::into))
                .unwrap();

            let rent_payer_pubkey = if let Some((payer_did, _)) = &some_payer {
                let payer_signer_state = signers.get_signer_state(&payer_did).unwrap();
                let payer_pubkey = payer_signer_state
                    .get_expected_value(CHECKED_PUBLIC_KEY)
                    .and_then(|v| SvmValue::to_pubkey(v).map_err(Into::into))
                    .unwrap();
                payer_pubkey
            } else {
                initiator_pubkey.clone()
            };

            let blockhash = rpc_client.get_latest_blockhash().map_err(|e| {
                (
                    signers.clone(),
                    signer_state.clone(),
                    diagnosed_error!("failed to get latest blockhash: {e}"),
                )
            })?;

            let (inner_transaction, do_sign_with_txtx_signer) =
                if values.get_bool(IS_DEPLOYMENT).unwrap_or(false) {
                    let deployment_transaction = DeploymentTransaction::from_value(&payload)
                        .map_err(|e| {
                            (
                                signers.clone(),
                                signer_state.clone(),
                                diagnosed_error!("failed to sign transaction: {e}"),
                            )
                        })?;

                    let mut transaction: Transaction = deployment_transaction.transaction.clone();

                    transaction.message.recent_blockhash = blockhash;

                    let keypairs = deployment_transaction.get_keypairs().map_err(|e| {
                        (
                            signers.clone(),
                            signer_state.clone(),
                            diagnosed_error!("failed to sign transaction: {e}"),
                        )
                    })?;

                    transaction
                        .try_partial_sign(&keypairs, transaction.message.recent_blockhash)
                        .map_err(|e| {
                        (
                            signers.clone(),
                            signer_state.clone(),
                            diagnosed_error!("failed to sign transaction: {e}"),
                        )
                    })?;

                    (transaction, deployment_transaction.signers.is_some())
                } else {
                    let mut transaction: Transaction =
                        build_transaction_from_svm_value(&payload)
                            .map_err(|e| (signers.clone(), signer_state.clone(), e))?;

                    transaction.message.recent_blockhash = blockhash;

                    (transaction, true)
                };

            let get_proposal_transaction = multisig
                .get_transaction(
                    rpc_client,
                    &initiator_pubkey,
                    &rent_payer_pubkey,
                    inner_transaction.message,
                )
                .map_err(|e| (signers.clone(), signer_state.clone(), e))?;
            let mut initiator_signer_state = signers.pop_signer_state(&initiator_did).unwrap();
            initiator_signer_state.insert_scoped_value(
                &construct_did.to_string(),
                TRANSACTION_BYTES,
                get_proposal_transaction.clone(),
            );
            signers.push_signer_state(initiator_signer_state);

            let mut signers_dids = vec![Value::string(initiator_did.to_string())];
            if let Some((payer_did, _)) = &some_payer {
                signers_dids.push(Value::string(payer_did.to_string()));
            }
            // update our squad signer state to mark that we have created a proposal and are ready to do
            // the actual squad signature
            // signer_state.insert_scoped_value(
            //     &construct_did.to_string(),
            //     "proposal_created",
            //     Value::bool(true),
            // );
            signers.push_signer_state(signer_state);
            // update our transaction description to include some Squads context
            let mut values = values.clone();
            values.insert(
                DESCRIPTION,
                Value::string(format!(
                    "Create Squad proposal: '{}'",
                    values.get_string(DESCRIPTION).unwrap_or(instance_name)
                )),
            );
            values.insert(TRANSACTION_BYTES, get_proposal_transaction);
            values.insert(SIGNERS, Value::array(signers_dids));
            let (updated_signers, signer_state, consolidated_actions) = check_signed_executability(
                construct_did,
                instance_name,
                &values,
                supervision_context,
                signers_instances,
                signers,
            )?;
            signers = updated_signers;
            return Ok((signers, signer_state, consolidated_actions));
        };
    }

    fn sign(
        construct_did: &ConstructDid,
        _title: &str,
        payload: &Value,
        _spec: &SignerSpecification,
        values: &ValueStore,
        mut signer_state: ValueStore,
        mut signers: SignersState,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
    ) -> SignerSignFutureResult {
        let construct_did = construct_did.clone();
        let values = values.clone();
        let signers_instances = signers_instances.clone();

        let future = async move {
            let multisig =
                signer_state.get_value("squads_multisig").map(SquadsMultisig::from_value).unwrap();

            let rpc_api_url = values
                .get_expected_string(RPC_API_URL)
                .map_err(|e| (signers.clone(), signer_state.clone(), e))?
                .to_string();

            let rpc_client = RpcClient::new(rpc_api_url);
            let is_proposal_created = signer_state
                .get_scoped_value(&construct_did.to_string(), "proposal_created")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if is_proposal_created {
                let proposal_status = multisig.get_proposal_status(&rpc_client).map_err(|e| {
                    (
                        signers.clone(),
                        signer_state.clone(),
                        diagnosed_error!("failed to get proposal status: {e}"),
                    )
                })?;

                match proposal_status {
                    ProposalStatus::Rejected { .. } => {
                        return Err((
                            signers.clone(),
                            signer_state.clone(),
                            diagnosed_error!("Proposal rejected"),
                        ))
                    }
                    ProposalStatus::Executed { .. } => {
                        return Ok((
                            signers,
                            signer_state,
                            CommandExecutionResult::from([(
                                "third_party_signature_complete",
                                Value::bool(true),
                            )]),
                        ));
                    }
                    ProposalStatus::Cancelled { .. } => {
                        return Err((
                            signers.clone(),
                            signer_state.clone(),
                            diagnosed_error!("Proposal cancelled"),
                        ))
                    }
                    _ => {
                        return Ok((
                            signers,
                            signer_state,
                            CommandExecutionResult::from([(
                                "third_party_signature_complete",
                                Value::bool(false),
                            )]),
                        ));
                    }
                }
            } else {
                let ((initiator_did, _), some_payer) =
                    get_signer_states(&signer_state, &signers_instances)
                        .map_err(|e| (signers.clone(), signer_state.clone(), e))?;

                let mut signers_dids = vec![Value::string(initiator_did.to_string())];
                if let Some((payer_did, _)) = &some_payer {
                    signers_dids.push(Value::string(payer_did.to_string()));
                }

                // update our squad signer state to mark that we have created a proposal and are ready to do
                // the actual squad signature
                signer_state.insert_scoped_value(
                    &construct_did.to_string(),
                    "proposal_created",
                    Value::bool(true),
                );
                signer_state.insert_scoped_value(
                    &construct_did.to_string(),
                    "third_party_signature_complete",
                    Value::bool(false),
                );
                signers.push_signer_state(signer_state);

                let mut values = values.clone();
                values.insert(SIGNERS, Value::array(signers_dids));

                let run_signing_future =
                    run_signed_execution(&construct_did, &values, &signers_instances, signers);
                let (signers, signer_state, mut result) = match run_signing_future {
                    Ok(future) => match future.await {
                        Ok(res) => res,
                        Err(err) => return Err(err),
                    },
                    Err(err) => return Err(err),
                };
                result.insert("third_party_signature_complete", Value::bool(false));

                Ok((signers, signer_state, result))
            }
        };

        Ok(Box::pin(future))
    }
}

pub fn get_signer_states(
    values: &ValueStore,
    signers_instances: &HashMap<ConstructDid, SignerInstance>,
) -> Result<(SignerDidWithInstance, Option<SignerDidWithInstance>), Diagnostic> {
    let initiator_uuid = values
        .get_expected_string("initiator")
        .map(|uuid| ConstructDid::from_hex_string(uuid))
        .map_err(|e| e)?;
    let initiator_signer_instance = signers_instances
        .get(&initiator_uuid)
        .ok_or_else(|| diagnosed_error!("Squads signer initiator not found"))?;

    let payer = if let Some(payer_uuid) =
        values.get_string("payer").map(|uuid| ConstructDid::from_hex_string(uuid))
    {
        if payer_uuid.ne(&initiator_uuid) {
            let payer_signer_instance = signers_instances
                .get(&payer_uuid)
                .ok_or_else(|| diagnosed_error!("Squads signer payer not found"))?;
            Some((payer_uuid, payer_signer_instance.clone()))
        } else {
            None
        }
    } else {
        None
    };
    Ok(((initiator_uuid, initiator_signer_instance.clone()), payer))
}

type SignerDidWithInstance = (ConstructDid, SignerInstance);
