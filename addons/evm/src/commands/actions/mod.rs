use std::str::FromStr;

use alloy::primitives::Address;
use check_confirmations::CHECK_CONFIRMATIONS;
use transfer_eth::TRANSFER_ETH;
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::{
    commands::PreCommandSpecification, diagnostics::Diagnostic, types::Value, ConstructDid, Did,
};

pub mod call_contract;
pub mod check_confirmations;
pub mod eth_call;
pub mod proxy_deploy_contract;
pub mod sign_transaction;
pub mod transfer_eth;
pub mod verify_contract;

use call_contract::SIGN_EVM_CONTRACT_CALL;
use eth_call::ETH_CALL;
use proxy_deploy_contract::PROXY_DEPLOY_CONTRACT;
use sign_transaction::SIGN_TRANSACTION;
use verify_contract::VERIFY_CONTRACT;

use crate::{
    constants::{GAS_LIMIT, NONCE, SIGNER, TRANSACTION_AMOUNT},
    typing::EVM_ADDRESS,
};

lazy_static! {
    pub static ref ACTIONS: Vec<PreCommandSpecification> = vec![
        SIGN_EVM_CONTRACT_CALL.clone(),
        ETH_CALL.clone(),
        VERIFY_CONTRACT.clone(),
        CHECK_CONFIRMATIONS.clone(),
        SIGN_TRANSACTION.clone(),
        TRANSFER_ETH.clone(),
        PROXY_DEPLOY_CONTRACT.clone(),
    ];
}

pub fn get_expected_address(value: &Value) -> Result<Address, String> {
    match value {
        Value::Buffer(bytes) => Ok(Address::from_slice(&bytes)),
        Value::String(address) => {
            Ok(Address::from_str(&address).map_err(|e| format!("invalid address: {}", e))?)
        }
        Value::Addon(addon_data) => {
            if addon_data.id != EVM_ADDRESS {
                return Err(format!("invalid data type for address: {}", addon_data.id));
            }
            Ok(Address::from_slice(&addon_data.bytes))
        }
        value => Err(format!("unexpected address type: {:?}", value)),
    }
}

pub fn get_common_tx_params_from_args(
    args: &ValueStore,
) -> Result<(u64, Option<u64>, Option<u64>), String> {
    let amount = args.get_uint(TRANSACTION_AMOUNT)?.unwrap_or(0);
    let gas_limit = args.get_uint(GAS_LIMIT)?;
    let nonce = args.get_uint(NONCE)?;
    Ok((amount, gas_limit, nonce))
}

fn get_signer_did(args: &ValueStore) -> Result<ConstructDid, Diagnostic> {
    let signer = args.get_expected_string(SIGNER)?;
    let signer_did = ConstructDid(Did::from_hex_string(signer));
    Ok(signer_did)
}
