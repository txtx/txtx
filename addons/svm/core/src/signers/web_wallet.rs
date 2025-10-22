use std::collections::HashMap;

use solana_client::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_signature::Signature;
use solana_transaction::Transaction;
use txtx_addon_kit::channel;
use txtx_addon_kit::constants::{SignerKey};
use txtx_addon_kit::types::commands::CommandExecutionResult;
use txtx_addon_kit::types::frontend::{
    ActionItemRequestUpdate, ActionItemStatus, Actions, BlockEvent, ProvideSignedTransactionRequest,
};
use txtx_addon_kit::types::signers::{
    return_synchronous_result, CheckSignabilityOk, SignerActionErr, SignerActionsFutureResult,
    SignerActivateFutureResult, SignerImplementation, SignerInstance, SignerSignFutureResult,
    SignerSpecification, SignersState,
};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::RunbookSupervisionContext;
use txtx_addon_kit::types::{
    commands::CommandSpecification,
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{AuthorizationContext, ConstructDid};

use crate::codec::{transaction_is_fully_signed, DeploymentTransaction};
use txtx_addon_kit::constants::ActionItemKey;
use crate::constants::{
    ADDRESS, CHECKED_ADDRESS, CHECKED_PUBLIC_KEY,
    EXPECTED_ADDRESS, FORMATTED_TRANSACTION, IS_DEPLOYMENT, IS_SIGNABLE, NAMESPACE, NETWORK_ID,
    PARTIALLY_SIGNED_TRANSACTION_BYTES, PUBLIC_KEY, REQUESTED_STARTUP_DATA, RPC_API_URL,
    TRANSACTION_BYTES, UPDATED_PARTIALLY_SIGNED_TRANSACTION,
};
use crate::typing::SvmValue;
use crate::utils::build_transaction_from_svm_value;

use super::get_additional_actions_for_address;

lazy_static! {
    pub static ref SVM_WEB_WALLET: SignerSpecification = {
        let mut signer = define_signer! {
            SvmWebWallet => {
                name: "SVM Web Wallet Signer",
                matcher: "web_wallet",
                documentation:txtx_addon_kit::indoc! {r#"The `svm::web_wallet` signer will allow a Runbook operator to sign the transaction with the browser signer of their choice."#},
                inputs: [
                    expected_address: {
                        documentation: "The SVM address that is expected to connect to the Runbook execution. Omitting this field will allow any address to be used for this signer.",
                        typing: Type::string(),
                        optional: true,
                        tainting: true,
                        sensitive: true
                    }
                ],
                outputs: [
                    address: {
                        documentation: "The address of the account. This is an alias for the `public_key` output.",
                        typing: Type::string()
                    },
                    public_key: {
                        documentation: "The address of the account.",
                        typing: Type::string()
                    }
                ],
                example: txtx_addon_kit::indoc! {r#"
                    signer "alice" "svm::web_wallet" {
                        expected_address = "zbBjhHwuqyKMmz8ber5oUtJJ3ZV4B6ePmANfGyKzVGV"
                    }
                "#}
            }
        };
        signer.requires_interaction = true;
        signer
    };
}

pub struct SvmWebWallet;
impl SignerImplementation for SvmWebWallet {
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
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        _supervision_context: &RunbookSupervisionContext,
        auth_ctx: &txtx_addon_kit::types::AuthorizationContext,
        _is_balance_check_required: bool,
        _is_public_key_required: bool,
    ) -> SignerActionsFutureResult {
        use crate::{codec::public_key_from_str, constants::NETWORK_ID};
        use txtx_addon_kit::constants::{ActionItemKey, DocumentationKey, SignerKey};

        let checked_public_key = signer_state.get_expected_string(CHECKED_PUBLIC_KEY);
        let is_balance_checked = signer_state.get_bool(SignerKey::IsBalanceChecked);

        let values = values.clone();
        let expected_address = values
            .get_string(EXPECTED_ADDRESS)
            .map(|a| {
                public_key_from_str(&a).map_err(|e| (signers.clone(), signer_state.clone(), e))
            })
            .transpose()?;
        let mut do_request_address_check = expected_address.is_some();
        let mut do_request_public_key = true;

        let _is_nonce_required = true;
        let do_request_balance = true;

        let instance_name = instance_name.to_string();
        let signer_did = construct_did.clone();
        let rpc_api_url = values
            .get_expected_string(RPC_API_URL)
            .map_err(|e| (signers.clone(), signer_state.clone(), e))?
            .to_owned();
        let description = values.get_string(DocumentationKey::Description).map(|d| d.to_string());
        let markdown = values
            .get_markdown(auth_ctx)
            .map_err(|d| (signers.clone(), signer_state.clone(), d))?;

        let network_id = values
            .get_expected_string(NETWORK_ID)
            .map_err(|e| (signers.clone(), signer_state.clone(), e))?
            .to_owned();

        let (mut actions, connected_public_key) = if let Ok(public_key_bytes) =
            values.get_expected_string(ActionItemKey::ProvidePublicKey)
        {
            let sol_address = public_key_from_str(&public_key_bytes)
                .map_err(|e| (signers.clone(), signer_state.clone(), e))?;

            let mut actions: Actions = Actions::none();
            let mut success = true;
            let mut status_update = ActionItemStatus::Success(Some(sol_address.to_string()));
            if let Some(expected_address) = expected_address.as_ref() {
                if !expected_address.eq(&sol_address) {
                    status_update = ActionItemStatus::Error(diagnosed_error!(
                        "expected address {} to connect; got address {}",
                        expected_address,
                        sol_address
                    ));
                    success = false;
                } else {
                    let update = ActionItemRequestUpdate::from_context(
                        &signer_did,
                        ActionItemKey::CheckAddress,
                    )
                    .set_status(status_update.clone());
                    actions.push_action_item_update(update);
                }
            }
            if success {
                signer_state.insert(CHECKED_PUBLIC_KEY, Value::string(sol_address.to_string()));
                signer_state.insert(CHECKED_ADDRESS, Value::string(sol_address.to_string()));
                do_request_address_check = false;
                do_request_public_key = false;
            }
            let update =
                ActionItemRequestUpdate::from_context(&signer_did, ActionItemKey::ProvidePublicKey)
                    .set_status(status_update);
            actions.push_action_item_update(update);

            (actions, Some(sol_address))
        } else if let Ok(checked_public_key) = checked_public_key {
            let sol_address = public_key_from_str(&checked_public_key)
                .map_err(|e| (signers.clone(), signer_state.clone(), e))?;
            signer_state.insert(CHECKED_PUBLIC_KEY, Value::string(sol_address.to_string()));
            signer_state.insert(CHECKED_ADDRESS, Value::string(sol_address.to_string()));
            (Actions::none(), Some(sol_address))
        } else {
            (Actions::none(), None)
        };

        match is_balance_checked {
            Some(true) => {
                actions.push_action_item_update(
                    ActionItemRequestUpdate::from_context(&signer_did, ActionItemKey::CheckBalance)
                        .set_status(ActionItemStatus::Success(None)),
                );
            }
            Some(false) => {
                actions.push_action_item_update(
                    ActionItemRequestUpdate::from_context(&signer_did, ActionItemKey::CheckBalance)
                        .set_status(ActionItemStatus::Todo),
                );
            }
            None => {}
        }

        let future = async move {
            let res = get_additional_actions_for_address(
                &expected_address,
                &connected_public_key,
                &signer_did,
                &instance_name,
                description,
                markdown,
                &network_id,
                &rpc_api_url,
                do_request_public_key,
                do_request_balance,
                do_request_address_check,
                is_balance_checked,
            )
            .await;
            signer_state.insert(&REQUESTED_STARTUP_DATA, Value::bool(true));

            let action_items = match res {
                Ok(action_items) => action_items,
                Err(diag) => return Err((signers, signer_state, diag)),
            };
            if !action_items.is_empty() {
                actions.push_group(
                    "Review and check the following signer related action items",
                    action_items,
                );
            }

            Ok((signers, signer_state, actions))
        };
        Ok(Box::pin(future))
    }

    fn activate(
        _construct_id: &ConstructDid,
        _spec: &SignerSpecification,
        _values: &ValueStore,
        signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        _progress_tx: &channel::Sender<BlockEvent>,
    ) -> SignerActivateFutureResult {
        let mut result = CommandExecutionResult::new();
        let public_key = signer_state
            .get_expected_value(CHECKED_PUBLIC_KEY)
            .map_err(|e| (signers.clone(), signer_state.clone(), e))?;
        let address = signer_state
            .get_expected_value(CHECKED_ADDRESS)
            .map_err(|e| (signers.clone(), signer_state.clone(), e))?;
        result.outputs.insert(ADDRESS.into(), address.clone());
        result.outputs.insert(PUBLIC_KEY.into(), public_key.clone());
        return_synchronous_result(Ok((signers, signer_state, result)))
    }

    fn check_signability(
        construct_did: &ConstructDid,
        title: &str,
        description: &Option<String>,
        meta_description: &Option<String>,
        markdown: &Option<String>,
        payload: &Value,
        _spec: &SignerSpecification,
        values: &ValueStore,
        mut signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        _supervision_context: &RunbookSupervisionContext,
        _auth_ctx: &AuthorizationContext,
    ) -> Result<CheckSignabilityOk, SignerActionErr> {
        let construct_did_str = &construct_did.to_string();
        signer_state.insert_scoped_value(&construct_did_str, TRANSACTION_BYTES, payload.clone());
        if let Some(_) = signer_state.get_scoped_value(&construct_did_str, SignerKey::SignedTransactionBytes)
        {
            return Ok((signers, signer_state, Actions::none()));
        }

        let network_id = values
            .get_expected_string(NETWORK_ID)
            .map_err(|e| (signers.clone(), signer_state.clone(), e))?;

        let signable = signer_state
            .get_scoped_value(&construct_did_str, IS_SIGNABLE)
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let status = match signable {
            true => ActionItemStatus::Todo,
            false => ActionItemStatus::Blocked,
        };

        let skippable = signer_state
            .get_scoped_value(&construct_did_str, SignerKey::SignatureSkippable)
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let expected_signer_address = signer_state.get_string(CHECKED_ADDRESS);

        let formatted_payload =
            signer_state.get_scoped_value(&construct_did_str, FORMATTED_TRANSACTION);

        let request = ProvideSignedTransactionRequest::new(
            &signer_state.uuid,
            &payload,
            NAMESPACE,
            &network_id,
        )
        .skippable(skippable)
        .expected_signer_address(expected_signer_address)
        .check_expectation_action_uuid(construct_did)
        .formatted_payload(formatted_payload)
        .to_action_type()
        .to_request(title, ActionItemKey::ProvideSignedTransaction)
        .with_construct_did(construct_did)
        .with_some_description(description.clone())
        .with_some_meta_description(meta_description.clone())
        .with_some_markdown(markdown.clone())
        .with_status(status);

        Ok((
            signers,
            signer_state,
            Actions::append_item(
                request,
                Some("Review and sign the transactions from the list below"),
                Some("Transaction Signing"),
            ),
        ))
    }

    fn sign(
        construct_did: &ConstructDid,
        _title: &str,
        payload: &Value,
        _spec: &SignerSpecification,
        values: &ValueStore,
        mut signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
    ) -> SignerSignFutureResult {
        let mut result = CommandExecutionResult::new();

        // value signed (partially, maybe) by the supervisor
        let signed_transaction_value =
            signer_state.remove_scoped_value(&construct_did.to_string(), SignerKey::SignedTransactionBytes);

        let supervisor_signed_tx = if let Some(signed_transaction_value) = signed_transaction_value
        {
            let transaction =
                build_transaction_from_svm_value(&signed_transaction_value).map_err(|e| {
                    (
                        signers.clone(),
                        signer_state.clone(),
                        diagnosed_error!("failed to deserialize signed transaction: {e}"),
                    )
                })?;

            let is_fully_signed = transaction_is_fully_signed(&transaction);

            if is_fully_signed {
                // the supervisor has fully signed the transaction, and there's a chance the web wallet has
                // added some instructions to the transaction, so we'll call this one "updated" so that the
                // upstream commands can handle accordingly
                result.outputs.insert(
                    UPDATED_PARTIALLY_SIGNED_TRANSACTION.into(),
                    signed_transaction_value.clone(),
                );
                return return_synchronous_result(Ok((signers, signer_state, result)));
            }
            Some(transaction)
        } else {
            None
        };

        // there's a chance the user sat for a while with the transaction unsigned in the supervisor,
        // so the supervisor updates the blockhash before signing. we'll use this blockhash for the transaction
        // if it's available
        let blockhash = if let Some(transaction) = &supervisor_signed_tx {
            transaction.message.recent_blockhash.clone()
        } else {
            let rpc_api_url = values
                .get_expected_string(RPC_API_URL)
                .map_err(|diag| (signers.clone(), signer_state.clone(), diag))?
                .to_string();

            let rpc_client =
                RpcClient::new_with_commitment(rpc_api_url.clone(), CommitmentConfig::processed());

            let blockhash = rpc_client.get_latest_blockhash().map_err(|e| {
                (
                    signers.clone(),
                    signer_state.clone(),
                    diagnosed_error!("failed to get latest blockhash: {e}"),
                )
            })?;
            blockhash
        };

        let mut did_update_transaction = false;
        let is_deployment = values.get_bool(IS_DEPLOYMENT).unwrap_or(false);
        let (mut transaction, do_sign_with_txtx_signer) = if is_deployment {
            let deployment_transaction = DeploymentTransaction::from_value(&payload)
                .map_err(|e| (signers.clone(), signer_state.clone(), e))?;
            // the supervisor may have changed the transaction some (specifically by adding compute budget instructions),
            // so we want to sign that new transaction
            let mut transaction = if let Some(supervisor_signed_tx) = &supervisor_signed_tx {
                did_update_transaction = true;
                supervisor_signed_tx.clone()
            } else {
                deployment_transaction.transaction.as_ref().unwrap().clone()
            };
            transaction.message.recent_blockhash = blockhash;

            let keypairs = deployment_transaction
                .get_keypairs()
                .map_err(|e| (signers.clone(), signer_state.clone(), e))?;

            transaction
                .try_partial_sign(&keypairs, transaction.message.recent_blockhash)
                .map_err(|e| (signers.clone(), signer_state.clone(), e.to_string().into()))?;
            (transaction, deployment_transaction.signers.is_some())
        } else {
            let mut transaction: Transaction =
                if let Some(supervisor_signed_tx) = &supervisor_signed_tx {
                    did_update_transaction = true;
                    supervisor_signed_tx.clone()
                } else {
                    build_transaction_from_svm_value(&payload)
                        .map_err(|e| (signers.clone(), signer_state.clone(), e))?
                };

            transaction.message.recent_blockhash = blockhash;

            (transaction, true)
        };

        if do_sign_with_txtx_signer {
            if let Some(supervisor_signed_tx) = &supervisor_signed_tx {
                for (i, sig) in supervisor_signed_tx.signatures.iter().enumerate() {
                    if sig != &Signature::default() {
                        transaction.signatures[i] = sig.clone();
                    }
                }
            } else {
                return Err((
                    signers,
                    signer_state,
                    diagnosed_error!("internal error: web wallet has not signed the transaction"),
                ));
            }
        }

        if did_update_transaction {
            result.outputs.insert(
                UPDATED_PARTIALLY_SIGNED_TRANSACTION.into(),
                SvmValue::transaction(&transaction)
                    .map_err(|e| (signers.clone(), signer_state.clone(), e))?,
            );
        } else {
            result.outputs.insert(
                PARTIALLY_SIGNED_TRANSACTION_BYTES.into(),
                SvmValue::transaction(&transaction)
                    .map_err(|e| (signers.clone(), signer_state.clone(), e))?,
            );
        }

        return_synchronous_result(Ok((signers, signer_state, result)))
    }
}
