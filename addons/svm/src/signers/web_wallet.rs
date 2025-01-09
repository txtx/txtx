use std::collections::HashMap;

use txtx_addon_kit::channel;
use txtx_addon_kit::constants::{SIGNATURE_SKIPPABLE, SIGNED_TRANSACTION_BYTES};
use txtx_addon_kit::types::commands::CommandExecutionResult;
use txtx_addon_kit::types::frontend::{
    ActionItemRequest, ActionItemRequestUpdate, ActionItemStatus, Actions, BlockEvent,
    ProvideSignedTransactionRequest,
};
use txtx_addon_kit::types::signers::{
    return_synchronous_actions, return_synchronous_result, CheckSignabilityOk, SignerActionErr,
    SignerActionsFutureResult, SignerActivateFutureResult, SignerImplementation, SignerInstance,
    SignerSignFutureResult, SignerSpecification, SignersState,
};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::RunbookSupervisionContext;
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::types::{
    commands::CommandSpecification,
    diagnostics::Diagnostic,
    types::{Type, Value},
};

use crate::constants::{
    ACTION_ITEM_CHECK_ADDRESS, ACTION_ITEM_PROVIDE_PUBLIC_KEY,
    ACTION_ITEM_PROVIDE_SIGNED_TRANSACTION, CHECKED_ADDRESS, CHECKED_PUBLIC_KEY, EXPECTED_ADDRESS,
    FORMATTED_TRANSACTION, IS_SIGNABLE, NAMESPACE, NETWORK_ID, REQUESTED_STARTUP_DATA, RPC_API_URL,
};

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
                        documentation: "The address of the account.",
                        typing: Type::array(Type::string())
                    }
                ],
                example: txtx_addon_kit::indoc! {r#"
                signer "alice" "svm::web_wallet" {
                    expected_address = "zbBjhHwuqyKMmz8ber5oUtJJ3ZV4B6ePmANfGyKzVGV"
                }
                "#},
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
        _auth_ctx: &txtx_addon_kit::types::AuthorizationContext,
        is_balance_check_required: bool,
        is_public_key_required: bool,
    ) -> SignerActionsFutureResult {
        use txtx_addon_kit::constants::PROVIDE_PUBLIC_KEY_ACTION_RESULT;

        use crate::{codec::public_key_from_str, constants::NETWORK_ID};

        let checked_public_key = signer_state.get_expected_string(CHECKED_PUBLIC_KEY);

        let values = values.clone();
        let expected_address = values
            .get_string(EXPECTED_ADDRESS)
            .map(|a| {
                public_key_from_str(&a).map_err(|e| (signers.clone(), signer_state.clone(), e))
            })
            .transpose()?;
        let do_request_address_check = expected_address.is_some();
        let do_request_public_key = is_public_key_required;

        let _is_nonce_required = true;
        let do_request_balance = is_balance_check_required;

        let instance_name = instance_name.to_string();
        let signer_did = construct_did.clone();
        let rpc_api_url = values
            .get_expected_string(RPC_API_URL)
            .map_err(|e| (signers.clone(), signer_state.clone(), e))?
            .to_owned();

        let network_id = values
            .get_expected_string(NETWORK_ID)
            .map_err(|e| (signers.clone(), signer_state.clone(), e))?
            .to_owned();

        if let Ok(public_key_bytes) = values.get_expected_string(PROVIDE_PUBLIC_KEY_ACTION_RESULT) {
            let mut actions: Actions = Actions::none();
            let mut success = true;

            let sol_address = public_key_from_str(&public_key_bytes)
                .map_err(|e| (signers.clone(), signer_state.clone(), e))?;

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
                        ACTION_ITEM_CHECK_ADDRESS,
                    )
                    .set_status(status_update.clone());
                    actions.push_action_item_update(update);
                }
            }
            if success {
                signer_state.insert(CHECKED_PUBLIC_KEY, Value::string(sol_address.to_string()));
                signer_state.insert(CHECKED_ADDRESS, Value::string(sol_address.to_string()));
            }
            let update =
                ActionItemRequestUpdate::from_context(&signer_did, ACTION_ITEM_PROVIDE_PUBLIC_KEY)
                    .set_status(status_update);
            actions.push_action_item_update(update);

            return return_synchronous_actions(Ok((signers, signer_state, actions)));
        } else if checked_public_key.is_ok() {
            return return_synchronous_actions(Ok((signers, signer_state, Actions::none())));
        }

        let future = async move {
            let mut actions = Actions::none();
            let res = get_additional_actions_for_address(
                &expected_address,
                &signer_did,
                &instance_name,
                &network_id,
                &rpc_api_url,
                do_request_public_key,
                do_request_balance,
                do_request_address_check,
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
        values: &ValueStore,
        mut signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        _progress_tx: &channel::Sender<BlockEvent>,
    ) -> SignerActivateFutureResult {
        let mut result = CommandExecutionResult::new();

        signer_state.insert("multi_sig", Value::bool(false));

        let address = values
            .get_value(EXPECTED_ADDRESS)
            .unwrap_or_else(|| signer_state.get_value(CHECKED_ADDRESS).unwrap());
        result.outputs.insert("address".into(), address.clone());
        return_synchronous_result(Ok((signers, signer_state, result)))
    }

    fn check_signability(
        construct_did: &ConstructDid,
        title: &str,
        description: &Option<String>,
        payload: &Value,
        _spec: &SignerSpecification,
        values: &ValueStore,
        signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        _supervision_context: &RunbookSupervisionContext,
    ) -> Result<CheckSignabilityOk, SignerActionErr> {
        let construct_did_str = &construct_did.to_string();
        if let Some(_) = signer_state.get_scoped_value(&construct_did_str, SIGNED_TRANSACTION_BYTES)
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
            .get_scoped_value(&construct_did_str, SIGNATURE_SKIPPABLE)
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let expected_signer_address = signer_state.get_string(CHECKED_ADDRESS);

        let formatted_payload = signer_state
            .get_scoped_value(&construct_did_str, FORMATTED_TRANSACTION)
            .and_then(|v| v.as_string())
            .and_then(|v| Some(v.to_string()));

        let request = ActionItemRequest::new(
            &Some(construct_did.clone()),
            title,
            description.clone(),
            status,
            ProvideSignedTransactionRequest::new(
                &signer_state.uuid,
                &payload,
                NAMESPACE,
                &network_id,
            )
            .skippable(skippable)
            .expected_signer_address(expected_signer_address)
            .check_expectation_action_uuid(construct_did)
            .formatted_payload(formatted_payload)
            .to_action_type(),
            ACTION_ITEM_PROVIDE_SIGNED_TRANSACTION,
        );
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
        _payload: &Value,
        _spec: &SignerSpecification,
        _values: &ValueStore,
        signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
    ) -> SignerSignFutureResult {
        let mut result = CommandExecutionResult::new();
        let key = construct_did.to_string();
        if let Some(signed_transaction) =
            signer_state.get_scoped_value(&key, SIGNED_TRANSACTION_BYTES)
        {
            result.outputs.insert(SIGNED_TRANSACTION_BYTES.into(), signed_transaction.clone());
        }

        return_synchronous_result(Ok((signers, signer_state, result)))
    }
}
