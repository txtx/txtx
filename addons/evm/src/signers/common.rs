use alloy_primitives::{utils::format_units, Address};
use txtx_addon_kit::{
    indexmap::IndexMap,
    types::{
        frontend::{
            ActionItemRequest, ActionItemRequestType, ActionItemStatus, ProvidePublicKeyRequest,
            ReviewInputRequest,
        },
        stores::ValueStore,
        types::Value,
        ConstructDid,
    },
};

use crate::{
    constants::{
        ACTION_ITEM_CHECK_ADDRESS, ACTION_ITEM_CHECK_BALANCE, ACTION_ITEM_PROVIDE_PUBLIC_KEY,
        DEFAULT_MESSAGE, NAMESPACE,
    },
    rpc::EvmRpc,
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
