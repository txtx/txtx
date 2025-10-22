use alloy::primitives::{utils::format_units, Address};
use txtx_addon_kit::types::{
    frontend::{
        ActionItemRequest, ActionItemRequestType, ActionItemStatus, ProvidePublicKeyRequest,
        ReviewInputRequest,
    },
    namespace::Namespace,
    stores::ValueStore,
    types::Value,
    ConstructDid,
};

use crate::{
    constants::{
        DEFAULT_MESSAGE, NAMESPACE, NONCE,
    },
    rpc::EvmRpc,
};
use txtx_addon_kit::constants::ActionItemKey;

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
                namespace: Namespace::from(NAMESPACE),
            })
            .to_request(instance_name, ActionItemKey::ProvidePublicKey)
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
                    .to_request(instance_name, ActionItemKey::CheckAddress)
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
                .to_request(instance_name, ActionItemKey::CheckBalance)
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
                .to_request(instance_name, ActionItemKey::CheckBalance)
                .with_construct_did(signer_did)
                .with_meta_description(&format!("Check '{}' signer balance", instance_name))
                .with_some_description(Some("".into()));

            action_items.push(check_balance);
        }
    }
    Ok(action_items)
}

pub fn get_signer_nonce(signer_state: &ValueStore, chain_id: u64) -> Result<Option<u64>, String> {
    signer_state
        .get_scoped_value(&format!("chain_id_{}", chain_id), NONCE)
        .map(|v| v.expect_uint())
        .transpose()
}

pub fn set_signer_nonce(signer_state: &mut ValueStore, chain_id: u64, nonce: u64) {
    signer_state.insert_scoped_value(
        &format!("chain_id_{}", chain_id),
        NONCE,
        Value::integer(nonce as i128),
    );
}
