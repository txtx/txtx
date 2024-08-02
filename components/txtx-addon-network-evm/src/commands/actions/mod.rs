use std::str::FromStr;

use alloy::primitives::Address;
use check_confirmations::CHECK_CONFIRMATIONS;
use txtx_addon_kit::types::{
    commands::PreCommandSpecification,
    diagnostics::Diagnostic,
    types::{PrimitiveValue, Value},
    ConstructDid, Did, ValueStore,
};

pub mod check_confirmations;
pub mod deploy_contract;
pub mod eth_call;
pub mod get_forge_deployment_artifacts;
pub mod sign_contract_call;
pub mod sign_transfer;
pub mod verify_contract;

use deploy_contract::EVM_DEPLOY_CONTRACT;
use eth_call::ETH_CALL;
use get_forge_deployment_artifacts::GET_FORGE_DEPLOYMENT_ARTIFACTS;
use sign_contract_call::SIGN_EVM_CONTRACT_CALL;
use sign_transfer::SIGN_EVM_TRANSFER;
use verify_contract::VERIFY_CONTRACT;

use crate::constants::TRANSACTION_FROM;

lazy_static! {
    pub static ref ACTIONS: Vec<PreCommandSpecification> = vec![
        SIGN_EVM_TRANSFER.clone(),
        SIGN_EVM_CONTRACT_CALL.clone(),
        ETH_CALL.clone(),
        EVM_DEPLOY_CONTRACT.clone(),
        GET_FORGE_DEPLOYMENT_ARTIFACTS.clone(),
        VERIFY_CONTRACT.clone(),
        CHECK_CONFIRMATIONS.clone()
    ];
}

pub fn get_expected_address(value: &Value) -> Result<Address, String> {
    match value {
        Value::Primitive(PrimitiveValue::Buffer(address)) => {
            Ok(Address::from_slice(&address.bytes))
        }
        Value::Primitive(PrimitiveValue::String(address)) => {
            Ok(Address::from_str(&address).map_err(|e| format!("invalid address: {}", e))?)
        }
        value => Err(format!("unexpected address type: {:?}", value)),
    }
}

fn get_signing_construct_did(args: &ValueStore) -> Result<ConstructDid, Diagnostic> {
    let signer = args.get_expected_string(TRANSACTION_FROM)?;
    let signing_construct_did = ConstructDid(Did::from_hex_string(signer));
    Ok(signing_construct_did)
}
