pub mod broadcast_transaction;
pub mod call_readonly_fn;
mod decode_contract_call;
mod deploy_contract;
pub mod encode_contract_call;
mod send_contract_call;
pub mod set_default_network;
pub mod sign_transaction;

use std::str::FromStr;

use crate::{stacks_helpers::parse_clarity_value, typing::STACKS_CONTRACT_CALL};
use broadcast_transaction::BROADCAST_STACKS_TRANSACTION;
use call_readonly_fn::CALL_READONLY_FN;
use clarity::codec::StacksMessageCodec;
use clarity::{
    types::chainstate::StacksAddress,
    vm::{types::PrincipalData, ClarityName},
};
use clarity_repl::codec::{TransactionContractCall, TransactionPayload};
use decode_contract_call::DECODE_STACKS_CONTRACT_CALL;
use deploy_contract::DEPLOY_STACKS_CONTRACT;
use encode_contract_call::ENCODE_STACKS_CONTRACT_CALL;
use send_contract_call::SEND_CONTRACT_CALL;
use set_default_network::SET_DEFAULT_NETWORK;
use sign_transaction::SIGN_STACKS_TRANSACTION;
use txtx_addon_kit::types::{
    commands::{CommandSpecification, PreCommandSpecification},
    diagnostics::Diagnostic,
    types::{PrimitiveValue, Value},
};
use txtx_addon_kit::types::{ConstructDid, Did, ValueStore};
use txtx_addon_kit::uuid::Uuid;

lazy_static! {
    pub static ref ACTIONS: Vec<PreCommandSpecification> = vec![
        SIGN_STACKS_TRANSACTION.clone(),
        DECODE_STACKS_CONTRACT_CALL.clone(),
        ENCODE_STACKS_CONTRACT_CALL.clone(),
        DEPLOY_STACKS_CONTRACT.clone(),
        BROADCAST_STACKS_TRANSACTION.clone(),
        CALL_READONLY_FN.clone(),
        SET_DEFAULT_NETWORK.clone(),
        SEND_CONTRACT_CALL.clone(),
    ];
}

pub fn encode_contract_call(
    spec: &CommandSpecification,
    function_name: &str,
    function_args_values: &Vec<Value>,
    network_id: &str,
    contract_id_value: &Value,
) -> Result<Value, Diagnostic> {
    // Extract contract_address
    let contract_id = match contract_id_value {
        Value::Primitive(PrimitiveValue::Buffer(contract_id)) => {
            match parse_clarity_value(&contract_id.bytes, &contract_id.typing).unwrap() {
                clarity::vm::Value::Principal(PrincipalData::Contract(c)) => c,
                cv => {
                    return Err(diagnosed_error!(
                        "command {}: unexpected clarity value {cv}",
                        spec.matcher
                    ))
                }
            }
        }
        Value::Primitive(PrimitiveValue::String(contract_id)) => {
            match clarity::vm::types::QualifiedContractIdentifier::parse(contract_id) {
                Ok(v) => v,
                Err(e) => {
                    return Err(diagnosed_error!(
                        "command {}: error parsing contract_id {}",
                        spec.matcher,
                        e.to_string()
                    ))
                }
            }
        }
        _ => {
            return Err(diagnosed_error!(
                "command {}: attribute 'contract_id' expecting type string",
                spec.matcher
            ))
        }
    };

    // validate contract_id against network_id
    let id_str = contract_id.to_string();
    let mainnet_match = id_str.starts_with("SP") && network_id.eq("mainnet");
    let testnet_match = id_str.starts_with("ST") && !network_id.eq("mainnet");

    if !mainnet_match && !testnet_match {
        return Err(diagnosed_error!(
            "command {}: contract id {} is not valid for network {}",
            spec.matcher,
            id_str,
            network_id
        ));
    }

    let mut function_args = vec![];
    for arg_value in function_args_values.iter() {
        let Some(buffer) = arg_value.as_buffer_data() else {
            return Err(diagnosed_error!(
                "function '{}': expected array, got {:?}",
                spec.matcher,
                arg_value
            ));
        };
        let arg = parse_clarity_value(&buffer.bytes, &buffer.typing).unwrap();
        function_args.push(arg);
    }

    let payload = TransactionPayload::ContractCall(TransactionContractCall {
        contract_name: contract_id.name.clone(),
        address: StacksAddress::from(contract_id.issuer.clone()),
        function_name: ClarityName::try_from(function_name).unwrap(),
        function_args,
    });

    let mut bytes = vec![];
    payload.consensus_serialize(&mut bytes).unwrap();
    let value = Value::buffer(bytes, STACKS_CONTRACT_CALL.clone());

    Ok(value)
}

fn get_signing_construct_did(args: &ValueStore) -> Result<ConstructDid, Diagnostic> {
    let signer = args.get_expected_string("signer")?;
    let signing_construct_did = ConstructDid(Did::from_hex_string(signer));
    Ok(signing_construct_did)
}
