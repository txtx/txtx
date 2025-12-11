use alloy::network::{EthereumWallet, TransactionBuilder};
use alloy::primitives::{utils::format_units, Address};
use alloy_rpc_types::TransactionRequest;
use txtx_addon_kit::{
    constants::SIGNATURE_APPROVED,
    indexmap::IndexMap,
    types::{
        commands::CommandExecutionResult,
        frontend::{
            ActionItemRequest, ActionItemRequestType, ActionItemStatus, Actions,
            ProvidePublicKeyRequest, ProvideSignedTransactionRequest, ReviewInputRequest,
        },
        signers::{SignerActionErr, SignersState},
        stores::ValueStore,
        types::{RunbookSupervisionContext, Value},
        ConstructDid,
    },
};

use crate::{
    codec::crypto::field_bytes_to_secret_key_signer,
    constants::{
        ACTION_ITEM_CHECK_ADDRESS, ACTION_ITEM_CHECK_BALANCE, ACTION_ITEM_PROVIDE_PUBLIC_KEY,
        ACTION_ITEM_PROVIDE_SIGNED_TRANSACTION, CHAIN_ID, DEFAULT_MESSAGE, FORMATTED_TRANSACTION,
        NAMESPACE, RPC_API_URL, SECRET_KEY_WALLET_UNSIGNED_TRANSACTION_BYTES, TX_HASH,
    },
    rpc::{EvmRpc, EvmWalletRpc},
    typing::EvmValue,
};

