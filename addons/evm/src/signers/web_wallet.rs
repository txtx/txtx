use std::collections::HashMap;

use txtx_addon_kit::channel;
use txtx_addon_kit::constants::TX_HASH;
use txtx_addon_kit::types::commands::CommandExecutionResult;
use txtx_addon_kit::types::frontend::{
    ActionItemRequest, ActionItemRequestUpdate, ActionItemStatus, Actions, BlockEvent,
    ReviewInputRequest, SendTransactionRequest,
};
use txtx_addon_kit::types::signers::{
    return_synchronous_actions, return_synchronous_result, CheckSignabilityOk, SignerActionErr,
    SignerActionsFutureResult, SignerActivateFutureResult, SignerImplementation, SignerInstance,
    SignerSignFutureResult, SignerSpecification, SignersState,
};
use txtx_addon_kit::types::signers::{signer_diag_with_ctx, signer_err_fn};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::RunbookSupervisionContext;
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::types::{
    commands::CommandSpecification,
    diagnostics::Diagnostic,
    types::{Type, Value},
};

use crate::constants::{
    ACTION_ITEM_CHECK_ADDRESS, ACTION_ITEM_PROVIDE_PUBLIC_KEY, ACTION_ITEM_SEND_TRANSACTION,
    ALREADY_DEPLOYED, CHAIN_ID, CHECKED_ADDRESS, CHECKED_COST_PROVISION, CHECKED_PUBLIC_KEY,
    CONTRACT_ADDRESS, EXPECTED_ADDRESS, FETCHED_BALANCE, FETCHED_NONCE, FORMATTED_TRANSACTION,
    NAMESPACE, PUBLIC_KEYS, REQUESTED_STARTUP_DATA, RPC_API_URL,
    WEB_WALLET_UNSIGNED_TRANSACTION_BYTES,
};

use super::namespaced_err_fn;

