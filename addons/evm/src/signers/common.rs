use alloy::primitives::{utils::format_units, Address};
use txtx_addon_kit::types::{
    frontend::{
        ActionItemRequest, ActionItemRequestType, ActionItemStatus, ProvidePublicKeyRequest,
        ReviewInputRequest,
    },
    types::Value,
    ConstructDid,
};

use crate::{
    constants::{
        ACTION_ITEM_CHECK_ADDRESS, ACTION_ITEM_CHECK_BALANCE, ACTION_ITEM_PROVIDE_PUBLIC_KEY,
        DEFAULT_MESSAGE, NAMESPACE,
    },
    rpc::EVMRpc,
};

pub async fn get_additional_actions_for_address(
    expected_address: &Option<Address>,
    signer_did: &ConstructDid,
    instance_name: &str,
    rpc_api_url: &str,
    chain_id: u64,
    do_request_public_key: bool,
    do_request_balance: bool,
    do_request_address_check: bool,
) -> Result<Vec<ActionItemRequest>, String> {
    let mut action_items: Vec<ActionItemRequest> = vec![];

    let rpc = EVMRpc::new(&rpc_api_url)?;

    if do_request_public_key {
        action_items.push(ActionItemRequest::new(
            &Some(signer_did.clone()),
            &format!("Connect wallet {instance_name}"),
            None,
            ActionItemStatus::Todo,
            ActionItemRequestType::ProvidePublicKey(ProvidePublicKeyRequest {
                check_expectation_action_uuid: Some(signer_did.clone()),
                message: DEFAULT_MESSAGE.to_string(),
                network_id: chain_id.to_string(),
                namespace: NAMESPACE.to_string(),
            }),
            ACTION_ITEM_PROVIDE_PUBLIC_KEY,
        ));
    }

    if let Some(ref expected_address) = expected_address {
        if do_request_address_check {
            action_items.push(ActionItemRequest::new(
                &Some(signer_did.clone()),
                &format!("Check {} expected address", instance_name),
                None,
                ActionItemStatus::Todo,
                ReviewInputRequest::new("", &Value::string(expected_address.to_string()))
                    .to_action_type(),
                ACTION_ITEM_CHECK_ADDRESS,
            ))
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
            let check_balance = ActionItemRequest::new(
                &Some(signer_did.clone()),
                "Check signer balance",
                None,
                action_status,
                ReviewInputRequest::new("", &value).to_action_type(),
                ACTION_ITEM_CHECK_BALANCE,
            );
            action_items.push(check_balance);
        }
    } else {
        if do_request_balance {
            let check_balance = ActionItemRequest::new(
                &Some(signer_did.clone()),
                "Check signer balance",
                None,
                ActionItemStatus::Todo,
                ReviewInputRequest::new("", &Value::string("N/A".to_string())).to_action_type(),
                ACTION_ITEM_CHECK_BALANCE,
            );
            action_items.push(check_balance);
        }
    }
    Ok(action_items)
}
