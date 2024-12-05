use alloy::{
    dyn_abi::{DynSolValue, JsonAbiExt},
    json_abi::JsonAbi,
    primitives::Address,
};
use alloy_rpc_types::TransactionRequest;
use txtx_addon_kit::{
    indexmap::IndexMap,
    types::{stores::ValueStore, types::Value},
};

use crate::{
    commands::actions::get_expected_address, constants::CONTRACT_CONSTRUCTOR_ARGS, typing::EvmValue,
};

use super::{get_typed_transaction_bytes, value_to_sol_value};

pub mod create_opts;
pub mod proxy_opts;

pub fn get_contract_init_code(
    args: &ValueStore,
    is_proxy_contract: bool,
) -> Result<Vec<u8>, String> {
    let contract = args.get_expected_object("contract").map_err(|e| e.to_string())?;
    let constructor_args = if let Some(function_args) = args.get_value(CONTRACT_CONSTRUCTOR_ARGS) {
        if is_proxy_contract {
            return Err(format!(
                "invalid arguments: constructor arguments provided, but contract is a proxy contract"
            ));
        }
        let sol_args = function_args
            .expect_array()
            .iter()
            .map(|v| value_to_sol_value(&v))
            .collect::<Result<Vec<DynSolValue>, String>>()?;
        Some(sol_args)
    } else {
        None
    };

    let Some(bytecode) =
        contract.get("bytecode").and_then(|code| Some(code.expect_string().to_string()))
    else {
        return Err(format!("contract missing required bytecode"));
    };

    // if we have an abi available in the contract, parse it out
    let json_abi: Option<JsonAbi> = match contract.get("abi") {
        Some(abi_string) => {
            let abi = serde_json::from_str(&abi_string.expect_string())
                .map_err(|e| format!("failed to decode contract abi: {e}"))?;
            Some(abi)
        }
        None => None,
    };
    create_init_code(bytecode, constructor_args, json_abi)
}

pub fn create_init_code(
    bytecode: String,
    constructor_args: Option<Vec<DynSolValue>>,
    json_abi: &Option<JsonAbi>,
) -> Result<Vec<u8>, String> {
    let mut init_code = alloy::hex::decode(bytecode).map_err(|e| e.to_string())?;
    if let Some(constructor_args) = constructor_args {
        // if we have an abi, use it to validate the constructor arguments
        let mut abi_encoded_args = if let Some(json_abi) = json_abi {
            if let Some(constructor) = &json_abi.constructor {
                constructor
                    .abi_encode_input(&constructor_args)
                    .map_err(|e| format!("failed to encode constructor args: {e}"))?
            } else {
                return Err(format!(
                    "invalid arguments: constructor arguments provided, but abi has no constructor"
                ));
            }
        } else {
            constructor_args.iter().flat_map(|s| s.abi_encode()).collect::<Vec<u8>>()
        };

        init_code.append(&mut abi_encoded_args);
    } else {
        // if we have an abi, use it to validate whether constructor arguments are needed
        if let Some(json_abi) = json_abi {
            if let Some(constructor) = &json_abi.constructor {
                if constructor.inputs.len() > 0 {
                    return Err(format!(
                        "invalid arguments: no constructor arguments provided, but abi has constructor"
                    ));
                }
            }
        }
    };
    Ok(init_code)
}

pub enum ContractDeploymentTransaction {
    Create2(ContractDeploymentTransactionStatus),
    Create(ContractDeploymentTransactionStatus),
}
impl ContractDeploymentTransaction {
    pub fn to_value(&self) -> Result<Value, String> {
        match self {
            ContractDeploymentTransaction::Create(status)
            | ContractDeploymentTransaction::Create2(status) => status.to_value(),
        }
    }
    pub fn contract_address(&self) -> Address {
        match self {
            ContractDeploymentTransaction::Create(status)
            | ContractDeploymentTransaction::Create2(status) => match status {
                ContractDeploymentTransactionStatus::AlreadyDeployed(address) => address.clone(),
                ContractDeploymentTransactionStatus::NotYetDeployed(data) => data.expected_address,
            },
        }
    }
}

pub enum ContractDeploymentTransactionStatus {
    AlreadyDeployed(Address),
    NotYetDeployed(TransactionDeploymentRequestData),
}

impl ContractDeploymentTransactionStatus {
    pub fn to_value(&self) -> Result<Value, String> {
        let mut object = IndexMap::new();
        match self {
            ContractDeploymentTransactionStatus::AlreadyDeployed(address) => {
                object.insert("contract_address".to_string(), EvmValue::address(&address));
                object.insert("already_deployed".to_string(), Value::bool(true));
            }
            ContractDeploymentTransactionStatus::NotYetDeployed(data) => {
                object.insert("tx_cost".to_string(), Value::integer(data.tx_cost));
                object.insert(
                    "contract_address".to_string(),
                    EvmValue::address(&data.expected_address),
                );
                object.insert("already_deployed".to_string(), Value::bool(false));
                let tx_payload =
                    get_typed_transaction_bytes(&data.tx).map_err(|e| e.to_string())?;
                object.insert("tx_payload".to_string(), EvmValue::transaction(tx_payload));
            }
        }
        Ok(Value::object(object))
    }

    pub fn from_value(value: &Value) -> Result<Self, String> {
        let object = value.as_object().ok_or("expected object")?;
        if let Some(tx_cost) = object.get("tx_cost") {
            let tx_cost = tx_cost.as_integer().ok_or("tx_cost must be an integer")?;
            let expected_address = get_expected_address(
                object.get("contract_address").ok_or("missing contract_address")?,
            )?;
            let tx_payload = object
                .get("tx_payload")
                .ok_or("missing tx_payload")?
                .as_buffer_data()
                .ok_or("tx_payload must be a buffer")?;
            let tx: TransactionRequest = serde_json::from_slice(tx_payload)
                .map_err(|e| format!("failed to decode tx: {e}"))?;

            Ok(ContractDeploymentTransactionStatus::NotYetDeployed(
                TransactionDeploymentRequestData { tx, tx_cost, expected_address },
            ))
        } else {
            let expected_address = get_expected_address(
                object.get("contract_address").ok_or("missing contract_address")?,
            )?;
            Ok(ContractDeploymentTransactionStatus::AlreadyDeployed(expected_address))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionDeploymentRequestData {
    pub tx: TransactionRequest,
    pub tx_cost: i128,
    pub expected_address: Address,
}
