use std::collections::HashMap;

use solana_client::rpc_client::RpcClient;
use solana_signature::Signature;
use solana_transaction::Transaction;
use txtx_addon_kit::channel;
use txtx_addon_kit::constants::{DocumentationKey, RunbookKey, SignerKey};
use txtx_addon_kit::types::commands::CommandExecutionResult;
use txtx_addon_kit::types::frontend::{
    ActionItemStatus, ReviewInputRequest, VerifyThirdPartySignatureRequest,
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
use txtx_addon_kit::{
    constants::ActionItemKey, types::frontend::ActionItemRequestUpdate,
};
use txtx_addon_network_svm_types::SVM_PUBKEY;

use crate::codec::squads::proposal::ProposalStatus;
use crate::codec::squads::SquadsMultisig;
use crate::codec::ui_encode::get_formatted_transaction_meta_description;
use crate::commands::sign_transaction::{check_signed_executability, run_signed_execution};
use crate::constants::{
    ADDRESS,
    CHECKED_ADDRESS, CHECKED_PUBLIC_KEY, FORMATTED_TRANSACTION, INITIATOR, IS_DEPLOYMENT,
    IS_SIGNABLE, MULTISIG_ACCOUNT_ADDRESS, MULTISIG_ACCOUNT_PUBLIC_KEY, NAMESPACE, NETWORK_ID,
    PAYER, PUBLIC_KEY, RPC_API_URL, SIGNATURE, SIGNERS, SQUADS_MULTISIG, TRANSACTION_BYTES,
    VAULT_ADDRESS, VAULT_PUBLIC_KEY,
};
use crate::typing::SvmValue;
use crate::utils::build_transaction_from_svm_value;
use txtx_addon_kit::types::signers::return_synchronous_actions;
use txtx_addon_kit::types::types::{RunbookSupervisionContext, ThirdPartySignatureStatus};

pub const SQUADS_MATCHER: &str = "squads";

lazy_static! {
    pub static ref SVM_SQUADS: SignerSpecification = define_signer! {
        SvmSecretKey => {
            name: "Squads Signer",
            matcher: SQUADS_MATCHER,
            documentation:txtx_addon_kit::indoc! {r#"The `svm::squads` signer can be used to sign a transaction with a squads multisig."#},
            inputs: [
                multisig_account_public_key: {
                    documentation: "The Squad multisig account pubkey. This is found on the settings page in the Squads app. This is not the vault address. Rather, this multisig account address will be used to derive the vault address and all transaction PDAs.",
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
                    documentation: "The public key of the Squad vault for the provided vault index. This is an alias for the `vault_public_key` output",
                    typing: Type::string()
                },
                address: {
                    documentation: "The public key of the Squad vault for the provided vault index. This is an alias for the `vault_public_key` output",
                    typing: Type::string()
                },
                vault_public_key: {
                    documentation: "The public key of the Squad vault for the provided vault index.",
                    typing: Type::string()
                },
                vault_address: {
                    documentation: "The public key of the Squad vault for the provided vault index. This is an alias for the `vault_public_key` output",
                    typing: Type::string()
                },
                multisig_account_public_key: {
                    documentation: "The public key of the Squad multisig pda. This address should not be funded.",
                    typing: Type::string()
                },
                multisig_account_address: {
                    documentation: "The public key of the Squad multisig pda. This address should not be funded. This is an alias for the `multisig_account_public_key` output",
                    typing: Type::string()
                }
            ],
            example: txtx_addon_kit::indoc! {r#"
                signer "initiator" "svm::web_wallet" {
                    expected_address = input.initiator_address
                }
                signer "deployer" "svm::squads" {
                    multisig_account_public_key = input.squads_multisig_address
                    initiator = signer.initiator
                }
            "#},
            force_sequential_signing: true
        }
    };
    pub static ref SQUADS_DEPLOYMENT_ADDITIONAL_INFO_TITLE: String =
        "Squads Deployment Follow-up Steps".into();
    pub static ref SQUADS_DEPLOYMENT_ADDITIONAL_INFO: String = format!(
        r#"
You've now deployed your program to with you Squads multisig as the authority!
However, to get the program to show up in the Squads app, you'll need to add it manually.

Follow steps 1 and 2 here to add your program to the squad frontend:
https://docs.squads.so/main/navigating-your-squad/developers-assets/programs#add-programs
                    "#
    );
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
use txtx_addon_kit::constants::{ActionItemKey, DocumentationKey};
        use txtx_addon_kit::types::signers::consolidate_signer_result;

        use crate::constants::{
            CREATE_KEY, MULTISIG_ACCOUNT_PUBLIC_KEY, PROGRAM_ID, SQUADS_FRONTEND_URL, VAULT_INDEX,
        };
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
            .get_u8(VAULT_INDEX)
            .map_err(|e| {
                (
                    signers.clone(),
                    signer_state.clone(),
                    diagnosed_error!("invalid vault index: {e}"),
                )
            })?
            .unwrap_or(0);

        let squad_program_id = values
            .get_value(PROGRAM_ID)
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

        let squads_frontend_url = values.get_string(SQUADS_FRONTEND_URL);

        let squad = if let Some(address) = values.get_value(MULTISIG_ACCOUNT_PUBLIC_KEY) {
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
        } else if let Some(create_key) = values.get_value(CREATE_KEY) {
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

        let is_balance_checked = signer_state.get_bool(SignerKey::IsBalanceChecked.as_ref());
        let rpc_api_url = values
            .get_expected_string(RPC_API_URL)
            .map_err(|e| (signers.clone(), signer_state.clone(), e))?
            .to_owned();
        let network_id = values
            .get_expected_string(NETWORK_ID)
            .map_err(|e| (signers.clone(), signer_state.clone(), e))?
            .to_owned();

        let pubkey = squad.multisig_pda;
        let vault_pubkey = squad.vault_pda;
        let vault_pubkey_value = SvmValue::pubkey(vault_pubkey.to_bytes().to_vec());
        let vault_pubkey_string_value = Value::string(vault_pubkey.to_string());
        let multisig_value = squad.to_value();
        let description = values.get_string(DocumentationKey::Description.as_ref()).map(|d| d.to_string());
        let markdown = values
            .get_markdown(auth_ctx)
            .map_err(|d| (signers.clone(), signer_state.clone(), d))?;
        let mut action_items = vec![];

        if let Ok(_) = signer_state.get_expected_string(CHECKED_ADDRESS) {
            signer_state.insert(CHECKED_PUBLIC_KEY, vault_pubkey_value.clone());
            signer_state.insert(CHECKED_ADDRESS, vault_pubkey_string_value.clone());
            signer_state.insert(MULTISIG_ACCOUNT_ADDRESS, Value::string(pubkey.to_string()));
            signer_state.insert(MULTISIG_ACCOUNT_PUBLIC_KEY, Value::string(pubkey.to_string()));
            signer_state.insert(SQUADS_MULTISIG, multisig_value);
            signer_state.insert(INITIATOR, Value::string(initiator_did.to_string()));
            if let Some((payer_did, _)) = &some_payer {
                signer_state.insert(PAYER, Value::string(payer_did.to_string()));
            }
            let update =
                ActionItemRequestUpdate::from_context(&construct_did, ActionItemKey::CheckAddress)
                    .set_status(ActionItemStatus::Success(Some(vault_pubkey.to_string())));
            consolidated_actions.push_action_item_update(update);
        } else {
            action_items.push(
                ReviewInputRequest::new("", &vault_pubkey_string_value)
                    .to_action_type()
                    .to_request(instance_name, ActionItemKey::CheckAddress)
                    .with_construct_did(construct_did)
                    .with_some_description(description)
                    .with_meta_description(&format!(
                        "Check expected vault address for Squads signer named '{}'",
                        instance_name
                    ))
                    .with_some_markdown(markdown),
            );
        }

        match is_balance_checked {
            Some(true) => {
                consolidated_actions.push_action_item_update(
                    ActionItemRequestUpdate::from_context(
                        &construct_did,
                        ActionItemKey::CheckBalance,
                    )
                    .set_status(ActionItemStatus::Success(None)),
                );
            }
            Some(false) => {
                consolidated_actions.push_action_item_update(
                    ActionItemRequestUpdate::from_context(
                        &construct_did,
                        ActionItemKey::CheckBalance,
                    )
                    .set_status(ActionItemStatus::Todo),
                );
            }
            None => {}
        }

        let values = values.clone();
        let construct_did = construct_did.clone();
        let instance_name = instance_name.to_string();
        let signers_instances = signers_instances.clone();
        let supervision_context = supervision_context.clone();
        let auth_ctx = auth_ctx.clone();
        let future = async move {
            use crate::{
                constants::REQUESTED_STARTUP_DATA, signers::get_additional_actions_for_address,
            };

            let is_first_pass = signer_state.get_bool(REQUESTED_STARTUP_DATA).is_none();

            let res = get_additional_actions_for_address(
                &None,
                &Some(vault_pubkey),
                &construct_did,
                &instance_name,
                None,
                None,
                &network_id,
                &rpc_api_url,
                false,
                is_balance_check_required,
                false,
                is_balance_checked,
            )
            .await;
            signer_state.insert(&REQUESTED_STARTUP_DATA, Value::bool(true));
            let additional_actions =
                &mut res.map_err(|diag| (signers.clone(), signer_state.clone(), diag))?;
            action_items.append(additional_actions);
            consolidated_actions.push_group(
                "Review the following Squads Signer related action items",
                action_items,
            );

            let initiator_signer_state = signers.pop_signer_state(&initiator_did).unwrap();
            let initiator_has_requested_startup_data =
                initiator_signer_state.get_bool(REQUESTED_STARTUP_DATA).unwrap_or(false);

            // if this is the first time we are checking this squad signer, but the initiator has already requested startup data,
            // then the initiator is being used by some other actions and we don't need to check activability again
            if is_first_pass && !initiator_has_requested_startup_data {
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
            } else {
                signers.push_signer_state(initiator_signer_state);
            }

            if let Some((payer_did, payer_instance)) = some_payer {
                let payer_signer_state = signers.pop_signer_state(&payer_did).unwrap();
                let payer_has_requested_startup_data =
                    payer_signer_state.get_bool(REQUESTED_STARTUP_DATA).unwrap_or(false);

                if is_first_pass && !payer_has_requested_startup_data {
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
                } else {
                    signers.push_signer_state(payer_signer_state);
                }
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
            signer_state.get_value(SQUADS_MULTISIG).map(SquadsMultisig::from_value).unwrap();

        result.outputs.insert(
            VAULT_PUBLIC_KEY.into(),
            SvmValue::pubkey(multisig.vault_pda.to_bytes().to_vec()),
        );
        result.outputs.insert(VAULT_ADDRESS.into(), Value::string(multisig.vault_pda.to_string()));

        result.outputs.insert(
            MULTISIG_ACCOUNT_ADDRESS.into(),
            Value::string(multisig.multisig_pda.to_string()),
        );
        result.outputs.insert(
            MULTISIG_ACCOUNT_PUBLIC_KEY.into(),
            SvmValue::pubkey(multisig.multisig_pda.to_bytes().to_vec()),
        );
        result.outputs.insert(ADDRESS.into(), address.clone());
        result.outputs.insert(PUBLIC_KEY.into(), public_key.clone());

        return_synchronous_result(Ok((signers, signer_state, result)))
    }

    fn check_signability(
        construct_did: &ConstructDid,
        instance_name: &str,
        description: &Option<String>,
        meta_description: &Option<String>,
        markdown: &Option<String>,
        payload: &Value,
        _spec: &SignerSpecification,
        values: &ValueStore,
        mut signer_state: ValueStore,
        mut signers: SignersState,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        supervision_context: &RunbookSupervisionContext,
        auth_ctx: &AuthorizationContext,
    ) -> Result<CheckSignabilityOk, SignerActionErr> {
        signer_state.insert_scoped_value(
            &construct_did.to_string(),
            TRANSACTION_BYTES,
            payload.clone(),
        );

        let third_party_signature_status = signer_state
            .get_scoped_value(&construct_did.to_string(), RunbookKey::ThirdPartySignatureStatus.as_ref())
            .and_then(|v| v.as_third_party_signature_status());

        let rpc_api_url = values
            .get_expected_string(RPC_API_URL)
            .map_err(|e| (signers.clone(), signer_state.clone(), e))?
            .to_string();

        let rpc_client = RpcClient::new(rpc_api_url);

        // The squads signer will have multiple passes through `check_signability` and `sign`. The enum variants are
        // ordered in accordance with the pass we're making through this function
        match third_party_signature_status {
            // The first pass will yield `None` for the third party signature status.
            // This means we still need to create the squads proposal.
            // Creating the squads proposal requires us to build the transaction and pass the details onto the
            // initiator and payer signers so that they can return the associated action items
            None => {
                let mut multisig = signer_state
                    .get_value(SQUADS_MULTISIG)
                    .map(SquadsMultisig::from_value)
                    .unwrap();

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

                let inner_transaction = {
                    let mut transaction: Transaction =
                        build_transaction_from_svm_value(&payload)
                            .map_err(|e| (signers.clone(), signer_state.clone(), e))?;

                    transaction.message.recent_blockhash = blockhash;
                    transaction
                };

                let (create_proposal_transaction, formatted_transaction) = multisig
                    .get_transaction(
                        rpc_client,
                        construct_did,
                        &initiator_pubkey,
                        &rent_payer_pubkey,
                        inner_transaction.message,
                    )
                    .map_err(|e| (signers.clone(), signer_state.clone(), e))?;

                // getting the transaction mutated multisig state, so updated it in our signer state
                signer_state.insert(SQUADS_MULTISIG, multisig.to_value());

                // update the initiator signer state with the transaction to sign
                {
                    let mut initiator_signer_state =
                        signers.pop_signer_state(&initiator_did).unwrap();
                    initiator_signer_state.insert_scoped_value(
                        &construct_did.to_string(),
                        TRANSACTION_BYTES,
                        create_proposal_transaction.clone(),
                    );
                    signers.push_signer_state(initiator_signer_state);

                    if let Some((payer_did, _)) = &some_payer {
                        let mut payer_signer_state = signers.pop_signer_state(&payer_did).unwrap();

                        payer_signer_state.insert_scoped_value(
                            &construct_did.to_string(),
                            TRANSACTION_BYTES,
                            create_proposal_transaction.clone(),
                        );
                        signers.push_signer_state(payer_signer_state);
                    }
                }

                let mut signers_dids = vec![initiator_did.clone()];
                if let Some((payer_did, _)) = &some_payer {
                    signers_dids.push(payer_did.clone());
                }

                signers.push_signer_state(signer_state);

                // add additional context to the value store that the initiator will used for signing,
                // including the transaction bytes, formatted transaction, and the signers that will be involved
                let values = {
                    let mut values = values.clone();
                    values.insert(
                        DocumentationKey::MetaDescription.as_ref(),
                        Value::string(get_formatted_transaction_meta_description(
                            &vec!["This transaction will create a Squads proposal.".into()],
                            &signers_dids,
                            signers_instances,
                        )),
                    );
                    values.insert(FORMATTED_TRANSACTION, formatted_transaction);
                    values.insert(TRANSACTION_BYTES, create_proposal_transaction);
                    values.insert(
                        SIGNERS,
                        Value::array(
                            signers_dids.iter().map(|d| Value::string(d.to_string())).collect(),
                        ),
                    );

                    values
                };

                let (updated_signers, signer_state, consolidated_actions) =
                    check_signed_executability(
                        construct_did,
                        instance_name,
                        &values,
                        supervision_context,
                        signers_instances,
                        signers,
                        auth_ctx,
                    )?;
                signers = updated_signers;
                return Ok((signers, signer_state, consolidated_actions));
            }
            // Step two: now the proposal has been created by the initiator/payer, so the user needs to actually
            // sign the squads proposal. The `sign` function, upon sending the transaction to create the proposal,
            // has inserted the `ThirdPartySignatureStatus::Initialized` status, so we know to return an action item
            // for the user to go through this flow.
            Some(ThirdPartySignatureStatus::Initialized) => {
                let construct_did_str = &construct_did.to_string();

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

                let multisig = signer_state
                    .get_value(SQUADS_MULTISIG)
                    .map(SquadsMultisig::from_value)
                    .unwrap();

                let formatted_payload =
                    signer_state.get_scoped_value(&construct_did_str, FORMATTED_TRANSACTION);

                let request = VerifyThirdPartySignatureRequest::new(
                    &signer_state.uuid,
                    &multisig.vault_transaction_url(&construct_did),
                    &instance_name,
                    "Squads",
                    payload,
                    NAMESPACE,
                    &network_id,
                )
                .check_expectation_action_uuid(construct_did)
                .formatted_payload(formatted_payload)
                .to_action_type()
                .to_request(instance_name, ActionItemKey::ProvideSignedSquadTransaction)
                .with_construct_did(construct_did)
                .with_some_description(description.clone())
                .with_some_meta_description(meta_description.clone())
                .with_some_markdown(markdown.clone())
                .with_status(status);

                let actions = Actions::append_item(
                    request,
                    Some("Review and sign the transactions from the list below"),
                    Some("Transaction Signing"),
                );
                return Ok((signers, signer_state, actions));
            }
            // Step 3: When the ThirdPartySignatureStatus is Submitted, we just need to maintain that the VerifyThirdPartySignature action
            // is still "Todo"
            Some(ThirdPartySignatureStatus::Submitted) => {
                let mut actions = Actions::none();
                actions.push_action_item_update(
                    ActionItemRequestUpdate::from_context(
                        &construct_did,
                        txtx_addon_kit::constants::ActionItemKey::ProvideSignedSquadTransaction,
                    )
                    .set_status(ActionItemStatus::Todo),
                );
                return Ok((signers, signer_state, actions));
            }
            // Step 4: The `CheckRequested` status indicates that the user has clicked the button to check if the signature is complete.
            // This means we need to check the status of the proposal and update the action accordingly.
            Some(ThirdPartySignatureStatus::CheckRequested) => {
                // fetch the proposal status from the Squads multisig
                let proposal_status = {
                    let multisig = signer_state
                        .get_value(SQUADS_MULTISIG)
                        .map(SquadsMultisig::from_value)
                        .unwrap();

                    let rpc_api_url = values
                        .get_expected_string(RPC_API_URL)
                        .map_err(|e| (signers.clone(), signer_state.clone(), e))?
                        .to_string();
                    let rpc_client = RpcClient::new(rpc_api_url);

                    multisig.get_proposal_status(&rpc_client, construct_did).map_err(|e| {
                        (
                            signers.clone(),
                            signer_state.clone(),
                            diagnosed_error!("failed to get proposal status: {e}"),
                        )
                    })?
                };

                match proposal_status {
                    // if Rejected or Cancelled, we can return errors
                    ProposalStatus::Rejected { .. } => {
                        return Err((
                            signers.clone(),
                            signer_state.clone(),
                            diagnosed_error!("Proposal rejected"),
                        ))
                    }
                    ProposalStatus::Cancelled { .. } => {
                        return Err((
                            signers.clone(),
                            signer_state.clone(),
                            diagnosed_error!("Proposal cancelled"),
                        ))
                    }
                    // If Executed, we can update the action item's status to Success, allowing the user to proceed.
                    ProposalStatus::Executed { .. } => {
                        let mut actions = Actions::none();
                        actions.push_action_item_update(
                            ActionItemRequestUpdate::from_context(
                                &construct_did,
                                txtx_addon_kit::constants::ActionItemKey::ProvideSignedSquadTransaction,
                            )
                            .set_status(ActionItemStatus::Success(None)),
                        );
                        return Ok((signers, signer_state, actions));
                    }
                    // Any other statuses will just keep the action as Todo, not returning an error or allowing them to proceed
                    _ => {
                        let mut actions = Actions::none();
                        actions.push_action_item_update(
                            ActionItemRequestUpdate::from_context(
                                &construct_did,
                                txtx_addon_kit::constants::ActionItemKey::ProvideSignedSquadTransaction,
                            )
                            .set_status(ActionItemStatus::Todo),
                        );
                        return Ok((signers, signer_state, actions));
                    }
                }
            }
            // If the third-party signature is approved or rejected, we can proceed without errors or action
            Some(ThirdPartySignatureStatus::Approved)
            | Some(ThirdPartySignatureStatus::Rejected) => {
                return Ok((signers, signer_state, Actions::none()));
            }
        };
    }

    fn sign(
        construct_did: &ConstructDid,
        _title: &str,
        _payload: &Value,
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
                signer_state.get_value(SQUADS_MULTISIG).map(SquadsMultisig::from_value).unwrap();

            let rpc_api_url = values
                .get_expected_string(RPC_API_URL)
                .map_err(|e| (signers.clone(), signer_state.clone(), e))?
                .to_string();

            let rpc_client = RpcClient::new(rpc_api_url);

            let third_party_signature_status = signer_state
                .get_scoped_value(&construct_did.to_string(), RunbookKey::ThirdPartySignatureStatus.as_ref())
                .and_then(|v| v.as_third_party_signature_status());

            // The squads signer will have multiple passes through `check_signability` and `sign`. The enum variants are
            // ordered in accordance with the pass we're making through this function
            match third_party_signature_status {
                // Step 1: if there is no third party signature status yet, but we've made it to this `sign` function, it means
                // the `check_signability` function has passed on actions to the initiator/payer to create the proposal. Our
                // signer states should have the signed transaction bytes, and here we need to call _those_ signer's `sign`
                // functions to allow the transaction to be signed/broadcasted. At this point, we can mark the third party
                // signature status as initialized
                None => {
                    // add the involved signers to the signer store so the the `run_signed_execution` function can route
                    // to the appropriate signers
                    let values = {
                        let ((initiator_did, _), some_payer) =
                            get_signer_states(&signer_state, &signers_instances)
                                .map_err(|e| (signers.clone(), signer_state.clone(), e))?;

                        let mut signers_dids = vec![Value::string(initiator_did.to_string())];
                        if let Some((payer_did, _)) = &some_payer {
                            signers_dids.push(Value::string(payer_did.to_string()));
                        }
                        let mut values = values.clone();
                        values.insert(SIGNERS, Value::array(signers_dids));
                        values.insert(IS_DEPLOYMENT, Value::bool(false));
                        values
                    };

                    signer_state.insert_scoped_value(
                        &construct_did.to_string(),
                        RunbookKey::ThirdPartySignatureStatus.as_ref(),
                        Value::third_party_signature_initialized(),
                    );
                    signers.push_signer_state(signer_state);

                    let run_signing_future =
                        run_signed_execution(&construct_did, &values, &signers_instances, signers);
                    let (signers, signer_state, mut result) = match run_signing_future {
                        Ok(future) => match future.await {
                            Ok(res) => res,
                            Err(err) => return Err(err),
                        },
                        Err(err) => return Err(err),
                    };

                    result.insert(
                        RunbookKey::ThirdPartySignatureStatus.as_ref(),
                        Value::third_party_signature_initialized(),
                    );

                    Ok((signers, signer_state, result))
                }
                // Step 2: if the third party signature status is check requested, we need to return that status
                // so the upstream can route accordingly
                Some(ThirdPartySignatureStatus::CheckRequested) => {
                    return Ok((
                        signers,
                        signer_state,
                        CommandExecutionResult::from([
                            (
                                RunbookKey::ThirdPartySignatureStatus.as_ref(),
                                Value::third_party_signature_check_requested(),
                            ),
                            // (SIGNED_TRANSACTION_BYTES, Value::null()),
                            // (SIGNATURE, Value::string(Signature::default().to_string())),
                        ]),
                    ));
                }
                // Step 3: if the third party signature status is approved, we can fetch the signature of the
                // transaction that approved the proposal and return that in our execution results
                Some(ThirdPartySignatureStatus::Approved) => {
                    let signature = multisig
                        .get_executed_signature(&rpc_client)
                        .unwrap_or(Signature::default().to_string());
                    return Ok((
                        signers,
                        signer_state,
                        CommandExecutionResult::from([
                            (RunbookKey::ThirdPartySignatureStatus.as_ref(), Value::third_party_signature_approved()),
                            (SignerKey::SignedTransactionBytes.as_ref(), Value::null()),
                            (SIGNATURE, Value::string(signature)),
                        ]),
                    ));
                }
                // Other states should be returning action items or errors in the `check_signability` function,
                // so they shouldn't make it here
                Some(_) => {
                    unreachable!()
                }
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
        .get_expected_string(INITIATOR)
        .map(|uuid| ConstructDid::from_hex_string(uuid))
        .map_err(|e| e)?;
    let initiator_signer_instance = signers_instances
        .get(&initiator_uuid)
        .ok_or_else(|| diagnosed_error!("Squads signer initiator not found"))?;

    let payer = if let Some(payer_uuid) =
        values.get_string(PAYER).map(|uuid| ConstructDid::from_hex_string(uuid))
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
