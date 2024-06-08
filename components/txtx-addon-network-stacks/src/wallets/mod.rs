use txtx_addon_kit::{
    types::{
        diagnostics::Diagnostic,
        frontend::{
            ActionItemRequest, ActionItemRequestType, ActionItemRequestUpdate, ActionItemStatus,
            Actions, ProvidePublicKeyRequest, ReviewInputRequest,
        },
        types::Value,
        wallets::WalletSpecification,
        ConstructUuid, ValueStore,
    },
    uuid::Uuid,
};

mod connect;
mod multisig;

use connect::STACKS_CONNECT;
use multisig::STACKS_MULTISIG;

use crate::{
    constants::{
        ACTION_ITEM_CHECK_ADDRESS, ACTION_ITEM_CHECK_BALANCE, ACTION_ITEM_CHECK_NONCE,
        ACTION_ITEM_PROVIDE_PUBLIC_KEY, DEFAULT_MESSAGE, FETCHED_BALANCE, FETCHED_NONCE,
        REQUESTED_STARTUP_DATA,
    },
    rpc::StacksRpc,
};

lazy_static! {
    pub static ref WALLETS: Vec<WalletSpecification> =
        vec![STACKS_CONNECT.clone(), STACKS_MULTISIG.clone()];
}

pub async fn get_additional_actions_for_address(
    connected_address: &Option<String>,
    expected_address: &Option<String>,
    wallet_uuid: &ConstructUuid,
    instance_name: &str,
    network_id: &str,
    rpc_api_url: &str,
    is_public_key_required: bool,
    is_balance_check_required: bool,
    is_address_check_required: bool,
    is_nonce_check_required: bool,
    wallet_state: &mut ValueStore,
) -> Result<Actions, Diagnostic> {
    let mut action_items: Vec<ActionItemRequest> = vec![];
    let mut actions = Actions::none();

    let nonce_is_cached = wallet_state.get_expected_uint(FETCHED_NONCE).is_ok();
    let balance_is_cached = wallet_state.get_expected_string(FETCHED_BALANCE).is_ok();
    let requested_startup_data = wallet_state
        .get_expected_bool(REQUESTED_STARTUP_DATA)
        .ok()
        .unwrap_or(false);

    let stacks_rpc = StacksRpc::new(&rpc_api_url);

    // only request public key if we haven't already created that action
    if is_public_key_required && !requested_startup_data {
        action_items.push(ActionItemRequest::new(
            &Uuid::new_v4(),
            &Some(wallet_uuid.value()),
            0,
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
        if is_address_check_required && !requested_startup_data {
            action_items.push(ActionItemRequest::new(
                &Uuid::new_v4(),
                &Some(wallet_uuid.value()),
                0,
                &format!("Check {} expected address", instance_name),
                None,
                ActionItemStatus::Todo,
                ActionItemRequestType::ReviewInput(ReviewInputRequest {
                    input_name: "".into(), // todo
                    value: Value::string(expected_address.to_owned()),
                }),
                ACTION_ITEM_CHECK_ADDRESS,
            ));
        }
    }
    let v2_fetch_required = (is_balance_check_required || is_nonce_check_required)
        && (!nonce_is_cached || !balance_is_cached);
    if v2_fetch_required {
        if let Some(ref connected_address) = connected_address {
            let (action_status, balance, nonce) =
                match stacks_rpc.get_v2_accounts(&connected_address).await {
                    Ok(response) => {
                        let balance = Value::string(response.balance);
                        wallet_state.insert(FETCHED_BALANCE, balance.clone());
                        let nonce = Value::uint(response.nonce);
                        wallet_state.insert(FETCHED_NONCE, nonce.clone());
                        (ActionItemStatus::Success(None), balance, nonce)
                    }
                    Err(err) => (
                        ActionItemStatus::Error(diagnosed_error!(
                            "unable to retrieve balance {}: {}",
                            connected_address,
                            err.to_string()
                        )),
                        Value::string("N/A".to_string()),
                        Value::string("N/A".to_string()),
                    ),
                };
            if is_balance_check_required && !balance_is_cached {
                let check_balance_update =
                    ActionItemRequestUpdate::from_context(wallet_uuid, ACTION_ITEM_CHECK_BALANCE)
                        .set_type(ActionItemRequestType::ReviewInput(ReviewInputRequest {
                            input_name: "".into(), // todo
                            value: balance,
                        }))
                        .set_status(action_status.clone());
                actions.push_action_item_update(check_balance_update);
            }
            if is_nonce_check_required && !nonce_is_cached {
                let check_nonce_update =
                    ActionItemRequestUpdate::from_context(wallet_uuid, ACTION_ITEM_CHECK_NONCE)
                        .set_type(ActionItemRequestType::ReviewInput(ReviewInputRequest {
                            input_name: "".into(), // todo
                            value: nonce,
                        }))
                        .set_status(action_status.clone());
                actions.push_action_item_update(check_nonce_update);
            }
        } else {
            println!("nonce check? {}", is_nonce_check_required);
            if is_balance_check_required && !requested_startup_data {
                println!("adding check balance action");
                let check_balance = ActionItemRequest::new(
                    &Uuid::new_v4(),
                    &Some(wallet_uuid.value()),
                    0,
                    "Check wallet balance (STX)",
                    None,
                    ActionItemStatus::Todo,
                    ActionItemRequestType::ReviewInput(ReviewInputRequest {
                        input_name: "".into(), // todo
                        value: Value::string("N/A".to_string()),
                    }),
                    ACTION_ITEM_CHECK_BALANCE,
                );
                action_items.push(check_balance);
            }
            if is_nonce_check_required && !requested_startup_data {
                println!("adding check nonce action");
                let check_nonce = ActionItemRequest::new(
                    &Uuid::new_v4(),
                    &Some(wallet_uuid.value()),
                    0,
                    "Check sender nonce",
                    None,
                    ActionItemStatus::Todo,
                    ActionItemRequestType::ReviewInput(ReviewInputRequest {
                        input_name: "".into(), // todo
                        value: Value::string("N/A".to_string()),
                    }),
                    ACTION_ITEM_CHECK_NONCE,
                );
                action_items.push(check_nonce);
            }
        }
    }
    actions.push_sub_group(action_items);
    Ok(actions)
}
