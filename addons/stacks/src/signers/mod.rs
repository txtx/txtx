use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    frontend::{
        ActionItemRequest, ActionItemRequestType, ActionItemStatus, ProvidePublicKeyRequest,
        ReviewInputRequest,
    },
    signers::SignerSpecification,
    types::Value,
    ConstructDid,
};

mod multisig;
mod secret_key;
mod web_wallet;

use multisig::STACKS_MULTISIG;
use secret_key::STACKS_SECRET_KEY;
use web_wallet::STACKS_WEB_WALLET;

use crate::{
    constants::{
        ActionItemKey::CheckAddress, ActionItemKey::CheckBalance, ActionItemKey::ProvidePublicKey,
        DEFAULT_MESSAGE,
    },
    rpc::StacksRpc,
};

pub const DEFAULT_DERIVATION_PATH: &str = "m/44'/5757'/0'/0/0";

lazy_static! {
    pub static ref WALLETS: Vec<SignerSpecification> =
        vec![STACKS_SECRET_KEY.clone(), STACKS_WEB_WALLET.clone(), STACKS_MULTISIG.clone()];
}

pub async fn get_addition_actions_for_address(
    expected_address: &Option<String>,
    signer_did: &ConstructDid,
    instance_name: &str,
    network_id: &str,
    rpc_api_url: &str,
    rpc_api_auth_token: &Option<String>,
    do_request_public_key: bool,
    do_request_balance: bool,
    do_request_address_check: bool,
) -> Result<Vec<ActionItemRequest>, Diagnostic> {
    let mut action_items: Vec<ActionItemRequest> = vec![];

    let stacks_rpc = StacksRpc::new(&rpc_api_url, rpc_api_auth_token);

    if do_request_public_key {
        action_items.push(ActionItemRequest::new(
            &Some(signer_did.clone()),
            &format!("Connect wallet '{instance_name}'"),
            None,
            ActionItemStatus::Todo,
            ActionItemRequestType::ProvidePublicKey(ProvidePublicKeyRequest {
                check_expectation_action_uuid: Some(signer_did.clone()),
                message: DEFAULT_MESSAGE.to_string(),
                network_id: network_id.into(),
                namespace: "stacks".to_string(),
            }),
            ActionItemKey::ProvidePublicKey,
        ));
    }

    if let Some(ref expected_address) = expected_address {
        if do_request_address_check {
            action_items.push(ActionItemRequest::new(
                &Some(signer_did.clone()),
                &format!("Check '{}' expected address", instance_name),
                None,
                ActionItemStatus::Todo,
                ReviewInputRequest::new("", &Value::string(expected_address.to_owned()))
                    .to_action_type(),
                ActionItemKey::CheckAddress,
            ))
        }
        if do_request_balance {
            let (action_status, value) = match stacks_rpc.get_balance(&expected_address).await {
                Ok(response) => {
                    (ActionItemStatus::Todo, Value::string(response.get_formatted_balance()))
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
                ActionItemKey::CheckBalance,
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
                ActionItemKey::CheckBalance,
            );
            action_items.push(check_balance);
        }
    }
    Ok(action_items)
}
