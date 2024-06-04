use txtx_addon_kit::{
    types::{
        diagnostics::Diagnostic,
        frontend::{
            ActionItemRequest, ActionItemRequestType, ActionItemStatus, ProvidePublicKeyRequest,
            ReviewInputRequest,
        },
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
    uuid: &ConstructUuid,
    instance_name: &str,
    network_id: &str,
    rpc_api_url: &str,
    is_public_key_required: bool,
    is_balance_check_required: bool,
    is_address_check_required: bool,
) -> Result<Vec<ActionItemRequest>, Diagnostic> {
    let mut action_items: Vec<ActionItemRequest> = vec![];

    let stacks_rpc = StacksRpc::new(&rpc_api_url);

    if is_public_key_required {
        action_items.push(ActionItemRequest::new(
            &uuid.value(),
            &Some(uuid.value()),
            0,
            &format!("Connect wallet {instance_name}"),
            "".into(),
            ActionItemStatus::Todo,
            ActionItemRequestType::ProvidePublicKey(ProvidePublicKeyRequest {
                check_expectation_action_uuid: Some(uuid.value()),
                message: DEFAULT_MESSAGE.to_string(),
                network_id: network_id.into(),
                namespace: "stacks".to_string(),
            }),
            ACTION_ITEM_PROVIDE_PUBLIC_KEY,
        ));
    }

    if is_address_check_required {
        if let Some(ref expected_address) = expected_address {
            action_items.push(ActionItemRequest::new(
                &Uuid::new_v4(),
                &Some(uuid.value()),
                0,
                &format!("Check {} expected address", instance_name),
                &expected_address.to_string(),
                ActionItemStatus::Todo,
                ActionItemRequestType::ReviewInput(ReviewInputRequest {
                    input_name: "".into(), // todo
                }),
                ACTION_ITEM_CHECK_ADDRESS,
            ))
        }
    }

    if is_balance_check_required {
        let mut check_balance = ActionItemRequest::new(
            &Uuid::new_v4(),
            &Some(uuid.value()),
            0,
            "Check wallet balance (STX)",
            "",
            ActionItemStatus::Todo,
            ActionItemRequestType::ReviewInput(ReviewInputRequest {
                input_name: "".into(), // todo
            }),
            ACTION_ITEM_CHECK_BALANCE,
        );
        if let Some(ref expected_address) = expected_address {
            let balance = stacks_rpc
                .get_balance(&expected_address)
                .await
                .map_err(|e| {
                    diagnosed_error!(
                        "unable to retrieve balance {}: {}",
                        expected_address,
                        e.to_string()
                    )
                })?;
            check_balance.description = balance.balance.clone();
        }
        action_items.push(check_balance);
    }
    Ok(action_items)
}
