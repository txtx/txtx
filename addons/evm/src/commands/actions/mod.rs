use std::str::FromStr;

use alloy::primitives::Address;
use check_confirmations::CHECK_CONFIRMATIONS;
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::{
    commands::PreCommandSpecification, diagnostics::Diagnostic, types::Value, ConstructDid, Did,
};

pub mod call_contract;
pub mod check_confirmations;
pub mod deploy_contract;
pub mod eth_call;
pub mod send_eth;
pub mod sign_transaction;
pub mod verify_contract;

use call_contract::SIGN_EVM_CONTRACT_CALL;
use deploy_contract::DEPLOY_CONTRACT;
use eth_call::ETH_CALL;
use send_eth::SEND_ETH;
use sign_transaction::SIGN_TRANSACTION;
use verify_contract::VERIFY_CONTRACT;

use crate::constants::{GAS_LIMIT, NONCE, SIGNER, TRANSACTION_AMOUNT};
use crate::typing::EvmValue;

lazy_static! {
    pub static ref ACTIONS: Vec<PreCommandSpecification> = vec![
        SIGN_EVM_CONTRACT_CALL.clone(),
        ETH_CALL.clone(),
        VERIFY_CONTRACT.clone(),
        CHECK_CONFIRMATIONS.clone(),
        SIGN_TRANSACTION.clone(),
        SEND_ETH.clone(),
        DEPLOY_CONTRACT.clone(),
    ];
}

pub fn get_expected_address(value: &Value) -> Result<Address, String> {
    EvmValue::to_address(value).map_err(|e| format!("failed to parse address: {}", e.message))
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