pub async fn get_additional_actions_for_address(
    expected_address: &Option<Address>,
    signer_did: &ConstructDid,
    instance_name: &str,
    description: Option<String>,
    markdown: Option<String>,
    rpc_api_url: &str,
    chain_id: u64,
    do_request_public_key: bool,
    do_request_balance: bool,
    do_request_address_check: bool,
) -> Result<Vec<ActionItemRequest>, String> {
    let mut action_items: Vec<ActionItemRequest> = vec![];

    let rpc = EvmRpc::new(&rpc_api_url)?;

    let actual_chain_id = rpc.get_chain_id().await.map_err(|e| {
        format!("unable to retrieve chain id from RPC {}: {}", rpc_api_url, e.to_string())
    })?;
    if actual_chain_id != chain_id {
        return Err(format!(
            "chain id mismatch: expected {}, got {} from the provided rpc",
            chain_id, actual_chain_id
        ));
    }

    if do_request_public_key {
        action_items.push(
            ActionItemRequestType::ProvidePublicKey(ProvidePublicKeyRequest {
                check_expectation_action_uuid: Some(signer_did.clone()),
                message: DEFAULT_MESSAGE.to_string(),
                network_id: chain_id.to_string(),
                namespace: NAMESPACE.to_string(),
            })
            .to_request(instance_name, ACTION_ITEM_PROVIDE_PUBLIC_KEY)
            .with_construct_did(signer_did)
            .with_some_description(description)
            .with_meta_description(&format!("Connect wallet '{instance_name}'"))
            .with_some_markdown(markdown),
        );
    }

    if let Some(ref expected_address) = expected_address {
        if do_request_address_check {
            action_items.push(
                ReviewInputRequest::new("", &Value::string(expected_address.to_string()))
                    .to_action_type()
                    .to_request(instance_name, ACTION_ITEM_CHECK_ADDRESS)
                    .with_construct_did(signer_did)
                    .with_meta_description(&format!("Check '{}' expected address", instance_name))
                    .with_some_description(Some("".into())),
            );
        }
        if do_request_balance {
            let (action_status, value) = match rpc.get_balance(&expected_address).await {
                Ok(response) => {
                    let balance = format_units(response, "ether")
                        .map_err(|e| format!("received invalid ethereum balance from RCP: {e}"))?;
                    (ActionItemStatus::Todo, Value::string(balance))
                }
                Err(err) => (
                    ActionItemStatus::Error(diagnosed_error!(
                        "unable to retrieve balance {}: {}",
                        expected_address,
                        err.to_string()
                    )),
                    Value::string("N/A".to_string()),
                ),
            };
            let check_balance = ReviewInputRequest::new("", &value)
                .to_action_type()
                .to_request(instance_name, ACTION_ITEM_CHECK_BALANCE)
                .with_construct_did(signer_did)
                .with_meta_description(&format!("Check '{}' signer balance", instance_name))
                .with_some_description(Some("".into()))
                .with_status(action_status);
            action_items.push(check_balance);
        }
    } else {
        if do_request_balance {
            let check_balance = ReviewInputRequest::new("", &Value::string("N/A".to_string()))
                .to_action_type()
                .to_request(instance_name, ACTION_ITEM_CHECK_BALANCE)
                .with_construct_did(signer_did)
                .with_meta_description(&format!("Check '{}' signer balance", instance_name))
                .with_some_description(Some("".into()));

            action_items.push(check_balance);
        }
    }
    Ok(action_items)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NonceManager(IndexMap<u64, IndexMap<ConstructDid, u64>>);
impl NonceManager {
    const KEY: &str = "nonce_manager";
    pub fn get_nonce_for_construct(
        signer_state: &ValueStore,
        chain_id: u64,
        construct_did: &ConstructDid,
    ) -> Option<u64> {
        let manager = Self::from_signer_state(signer_state).ok()?;
        manager.0.get(&chain_id).and_then(|map_for_chain| map_for_chain.get(construct_did).cloned())
    }
    pub async fn claim_next_nonce(
        signer_state: &mut ValueStore,
        construct_did: &ConstructDid,
        chain_id: u64,
        rpc_api_url: &str,
        address: &Address,
    ) -> Result<(), String> {
        let mut manager = Self::from_signer_state(signer_state)?;
        let map_for_chain = manager.0.entry(chain_id).or_insert(IndexMap::new());
        if map_for_chain.is_empty() {
            let rpc = EvmRpc::new(&rpc_api_url)?;
            let nonce =
                rpc.get_nonce(address).await.map_err(|e| format!("failed to get nonce: {e}"))?;
            map_for_chain.insert(construct_did.clone(), nonce);
        } else if map_for_chain.get(construct_did).is_none() {
            let last_nonce = map_for_chain.values().max().cloned().unwrap();
            map_for_chain.insert(construct_did.clone(), last_nonce + 1);
        }
        let serialized = serde_json::to_string(&manager)
            .map_err(|e| format!("failed to serialize nonce manager: {e}"))?;
        signer_state.insert(Self::KEY, Value::string(serialized));
        Ok(())
    }

    pub fn from_signer_state(signer_state: &ValueStore) -> Result<Self, String> {
        if let Some(value) = signer_state.get_value(Self::KEY) {
            let nonce_manager =
                serde_json::from_str::<NonceManager>(&value.as_string().unwrap())
                    .map_err(|e| format!("failed to parse nonce manager from signer state: {e}"))?;

            return Ok(nonce_manager);
        }
        Ok(NonceManager(IndexMap::new()))
    }
}

/// Shared activate implementation for secret_key and keystore signers
pub fn activate_signer(
    signer_state: ValueStore,
    signers: SignersState,
) -> Result<(SignersState, ValueStore, CommandExecutionResult), (SignersState, ValueStore, txtx_addon_kit::types::diagnostics::Diagnostic)> {
    let mut result = CommandExecutionResult::new();
    let address = signer_state
        .get_expected_value("signer_address")
        .map_err(|e| (signers.clone(), signer_state.clone(), e))?;
    result.outputs.insert("address".into(), address.clone());
    Ok((signers, signer_state, result))
}

/// Shared check_signability implementation for secret_key and keystore signers
pub fn check_signability(
    construct_did: &ConstructDid,
    title: &str,
    description: &Option<String>,
    meta_description: &Option<String>,
    markdown: &Option<String>,
    payload: &Value,
    values: &ValueStore,
    signer_state: ValueStore,
    signers: SignersState,
    supervision_context: &RunbookSupervisionContext,
) -> Result<(SignersState, ValueStore, Actions), SignerActionErr> {
    let actions = if supervision_context.review_input_values {
        let construct_did_str = construct_did.to_string();
        if signer_state.get_scoped_value(&construct_did_str, SIGNATURE_APPROVED).is_some() {
            return Ok((signers, signer_state, Actions::none()));
        }

        let chain_id = values
            .get_expected_uint(CHAIN_ID)
            .map_err(|e| (signers.clone(), signer_state.clone(), e))?;

        let formatted_payload =
            signer_state.get_scoped_value(&construct_did_str, FORMATTED_TRANSACTION);

        let request = ProvideSignedTransactionRequest::new(
            &signer_state.uuid,
            payload,
            NAMESPACE,
            &chain_id.to_string(),
        )
        .check_expectation_action_uuid(construct_did)
        .only_approval_needed()
        .formatted_payload(formatted_payload)
        .to_action_type()
        .to_request(title, ACTION_ITEM_PROVIDE_SIGNED_TRANSACTION)
        .with_construct_did(construct_did)
        .with_some_description(description.clone())
        .with_some_meta_description(meta_description.clone())
        .with_some_markdown(markdown.clone())
        .with_status(ActionItemStatus::Todo);

        Actions::append_item(
            request,
            Some("Review and sign the transactions from the list below"),
            Some("Transaction Signing"),
        )
    } else {
        Actions::none()
    };
    Ok((signers, signer_state, actions))
}

/// Shared sign implementation for secret_key and keystore signers
pub async fn sign_transaction(
    caller_uuid: &ConstructDid,
    values: &ValueStore,
    signer_state: ValueStore,
    signers: SignersState,
) -> Result<(SignersState, ValueStore, CommandExecutionResult), (SignersState, ValueStore, txtx_addon_kit::types::diagnostics::Diagnostic)> {
    let mut result = CommandExecutionResult::new();

    let rpc_api_url = values
        .get_expected_string(RPC_API_URL)
        .map_err(|e| (signers.clone(), signer_state.clone(), e))?;

    let signer_field_bytes = signer_state
        .get_expected_buffer_bytes("signer_field_bytes")
        .map_err(|e| (signers.clone(), signer_state.clone(), e))?;

    let payload_bytes = signer_state
        .get_expected_scoped_buffer_bytes(
            &caller_uuid.to_string(),
            SECRET_KEY_WALLET_UNSIGNED_TRANSACTION_BYTES,
        )
        .map_err(|e| (signers.clone(), signer_state.clone(), e))?;

    let secret_key_signer = field_bytes_to_secret_key_signer(&signer_field_bytes)
        .map_err(|e| (signers.clone(), signer_state.clone(), diagnosed_error!("{e}")))?;

    let eth_signer = EthereumWallet::from(secret_key_signer);

    let mut tx: TransactionRequest = serde_json::from_slice(&payload_bytes).map_err(|e| {
        (
            signers.clone(),
            signer_state.clone(),
            diagnosed_error!("failed to deserialize transaction: {e}"),
        )
    })?;

    if tx.to.is_none() {
        tx.set_create();
    }

    let tx_envelope = tx.build(&eth_signer).await.map_err(|e| {
        (
            signers.clone(),
            signer_state.clone(),
            diagnosed_error!("failed to build transaction envelope: {e}"),
        )
    })?;

    let rpc = EvmWalletRpc::new(rpc_api_url, eth_signer)
        .map_err(|e| (signers.clone(), signer_state.clone(), diagnosed_error!("{e}")))?;

    let tx_hash = rpc.sign_and_send_tx(tx_envelope).await.map_err(|e| {
        (signers.clone(), signer_state.clone(), diagnosed_error!("{}", e.to_string()))
    })?;

    result.outputs.insert(TX_HASH.to_string(), EvmValue::tx_hash(tx_hash.to_vec()));

    Ok((signers, signer_state, result))
}
