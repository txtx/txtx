pub mod secret_key;
pub mod squads;
pub mod web_wallet;

use crate::functions::lamports_to_sol;
use secret_key::SVM_SECRET_KEY;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_pubkey::Pubkey;
use squads::SVM_SQUADS;
use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    frontend::{
        ActionItemRequest, ActionItemRequestType, ActionItemStatus, ProvidePublicKeyRequest,
        ReviewInputRequest,
    },
    namespace::Namespace,
    signers::SignerSpecification,
    types::Value,
    ConstructDid,
};
use web_wallet::SVM_WEB_WALLET;

use txtx_addon_kit::constants::ActionItemKey;
use crate::constants::NAMESPACE;

lazy_static! {
    pub static ref SIGNERS: Vec<SignerSpecification> =
        vec![SVM_SECRET_KEY.clone(), SVM_WEB_WALLET.clone(), SVM_SQUADS.clone()];
}

pub async fn get_additional_actions_for_address(
    expected_address: &Option<Pubkey>,
    connected_address: &Option<Pubkey>,
    signer_did: &ConstructDid,
    instance_name: &str,
    description: Option<String>,
    markdown: Option<String>,
    network_id: &str,
    rpc_api_url: &str,
    do_request_public_key: bool,
    do_request_balance: bool,
    do_request_address_check: bool,
    is_balance_checked: Option<bool>,
) -> Result<Vec<ActionItemRequest>, Diagnostic> {
    let mut action_items: Vec<ActionItemRequest> = vec![];

    let solana_rpc = RpcClient::new(rpc_api_url.to_string());

    if do_request_public_key {
        action_items.push(
            ActionItemRequestType::ProvidePublicKey(ProvidePublicKeyRequest {
                check_expectation_action_uuid: Some(signer_did.clone()),
                message: "".to_string(),
                network_id: network_id.into(),
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
            if connected_address.is_none() {
                action_items.push(
                    ReviewInputRequest::new("", &Value::string(expected_address.to_string()))
                        .to_action_type()
                        .to_request(instance_name, ActionItemKey::CheckAddress)
                        .with_construct_did(signer_did)
                        .with_some_description(Some("".into()))
                        .with_meta_description(&format!(
                            "Check '{}' expected address",
                            instance_name
                        )),
                )
            }
        }
        if do_request_balance {
            if let Some(check_balance) = get_check_balance_action(
                &solana_rpc,
                &Some(expected_address.clone()),
                signer_did,
                is_balance_checked,
                instance_name,
            )
            .await
            {
                action_items.push(check_balance);
            }
        }
    } else {
        if do_request_balance {
            if let Some(check_balance) = get_check_balance_action(
                &solana_rpc,
                connected_address,
                signer_did,
                is_balance_checked,
                instance_name,
            )
            .await
            {
                action_items.push(check_balance);
            }
        }
    }
    Ok(action_items)
}

async fn get_check_balance_action(
    solana_rpc: &RpcClient,
    address: &Option<Pubkey>,
    signer_did: &ConstructDid,
    is_balance_checked: Option<bool>,
    instance_name: &str,
) -> Option<ActionItemRequest> {
    if is_balance_checked.is_some() {
        return None;
    }

    let (action_status, value) = match address {
        Some(address) => match solana_rpc.get_balance(&address).await {
            Ok(response) => (ActionItemStatus::Todo, Value::float(lamports_to_sol(response))),
            Err(err) => (
                ActionItemStatus::Error(diagnosed_error!(
                    "unable to retrieve balance {}: {}",
                    address,
                    err.to_string()
                )),
                Value::string("N/A".to_string()),
            ),
        },
        None => (ActionItemStatus::Todo, Value::string("N/A".to_string())),
    };

    Some(
        ReviewInputRequest::new("", &value)
            .to_action_type()
            .to_request(instance_name, ActionItemKey::CheckBalance)
            .with_construct_did(signer_did)
            .with_meta_description(&format!("Check '{}' signer balance", instance_name))
            .with_some_description(Some("".into()))
            .with_status(action_status),
    )
}
