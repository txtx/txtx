use std::str::FromStr;

use alloy::primitives::Address;
use txtx_addon_kit::types::{
    commands::PreCommandSpecification,
    diagnostics::Diagnostic,
    types::{PrimitiveValue, Value},
    ConstructDid, Did, ValueStore,
};

pub mod call_view_function;
pub mod get_forge_deployment_artifacts;
pub mod set_default_network;
pub mod sign_contract_call;
pub mod sign_contract_deploy;
pub mod sign_transfer;
pub mod verify_deployment;

use call_view_function::CALL_VIEW_FUNCTION;
use get_forge_deployment_artifacts::GET_FORGE_DEPLOYMENT_ARTIFACTS;
use set_default_network::SET_DEFAULT_NETWORK;
use sign_contract_call::SIGN_EVM_CONTRACT_CALL;
use sign_contract_deploy::SIGN_EVM_CONTRACT_DEPLOY;
use sign_transfer::SIGN_EVM_TRANSFER;
use verify_deployment::VERIFY_CONTRACT_DEPLOYMENT;

use crate::constants::TRANSACTION_FROM;

lazy_static! {
    pub static ref ACTIONS: Vec<PreCommandSpecification> = vec![
        SET_DEFAULT_NETWORK.clone(),
        SIGN_EVM_TRANSFER.clone(),
        SIGN_EVM_CONTRACT_CALL.clone(),
        CALL_VIEW_FUNCTION.clone(),
        SIGN_EVM_CONTRACT_DEPLOY.clone(),
        GET_FORGE_DEPLOYMENT_ARTIFACTS.clone(),
        VERIFY_CONTRACT_DEPLOYMENT.clone()
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
