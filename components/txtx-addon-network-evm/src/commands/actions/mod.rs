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
mod deploy_contract_create2;
pub mod eth_call;
pub mod sign_contract_call;
pub mod sign_transaction;
pub mod sign_transfer;
pub mod verify_contract;

use deploy_contract::EVM_DEPLOY_CONTRACT;
use deploy_contract_create2::EVM_DEPLOY_CONTRACT_CREATE2;
use eth_call::ETH_CALL;
use sign_contract_call::SIGN_EVM_CONTRACT_CALL;
use sign_transaction::SIGN_TRANSACTION;
use sign_transfer::SIGN_EVM_TRANSFER;
use verify_contract::VERIFY_CONTRACT;

use crate::{constants::SIGNER, typing::ETH_ADDRESS};

lazy_static! {
    pub static ref ACTIONS: Vec<PreCommandSpecification> = vec![
        SIGN_EVM_TRANSFER.clone(),
        SIGN_EVM_CONTRACT_CALL.clone(),
        ETH_CALL.clone(),
        EVM_DEPLOY_CONTRACT.clone(),
        VERIFY_CONTRACT.clone(),
        CHECK_CONFIRMATIONS.clone(),
        SIGN_TRANSACTION.clone(),
        EVM_DEPLOY_CONTRACT_CREATE2.clone()
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
        Value::Addon(addon_data) => {
            if addon_data.typing.id != ETH_ADDRESS.id {
                return Err(format!(
                    "invalid data type for address: {}",
                    addon_data.typing.id
                ));
            }
            get_expected_address(&addon_data.value)
        }
        value => Err(format!("unexpected address type: {:?}", value)),
    }
}

fn get_signing_construct_did(args: &ValueStore) -> Result<ConstructDid, Diagnostic> {
    let signer = args.get_expected_string(SIGNER)?;
    let signing_construct_did = ConstructDid(Did::from_hex_string(signer));
    Ok(signing_construct_did)
}
