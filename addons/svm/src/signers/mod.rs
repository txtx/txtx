pub mod secret_key;
pub mod squads;
pub mod web_wallet;

use secret_key::SVM_SECRET_KEY;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use squads::SVM_SQUADS;
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
use web_wallet::SVM_WEB_WALLET;

use crate::constants::{
    ACTION_ITEM_CHECK_ADDRESS, ACTION_ITEM_CHECK_BALANCE, ACTION_ITEM_PROVIDE_PUBLIC_KEY, NAMESPACE,
};

lazy_static! {
    pub static ref SIGNERS: Vec<SignerSpecification> =
        vec![SVM_SECRET_KEY.clone(), SVM_WEB_WALLET.clone(), SVM_SQUADS.clone()];
}

pub async fn get_additional_actions_for_address(
    expected_address: &Option<Pubkey>,
    signer_did: &ConstructDid,
    instance_name: &str,
    network_id: &str,
    rpc_api_url: &str,
    do_request_public_key: bool,
    do_request_balance: bool,
    do_request_address_check: bool,
) -> Result<Vec<ActionItemRequest>, Diagnostic> {
    let mut action_items: Vec<ActionItemRequest> = vec![];

    let solana_rpc = RpcClient::new(rpc_api_url.to_string());

    if do_request_public_key {
        action_items.push(ActionItemRequest::new(
            &Some(signer_did.clone()),
            &format!("Connect wallet {instance_name}"),
            None,
            ActionItemStatus::Todo,
            ActionItemRequestType::ProvidePublicKey(ProvidePublicKeyRequest {
                check_expectation_action_uuid: Some(signer_did.clone()),
                message: "".to_string(),
                network_id: network_id.into(),
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
            let (action_status, value) = match solana_rpc.get_balance(&expected_address).await {
                Ok(response) => (
                    ActionItemStatus::Todo,
                    Value::float(solana_sdk::native_token::lamports_to_sol(response)),
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
