use txtx_addon_kit::{
    types::{
        diagnostics::Diagnostic,
        frontend::{
            ActionItemRequest, ActionItemRequestType, ActionItemStatus, ProvidePublicKeyRequest,
            ReviewInputRequest,
        },
        types::Value,
        wallets::WalletSpecification,
        ConstructUuid,
    },
    uuid::Uuid,
};

mod connect;
mod multisig;

use connect::STACKS_CONNECT;
use multisig::STACKS_MULTISIG;

use crate::{
    constants::{
        ACTION_ITEM_CHECK_ADDRESS, ACTION_ITEM_CHECK_BALANCE, ACTION_ITEM_PROVIDE_PUBLIC_KEY,
        DEFAULT_MESSAGE,
    },
    rpc::StacksRpc,
};

lazy_static! {
    pub static ref WALLETS: Vec<WalletSpecification> =
        vec![STACKS_CONNECT.clone(), STACKS_MULTISIG.clone()];
}

pub async fn get_addition_actions_for_address(
    expected_address: &Option<String>,
    wallet_uuid: &ConstructUuid,
    instance_name: &str,
    network_id: &str,
    rpc_api_url: &str,
    do_request_public_key: bool,
    do_request_balance: bool,
    do_request_address_check: bool,
) -> Result<Vec<ActionItemRequest>, Diagnostic> {
    let mut action_items: Vec<ActionItemRequest> = vec![];

    let stacks_rpc = StacksRpc::new(&rpc_api_url);

    if do_request_public_key {
        action_items.push(ActionItemRequest::new(
            &Uuid::new_v4(),
            &Some(wallet_uuid.value()),
            &format!("Connect wallet {instance_name}"),
            None,
            ActionItemStatus::Todo,
            ActionItemRequestType::ProvidePublicKey(ProvidePublicKeyRequest {
                check_expectation_action_uuid: Some(wallet_uuid.value()),
                message: DEFAULT_MESSAGE.to_string(),
                network_id: network_id.into(),
                namespace: "stacks".to_string(),
            }),
            ACTION_ITEM_PROVIDE_PUBLIC_KEY,
        ));
    }

    if let Some(ref expected_address) = expected_address {
        if do_request_address_check {
            action_items.push(ActionItemRequest::new(
                &Uuid::new_v4(),
                &Some(wallet_uuid.value()),
                &format!("Check {} expected address", instance_name),
                None,
                ActionItemStatus::Todo,
                ActionItemRequestType::ReviewInput(ReviewInputRequest {
                    input_name: "".into(), // todo
                    value: Value::string(expected_address.to_owned()),
                }),
                ACTION_ITEM_CHECK_ADDRESS,
            ))
        }
        if do_request_balance {
            let (action_status, value) = match stacks_rpc.get_balance(&expected_address).await {
                Ok(response) => (
                    ActionItemStatus::Todo,
                    Value::string(response.get_formatted_balance()),
                ),
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
                &Uuid::new_v4(),
                &Some(wallet_uuid.value()),
                "Check wallet balance",
                None,
                action_status,
                ActionItemRequestType::ReviewInput(ReviewInputRequest {
                    input_name: "".into(), // todo
                    value: value,
                }),
                ACTION_ITEM_CHECK_BALANCE,
            );
            action_items.push(check_balance);
        }
    }
    Ok(action_items)
}