lazy_static! {
    pub static ref EVM_WEB_WALLET: SignerSpecification = {
        let mut signer = define_signer! {
            EvmWebWallet => {
                name: "Stacks Web Wallet",
                matcher: "web_wallet",
                documentation:txtx_addon_kit::indoc! {r#"The `web_wallet` signer will route the transaction signing process through [wagmi](https://wagmi.sh/).
                This allows a Runbook operator to sign the transaction with the browser signer of their choice."#},
                inputs: [
                    expected_address: {
                        documentation: "The EVM address that is expected to connect to the Runbook execution. Omitting this field will allow any address to be used for this signer.",
                        typing: Type::string(),
                        optional: false,
                        tainting: true,
                        sensitive: true
                    }
                ],
                outputs: [
                    address: {
                        documentation: "The address of the account generated from the public key.",
                        typing: Type::array(Type::string())
                    }
                ],
                example: txtx_addon_kit::indoc! {r#"
                signer "alice" "evm::web_wallet" {
                    expected_address = "0xCe246168E59dd8e28e367BB49b38Dc621768F425"
                }
                "#},
            }
        };
        signer.requires_interaction = true;
        signer
    };
}

pub struct EvmWebWallet;
impl SignerImplementation for EvmWebWallet {
    fn check_instantiability(
        _ctx: &SignerSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    // check_activability analyses the signer constructs.
    // it will returns all the ActionItemRequests required for a given signer, which includes:
    // - ProvidePublicKey:
    // - ReviewInput (StacksAddress): Most of the case, unknown the first time it's being executed unless expected_address is provided in the construct
    // - ReviewInput (StacksBalance):
    // - ReviewInput (Assosiated Costs):
    // If the all of the informations above are present in the signer state, nothing is returned.
    #[cfg(not(feature = "wasm"))]
    fn check_activability(
        construct_did: &ConstructDid,
        instance_name: &str,
        spec: &SignerSpecification,
        values: &ValueStore,
        mut signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        _supervision_context: &RunbookSupervisionContext,
        is_balance_check_required: bool,
        is_public_key_required: bool,
    ) -> SignerActionsFutureResult {
        use txtx_addon_kit::constants::PROVIDE_PUBLIC_KEY_ACTION_RESULT;

        use crate::{
            codec::{
                crypto::{public_key_from_signed_message, public_key_to_address},
                string_to_address,
            },
            constants::DEFAULT_MESSAGE,
            signers::common::get_additional_actions_for_address,
        };

        let signer_err =
            signer_err_fn(signer_diag_with_ctx(spec, &instance_name, namespaced_err_fn()));

        let checked_public_key = signer_state.get_expected_string(CHECKED_PUBLIC_KEY);
        let _requested_startup_data =
            signer_state.get_expected_bool(REQUESTED_STARTUP_DATA).ok().unwrap_or(false);
        let _checked_address = signer_state.get_expected_string(CHECKED_ADDRESS);
        let _checked_cost_provision = signer_state.get_expected_integer(CHECKED_COST_PROVISION);
        let _fetched_nonce = signer_state.get_expected_integer(FETCHED_NONCE);
        let _fetched_balance = signer_state.get_expected_integer(FETCHED_BALANCE);

        let values = values.clone();
        let chain_id = values
            .get_expected_uint(CHAIN_ID)
            .map_err(|e| signer_err(&signers, &signer_state, e.message))?;
        let expected_address = values
            .get_string(EXPECTED_ADDRESS)
            .map(|e| e.to_string())
            .and_then(|a| Some(string_to_address(a)))
            .transpose()
            .map_err(|e| signer_err(&signers, &signer_state, e))?;
        let do_request_address_check = expected_address.is_some();
        let do_request_public_key = is_public_key_required;
        // only request public key if we haven't already created that action

        let _is_nonce_required = true;
        let do_request_balance = is_balance_check_required;

        let instance_name = instance_name.to_string();
        let signer_did = construct_did.clone();
        let rpc_api_url = values
            .get_expected_string(RPC_API_URL)
            .map_err(|e| signer_err(&signers, &signer_state, e.message))?
            .to_owned();

        if let Ok(ref signed_message_hex) =
            values.get_expected_string(PROVIDE_PUBLIC_KEY_ACTION_RESULT)
        {
            let public_key_bytes =
                public_key_from_signed_message(&DEFAULT_MESSAGE, signed_message_hex)
                    .map_err(|e| signer_err(&signers, &signer_state, e))?;
            let evm_address = public_key_to_address(&public_key_bytes)
                .map_err(|e| signer_err(&signers, &signer_state, e))?;

            let mut actions: Actions = Actions::none();
            let mut success = true;
            let mut status_update = ActionItemStatus::Success(Some(evm_address.to_string()));
            if let Some(expected_address) = expected_address {
                if !expected_address.eq(&evm_address) {
                    status_update = ActionItemStatus::Error(diagnosed_error!(
                        "Signer '{}': expected {} got {}",
                        instance_name,
                        expected_address,
                        evm_address
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
                signer_state.insert(
                    CHECKED_PUBLIC_KEY,
                    Value::string(txtx_addon_kit::hex::encode(public_key_bytes)),
                );
                signer_state.insert(CHECKED_ADDRESS, Value::string(evm_address.to_string()));
                signer_state.insert("signer_address", Value::string(evm_address.to_string()));
            }
            let update =
                ActionItemRequestUpdate::from_context(&signer_did, ACTION_ITEM_PROVIDE_PUBLIC_KEY)
                    .set_status(status_update);
            actions.push_action_item_update(update);

            return return_synchronous_actions(Ok((signers, signer_state, actions)));
        } else if checked_public_key.is_ok() {
            return return_synchronous_actions(Ok((signers, signer_state, Actions::none())));
        }

        let spec = spec.clone();
        let future = async move {
            let signer_err =
                signer_err_fn(signer_diag_with_ctx(&spec, &instance_name, namespaced_err_fn()));
            let mut actions = Actions::none();
            let res = get_additional_actions_for_address(
                &expected_address,
                &signer_did,
                &instance_name,
                &rpc_api_url,
                chain_id,
                do_request_public_key,
                do_request_balance,
                do_request_address_check,
            )
            .await;
            signer_state.insert(&REQUESTED_STARTUP_DATA, Value::bool(true));

            let action_items = match res {
                Ok(action_items) => action_items,
                Err(e) => return Err(signer_err(&signers, &signer_state, e)),
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
        spec: &SignerSpecification,
        values: &ValueStore,
        mut signer_state: ValueStore,
        signers: SignersState,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        _progress_tx: &channel::Sender<BlockEvent>,
    ) -> SignerActivateFutureResult {
        let signer_did = ConstructDid(signer_state.uuid.clone());
        let signer_instance = signers_instances.get(&signer_did).unwrap();
        let signer_err =
            signer_err_fn(signer_diag_with_ctx(spec, &signer_instance.name, namespaced_err_fn()));

        let mut result = CommandExecutionResult::new();
        let public_key = signer_state
            .get_expected_value(CHECKED_PUBLIC_KEY)
            .map_err(|e| signer_err(&signers, &signer_state, e.message))?;
        let chain_id = values
            .get_expected_value(CHAIN_ID)
            .map_err(|e| signer_err(&signers, &signer_state, e.message))?;

        signer_state.insert(PUBLIC_KEYS, Value::array(vec![public_key.clone()]));
        signer_state.insert("multi_sig", Value::bool(false));

        let address = signer_state.get_value(CHECKED_ADDRESS).unwrap();
        result.outputs.insert("address".into(), address.clone());
        result.outputs.insert(CHAIN_ID.into(), chain_id.clone());
        return_synchronous_result(Ok((signers, signer_state, result)))
    }

    fn check_signability(
        construct_did: &ConstructDid,
        title: &str,
        description: &Option<String>,
        _payload: &Value,
        spec: &SignerSpecification,
        values: &ValueStore,
        mut signer_state: ValueStore,
        signers: SignersState,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        _supervision_context: &RunbookSupervisionContext,
    ) -> Result<CheckSignabilityOk, SignerActionErr> {
        let signer_did = ConstructDid(signer_state.uuid.clone());
        let signer_instance = signers_instances.get(&signer_did).unwrap();
        let signer_err =
            signer_err_fn(signer_diag_with_ctx(spec, &signer_instance.name, namespaced_err_fn()));

        let construct_did_str = &construct_did.to_string();
        if let Some(_) = signer_state.get_scoped_value(&construct_did_str, TX_HASH) {
            println!("already?");
            return Ok((signers, signer_state, Actions::none()));
        }

        let already_deployed = signer_state
            .get_scoped_bool(&construct_did.to_string(), ALREADY_DEPLOYED)
            .unwrap_or(false);
        // the tx hash won't actually be used in the path where the contract is already deployed, but we need
        // this value set in order to prevent re-adding the same action item every time we get to this fn
        signer_state.insert_scoped_value(&construct_did_str, TX_HASH, Value::null());
        let actions = if already_deployed {
            let contract_address = signer_state
                .get_scoped_value(&construct_did.to_string(), CONTRACT_ADDRESS)
                .unwrap();
            let request = ActionItemRequest::new(
                &Some(construct_did.clone()),
                title,
                description.clone(),
                ActionItemStatus::Success(None),
                ReviewInputRequest::new("", contract_address).to_action_type(),
                "action_item_review_deployed_contract",
            );
            Actions::append_item(
                request,
                Some("The following contract has already been deployed"),
                Some("Transaction Execution"),
            )
        } else {
            let chain_id = values
                .get_expected_uint(CHAIN_ID)
                .map_err(|e| signer_err(&signers, &signer_state, e.message))?;
            // let signable = signer_state
            //     .get_scoped_value(&construct_did_str, IS_SIGNABLE)
            //     .and_then(|v| v.as_bool())
            //     .unwrap_or(true);

            let status = ActionItemStatus::Todo; // match signable {
                                                 //     true => ActionItemStatus::Todo,
                                                 //     false => ActionItemStatus::Blocked,
                                                 // };

            let expected_signer_address = signer_state.get_string(CHECKED_ADDRESS);

            let payload = signer_state
                .get_expected_scoped_value(
                    &construct_did_str,
                    WEB_WALLET_UNSIGNED_TRANSACTION_BYTES,
                )
                .map_err(|e| signer_err(&signers, &signer_state, e.message))?;

            let formatted_payload = signer_state
                .get_scoped_value(&construct_did_str, FORMATTED_TRANSACTION)
                .and_then(|v| v.as_string())
                .and_then(|v| Some(v.to_string()));

            let request = ActionItemRequest::new(
                &Some(construct_did.clone()),
                title,
                description.clone(),
                status,
                SendTransactionRequest::new(
                    &signer_state.uuid,
                    &payload,
                    NAMESPACE,
                    &chain_id.to_string(),
                )
                .expected_signer_address(expected_signer_address)
                .check_expectation_action_uuid(construct_did)
                .formatted_payload(formatted_payload)
                .to_action_type(),
                ACTION_ITEM_SEND_TRANSACTION,
            );
            Actions::append_item(
                request,
                Some("Review and send the transactions from the list below"),
                Some("Transaction Execution"),
            )
        };

        Ok((signers, signer_state, actions))
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
        if let Some(signed_transaction) = signer_state.get_scoped_value(&key, TX_HASH) {
            result.outputs.insert(TX_HASH.into(), signed_transaction.clone());
        }

        return_synchronous_result(Ok((signers, signer_state, result)))
    }
}
