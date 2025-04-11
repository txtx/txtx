pub mod contract_deployment;
pub mod crypto;
pub mod foundry;
pub mod hardhat;
pub mod verify;

use std::collections::VecDeque;
use std::num::NonZeroUsize;

use crate::commands::actions::get_expected_address;
use crate::constants::{GAS_PRICE, MAX_FEE_PER_GAS, MAX_PRIORITY_FEE_PER_GAS};
use crate::rpc::EvmRpc;
use crate::typing::{
    DecodedLog, EvmValue, EVM_ADDRESS, EVM_BYTES, EVM_BYTES32, EVM_FUNCTION_CALL, EVM_INIT_CODE,
    EVM_KNOWN_SOL_PARAM, EVM_SIM_RESULT, EVM_UINT256, EVM_UINT32, EVM_UINT8,
};
use alloy::consensus::{SignableTransaction, Transaction, TypedTransaction};
use alloy::dyn_abi::parser::TypeSpecifier;
use alloy::dyn_abi::{DynSolValue, EventExt, FunctionExt, Word};
use alloy::hex::{self, FromHex};
use alloy::json_abi::{Constructor, JsonAbi, Param};
use alloy::network::TransactionBuilder;
use alloy::primitives::utils::format_units;
use alloy::primitives::{Address, TxKind, U256};
use alloy::rpc::types::TransactionRequest;
use alloy_rpc_types::{AccessList, Log};
use contract_deployment::AddressAbiMap;
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::{ObjectType, Value};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransactionType {
    Legacy,
    EIP2930,
    EIP1559,
    EIP4844,
}

impl TransactionType {
    pub fn from_some_value(input: Option<&str>) -> Result<Self, Diagnostic> {
        input
            .and_then(|t| Some(TransactionType::from_str(t)))
            .unwrap_or(Ok(TransactionType::EIP1559))
    }
    pub fn from_str(input: &str) -> Result<Self, Diagnostic> {
        match input.to_ascii_lowercase().as_ref() {
            "legacy" => Ok(TransactionType::Legacy),
            "eip2930" => Ok(TransactionType::EIP2930),
            "eip1559" => Ok(TransactionType::EIP1559),
            "eip4844" => Ok(TransactionType::EIP4844),
            other => Err(diagnosed_error!("invalid Ethereum Transaction type: {}", other)),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommonTransactionFields {
    pub to: Option<Value>,
    pub from: Value,
    pub nonce: Option<u64>,
    pub chain_id: u64,
    pub amount: u64,
    pub gas_limit: Option<u64>,
    pub input: Option<Vec<u8>>,
    pub tx_type: TransactionType,
    pub deploy_code: Option<Vec<u8>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct FilledCommonTransactionFields {
    pub to: Option<Address>,
    pub from: Address,
    pub nonce: u64,
    pub chain_id: u64,
    pub amount: u64,
    pub gas_limit: Option<u64>,
    pub input: Option<Vec<u8>>,
    pub deploy_code: Option<Vec<u8>>,
}
pub async fn build_unsigned_transaction(
    rpc: EvmRpc,
    args: &ValueStore,
    fields: CommonTransactionFields,
) -> Result<(TransactionRequest, i128, String), String> {
    let from = get_expected_address(&fields.from)
        .map_err(|e| format!("failed to parse 'from' address: {e}"))?;
    let to = if let Some(to) = fields.to {
        Some(get_expected_address(&to).map_err(|e| format!("failed to parse 'to' address: {e}"))?)
    } else {
        None
    };

    let nonce = match fields.nonce {
        Some(nonce) => nonce,
        None => rpc.get_nonce(&from).await.map_err(|e| e.to_string())?,
    };

    let filled_fields = FilledCommonTransactionFields {
        to,
        from,
        nonce,
        chain_id: fields.chain_id,
        amount: fields.amount,
        gas_limit: fields.gas_limit,
        input: fields.input,
        deploy_code: fields.deploy_code,
    };

    let mut tx = match fields.tx_type {
        TransactionType::Legacy => {
            build_unsigned_legacy_transaction(&rpc, args, &filled_fields).await?
        }
        TransactionType::EIP2930 => {
            println!("Unsupported tx type EIP2930 was used. Defaulting to EIP1559 tx");
            build_unsigned_eip1559_transaction(&rpc, args, &filled_fields).await?
        }
        TransactionType::EIP1559 => {
            build_unsigned_eip1559_transaction(&rpc, args, &filled_fields).await?
        }
        TransactionType::EIP4844 => {
            println!("Unsupported tx type EIP4844 was used. Defaulting to EIP1559 tx");
            build_unsigned_eip1559_transaction(&rpc, args, &filled_fields).await?
        }
    };

    // set gas limit _after_ all other fields have been set to get an accurate estimate
    tx = set_gas_limit(&rpc, tx, fields.gas_limit).await?;

    let typed_transaction =
        tx.clone().build_unsigned().map_err(|e| format!("failed to build transaction: {e}"))?;
    let cost = get_transaction_cost(&typed_transaction, &rpc).await?;

    let sim = rpc
        .call(&tx)
        .await
        .map_err(|e| format!("failed to simulate transaction: {}", e.to_string()))?;
    Ok((tx, cost, sim))
}

async fn build_unsigned_legacy_transaction(
    rpc: &EvmRpc,
    args: &ValueStore,
    fields: &FilledCommonTransactionFields,
) -> Result<TransactionRequest, String> {
    let gas_price = args.get_value(GAS_PRICE).map(|v| v.expect_uint()).transpose()?;

    let gas_price = match gas_price {
        Some(gas_price) => gas_price as u128,
        None => rpc.get_gas_price().await.map_err(|e| e.to_string())?,
    };
    let mut tx = TransactionRequest::default()
        .with_from(fields.from)
        .with_value(U256::from(fields.amount))
        .with_nonce(fields.nonce)
        .with_chain_id(fields.chain_id)
        .with_gas_price(gas_price);

    if let Some(to) = fields.to {
        tx = tx.with_to(to);
    }
    if let Some(input) = &fields.input {
        tx = tx.with_input(input.clone());
    }
    if let Some(code) = &fields.deploy_code {
        tx = tx.with_deploy_code(code.clone()).with_kind(TxKind::Create);
    }
    Ok(tx)
}

async fn build_unsigned_eip1559_transaction(
    rpc: &EvmRpc,
    args: &ValueStore,
    fields: &FilledCommonTransactionFields,
) -> Result<TransactionRequest, String> {
    let max_fee_per_gas = args.get_value(MAX_FEE_PER_GAS).map(|v| v.expect_uint()).transpose()?;
    let max_priority_fee_per_gas =
        args.get_value(MAX_PRIORITY_FEE_PER_GAS).map(|v| v.expect_uint()).transpose()?;

    let (max_fee_per_gas, max_priority_fee_per_gas) =
        if max_fee_per_gas.is_none() || max_priority_fee_per_gas.is_none() {
            let fees = rpc.estimate_eip1559_fees().await.map_err(|e| e.to_string())?;

            (
                max_fee_per_gas.and_then(|f| Some(f as u128)).unwrap_or(fees.max_fee_per_gas),
                max_priority_fee_per_gas
                    .and_then(|f| Some(f as u128))
                    .unwrap_or(fees.max_priority_fee_per_gas),
            )
        } else {
            (max_fee_per_gas.unwrap() as u128, max_priority_fee_per_gas.unwrap() as u128)
        };

    let mut tx = TransactionRequest::default()
        .with_from(fields.from)
        .with_value(U256::from(fields.amount))
        .with_nonce(fields.nonce)
        .with_chain_id(fields.chain_id)
        .max_fee_per_gas(max_fee_per_gas)
        .with_max_priority_fee_per_gas(max_priority_fee_per_gas);

    if let Some(to) = fields.to {
        tx = tx.with_to(to);
    }
    if let Some(input) = &fields.input {
        tx = tx.with_input(input.clone());
    }
    if let Some(code) = &fields.deploy_code {
        tx = tx.with_deploy_code(code.clone()).with_kind(TxKind::Create);
    }

    Ok(tx)
}

async fn set_gas_limit(
    rpc: &EvmRpc,
    mut tx: TransactionRequest,
    gas_limit: Option<u64>,
) -> Result<TransactionRequest, String> {
    if let Some(gas_limit) = gas_limit {
        tx = tx.with_gas_limit(gas_limit.into());
    } else {
        let call_res = rpc.call(&tx).await;

        let gas_limit = rpc.estimate_gas(&tx).await.map_err(|estimate_err| match call_res {
            Ok(res) => format!(
                "failed to estimate gas: {};\nsimulation results: {}",
                estimate_err.to_string(),
                res
            ),
            Err(e) => format!(
                "failed to estimate gas: {};\nfailed to simulate transaction: {}",
                estimate_err.to_string(),
                e.to_string()
            ),
        })?;
        tx = tx.with_gas_limit(gas_limit.into());
    }
    Ok(tx)
}

pub fn get_typed_transaction_bytes(tx: &TransactionRequest) -> Result<Vec<u8>, String> {
    serde_json::to_vec(&tx).map_err(|e| format!("failed to serialized transaction: {}", e))
}

pub fn value_to_abi_function_args(
    function_name: &str,
    value: &Value,
    abi: &JsonAbi,
) -> Result<Vec<DynSolValue>, Diagnostic> {
    let function = abi
        .function(function_name)
        .and_then(|r| r.first())
        .ok_or(diagnosed_error!("function {function_name} not found in abi"))?;

    let values =
        value.as_array().ok_or(diagnosed_error!("expected array for function argument"))?;

    if values.len() != function.inputs.len() {
        return Err(diagnosed_error!(
            "expected {} values for function arguments, found {}",
            function.inputs.len(),
            values.len()
        ));
    }
    value_to_abi_params(values, &function.inputs)
        .map_err(|e| diagnosed_error!("failed to encode function arguments: {e}"))
}

pub fn value_to_abi_constructor_args(
    value: &Value,
    abi_constructor: &Constructor,
) -> Result<Vec<DynSolValue>, Diagnostic> {
    let values =
        value.as_array().ok_or(diagnosed_error!("expected array for constructor argument"))?;

    if values.len() != abi_constructor.inputs.len() {
        return Err(diagnosed_error!(
            "expected {} values for constructor arguments, found {}",
            abi_constructor.inputs.len(),
            values.len()
        ));
    }

    value_to_abi_params(values, &abi_constructor.inputs)
        .map_err(|e| diagnosed_error!("failed to encode constructor arguments: {e}"))
}

pub fn value_to_abi_params(
    values: &Vec<Value>,
    params: &Vec<Param>,
) -> Result<Vec<DynSolValue>, Diagnostic> {
    let mut sol_values = vec![];
    for (i, param) in params.iter().enumerate() {
        let value = values.get(i).ok_or(diagnosed_error!("expected {} arguments", params.len()))?;
        let sol_value = value_to_abi_param(value, param).map_err(|e| {
            diagnosed_error!("failed to encode param #{} (name '{}'): {}", i + 1, param.name, e)
        })?;
        sol_values.push(sol_value);
    }
    Ok(sol_values)
}

pub fn value_to_abi_param(value: &Value, param: &Param) -> Result<DynSolValue, Diagnostic> {
    let msg = format!(
        "failed to convert value {} to {}",
        value.get_type().to_string(),
        param.ty.as_str()
    );

    if let Some(addon_data) = value.as_addon_data() {
        if addon_data.id == EVM_SIM_RESULT {
            let (result, fn_spec) = EvmValue::to_sim_result(value)?;
            if let Some(fn_spec) = fn_spec {
                let res = fn_spec.abi_decode_output(&result).map_err(|e| {
                    diagnosed_error!("{msg}: failed to decode function output: {e}")
                })?;
                if res.len() == 1 {
                    return Ok(res.get(0).unwrap().clone());
                } else {
                    return Ok(DynSolValue::Tuple(res));
                }
            }
        } else if addon_data.id == EVM_KNOWN_SOL_PARAM {
            let (value, param) = EvmValue::to_known_sol_param(value)?;
            return value_to_abi_param(&value, &param).map_err(|e| {
                diagnosed_error!("{msg}: failed to encode known Solidity type: {e}",)
            });
        }
    }

    let type_specifier = TypeSpecifier::try_from(param.ty.as_str())
        .map_err(|e| diagnosed_error!("{msg}:failed to parse type specifier: {e}"))?;
    let is_array = type_specifier.sizes.len() > 0;

    if is_array {
        let values = value.as_array().ok_or(diagnosed_error!("{msg}: expected array"))?;
        let arr_res =
            value_to_array_abi_type(values, &mut VecDeque::from(type_specifier.sizes), &param)
                .map_err(|e| diagnosed_error!("{msg}: {e}"))?;
        Ok(arr_res)
    } else {
        value_to_primitive_abi_type(value, &param)
    }
}

pub fn value_to_primitive_abi_type(
    value: &Value,
    param: &Param,
) -> Result<DynSolValue, Diagnostic> {
    let msg = format!(
        "failed to convert value {} to {}",
        value.get_type().to_string(),
        param.ty.as_str()
    );

    let type_specifier = TypeSpecifier::try_from(param.ty.as_str())
        .map_err(|e| diagnosed_error!("{msg}: failed to parse type specifier: {e}"))?;

    let sol_value = match type_specifier.stem.span() {
        "address" => DynSolValue::Address(EvmValue::to_address(value)?),
        "uint8" => DynSolValue::Uint(
            U256::try_from_be_slice(&value.to_bytes()).ok_or(diagnosed_error!("{msg}"))?,
            8,
        ),
        "uint16" => DynSolValue::Uint(
            U256::try_from_be_slice(&value.to_bytes()).ok_or(diagnosed_error!("{msg}"))?,
            16,
        ),
        "uint32" => DynSolValue::Uint(
            U256::try_from_be_slice(&value.to_bytes()).ok_or(diagnosed_error!("{msg}"))?,
            32,
        ),
        "uint64" => DynSolValue::Uint(
            U256::try_from_be_slice(&value.to_bytes()).ok_or(diagnosed_error!("{msg}"))?,
            64,
        ),
        "uint96" => DynSolValue::Uint(
            U256::try_from_be_slice(&value.to_bytes()).ok_or(diagnosed_error!("{msg}"))?,
            96,
        ),
        "uint256" => DynSolValue::Uint(
            U256::try_from_be_slice(&value.to_bytes()).ok_or(diagnosed_error!("{msg}"))?,
            256,
        ),
        "bytes" => DynSolValue::Bytes(value.to_bytes()),
        "bytes32" => DynSolValue::FixedBytes(Word::from_slice(&value.to_bytes()), 32),
        "bool" => DynSolValue::Bool(value.as_bool().ok_or(diagnosed_error!("{msg}"))?),
        "string" => DynSolValue::String(value.to_string()),
        "tuple" => {
            let mut tuple = vec![];
            let values =
                value.as_array().ok_or(diagnosed_error!("expected array for tuple"))?.clone();
            for (i, component) in param.components.iter().enumerate() {
                let value = values.get(i).ok_or(diagnosed_error!(
                    "expected {} values for tuple argument",
                    param.components.len()
                ))?;
                tuple.push(value_to_abi_param(value, &component).map_err(|e| {
                    diagnosed_error!(
                        "failed to encode tuple component #{} (name '{}'): {}",
                        i + 1,
                        component.name,
                        e
                    )
                })?);
            }

            DynSolValue::Tuple(tuple)
        }
        "struct" => value_to_struct_abi_type(value, param)?,
        _ => return Err(diagnosed_error!("unsupported primitive abi type: {}", param.ty)),
    };
    Ok(sol_value)
}

pub fn value_to_array_abi_type(
    values: &Vec<Value>,
    sizes: &mut VecDeque<Option<NonZeroUsize>>,
    param: &Param,
) -> Result<DynSolValue, Diagnostic> {
    let Some(size) = sizes.pop_back() else {
        return Err(diagnosed_error!("array dimension mismatch or unspecified dimension"));
    };
    let mut arr = vec![];
    if let Some(size) = size {
        let size = size.get();
        if values.len() != size {
            return Err(diagnosed_error!(
                "expected array of length {}, found {}",
                size,
                values.len()
            ));
        }

        for i in 0..size {
            if sizes.len() > 0 {
                let new_value = values[i].clone();
                let new_values = new_value.as_array().ok_or(diagnosed_error!(
                    "expected array, found {}",
                    new_value.get_type().to_string()
                ))?;

                arr.push(value_to_array_abi_type(&new_values, sizes, param)?);
            } else {
                arr.push(value_to_primitive_abi_type(&values[i], param)?);
            }
        }

        Ok(DynSolValue::FixedArray(arr))
    } else {
        for value in values {
            if sizes.len() > 0 {
                let new_value = value.clone();
                let new_values = new_value.as_array().ok_or(diagnosed_error!(
                    "expected array, found {}",
                    new_value.get_type().to_string()
                ))?;
                arr.push(value_to_array_abi_type(&new_values, sizes, param)?);
            } else {
                arr.push(value_to_primitive_abi_type(value, param)?);
            }
        }

        Ok(DynSolValue::Array(arr))
    }
}

pub fn value_to_struct_abi_type(value: &Value, param: &Param) -> Result<DynSolValue, Diagnostic> {
    let mut prop_names = vec![];
    let mut tuple = vec![];
    for component in param.components.iter() {
        let component_name = component.name.clone();
        let component_value = value_to_abi_param(value, &component)?;
        tuple.push(component_value);
        prop_names.push(component_name);
    }
    let sol_value = DynSolValue::CustomStruct { name: param.name.clone(), prop_names, tuple };
    Ok(sol_value)
}

pub fn value_to_sol_value(value: &Value) -> Result<DynSolValue, String> {
    let sol_value = match value {
        Value::Bool(value) => DynSolValue::Bool(value.clone()),
        Value::Integer(value) => DynSolValue::Uint(U256::from(*value), 256),
        Value::String(value) => DynSolValue::String(value.clone()),
        Value::Float(_value) => todo!(),
        Value::Buffer(bytes) => DynSolValue::Bytes(bytes.clone()),
        Value::Null => {
            todo!()
        }
        Value::Object(_) => todo!(),
        Value::Array(values) => DynSolValue::Array(
            values.iter().map(value_to_sol_value).collect::<Result<Vec<_>, _>>()?,
        ),
        Value::Addon(addon) => {
            if addon.id == EVM_ADDRESS {
                DynSolValue::Address(Address::from_slice(&addon.bytes))
            } else if addon.id == EVM_BYTES32 {
                DynSolValue::FixedBytes(Word::from_slice(&addon.bytes), 32)
            } else if addon.id == EVM_UINT256 {
                DynSolValue::Uint(U256::from_be_slice(&addon.bytes), 256)
            } else if addon.id == EVM_UINT32 {
                DynSolValue::Uint(U256::from_be_slice(&addon.bytes), 32)
            } else if addon.id == EVM_UINT8 {
                DynSolValue::Uint(U256::from_be_slice(&addon.bytes), 8)
            } else if addon.id == EVM_BYTES
                || addon.id == EVM_INIT_CODE
                || addon.id == EVM_FUNCTION_CALL
            {
                DynSolValue::Bytes(addon.bytes.clone())
            } else if addon.id == EVM_SIM_RESULT {
                let (result, fn_spec) =
                    EvmValue::to_sim_result(value).map_err(|e| format!("{}", e))?;
                if let Some(fn_spec) = fn_spec {
                    let res = fn_spec
                        .abi_decode_output(&result)
                        .map_err(|e| format!("failed to decode function output: {}", e))?;
                    if res.len() == 1 {
                        res.get(0).unwrap().clone()
                    } else {
                        DynSolValue::Tuple(res)
                    }
                } else {
                    DynSolValue::Bytes(result)
                }
            } else if addon.id == EVM_KNOWN_SOL_PARAM {
                let (value, param) =
                    EvmValue::to_known_sol_param(value).map_err(|e| format!("{}", e))?;
                value_to_abi_param(&value, &param).map_err(|e| format!("{}", e))?
            } else {
                return Err(format!(
                    "unsupported addon type for encoding Solidity value: {}",
                    addon.id
                ));
            }
        }
    };
    Ok(sol_value)
}

pub fn sol_value_to_value(sol_value: &DynSolValue) -> Result<Value, Diagnostic> {
    let value = match sol_value {
        DynSolValue::Bool(value) => Value::bool(*value),
        DynSolValue::Int(value, _) => Value::integer(value.as_i64() as i128),
        DynSolValue::Uint(value, _) => {
            let res: Result<u64, _> = value.try_into();
            match res {
                Ok(v) => Value::integer(v as i128),
                Err(_) => Value::string(value.to_string()),
            }
        }
        DynSolValue::FixedBytes(_, _) => todo!(),
        DynSolValue::Address(value) => EvmValue::address(&value),
        DynSolValue::Function(_) => todo!(),
        DynSolValue::Bytes(_) => todo!(),
        DynSolValue::String(value) => Value::string(value.clone()),
        DynSolValue::Array(values) => {
            Value::array(values.iter().map(sol_value_to_value).collect::<Result<Vec<_>, _>>()?)
        }
        DynSolValue::FixedArray(_) => todo!(),
        DynSolValue::Tuple(_) => todo!(),
        DynSolValue::CustomStruct { name, prop_names, tuple } => ObjectType::from(vec![(
            &name,
            ObjectType::from_map(
                tuple
                    .iter()
                    .map(|v| sol_value_to_value(v))
                    .collect::<Result<Vec<_>, _>>()?
                    .iter()
                    .zip(prop_names)
                    .map(|(v, k)| (k.clone(), v.clone()))
                    .collect(),
            )
            .to_value(),
        )])
        .to_value(),
    };
    Ok(value)
}

pub fn string_to_address(address_str: String) -> Result<Address, String> {
    let mut address_str = address_str.replace("0x", "");
    // hack: we're assuming that if the address is 32 bytes, it's a sol value that's padded with 0s, so we trim them
    if address_str.len() == 64 {
        let split_pos = address_str.char_indices().nth_back(39).unwrap().0;
        address_str = address_str[split_pos..].to_owned();
    }
    let address = Address::from_hex(&address_str).map_err(|e| format!("invalid address: {}", e))?;
    Ok(address)
}

pub fn typed_transaction_bytes(typed_transaction: &TypedTransaction) -> Vec<u8> {
    let mut bytes = vec![];
    match typed_transaction {
        TypedTransaction::Legacy(tx) => tx.encode_for_signing(&mut bytes),
        TypedTransaction::Eip2930(tx) => tx.encode_for_signing(&mut bytes),
        TypedTransaction::Eip1559(tx) => tx.encode_for_signing(&mut bytes),
        TypedTransaction::Eip4844(tx) => tx.encode_for_signing(&mut bytes),
        TypedTransaction::Eip7702(tx) => tx.encode_for_signing(&mut bytes),
    }
    bytes
}
pub fn format_transaction_for_display(typed_transaction: &TypedTransaction) -> Value {
    let mut res = ObjectType::from(vec![
        (
            "kind",
            match typed_transaction.to() {
                None => Value::string("create".to_string()),
                Some(address) => Value::string(format!("to:{}", address.to_string())),
            },
        ),
        ("nonce", Value::integer(typed_transaction.nonce() as i128)),
        ("gas_limit", Value::integer(typed_transaction.gas_limit() as i128)),
        ("input", Value::string(hex::encode(&typed_transaction.input()))),
        ("value", Value::string(format_units(typed_transaction.value(), "ether").unwrap())),
        ("type", Value::string(typed_transaction.tx_type().to_string())),
    ]);
    if let Some(chain_id) = typed_transaction.chain_id() {
        res.insert("chain_id", Value::integer(chain_id as i128));
    }
    match typed_transaction {
        TypedTransaction::Legacy(tx) => {
            if let Some(gas_price) = tx.gas_price() {
                res.insert("gas_price", Value::integer(gas_price as i128));
            }
        }
        TypedTransaction::Eip2930(tx) => {
            res.insert(
                "access_list",
                Value::array(format_access_list_for_display(&tx.access_list)),
            );
        }
        TypedTransaction::Eip1559(tx) => {
            res.insert(
                "access_list",
                Value::array(format_access_list_for_display(&tx.access_list)),
            );
            res.insert("max_fee_per_gas", Value::integer(tx.max_fee_per_gas as i128));
            res.insert(
                "max_priority_fee_per_gas",
                Value::integer(tx.max_priority_fee_per_gas as i128),
            );
        }
        TypedTransaction::Eip4844(_tx) => {
            unimplemented!("EIP-4844 is not supported");
        }
        TypedTransaction::Eip7702(_tx) => {
            unimplemented!("EIP-7702 is not supported");
        }
    }
    res.to_value()
}

pub fn format_access_list_for_display(access_list: &AccessList) -> Vec<Value> {
    access_list
        .0
        .iter()
        .map(|item| {
            ObjectType::from(vec![
                ("address", Value::string(item.address.to_string())),
                (
                    "storage_keys",
                    Value::array(
                        item.storage_keys
                            .iter()
                            .map(|key| Value::string(hex::encode(key.0)))
                            .collect::<Vec<Value>>(),
                    ),
                ),
            ])
            .to_value()
        })
        .collect::<Vec<Value>>()
}

pub async fn get_transaction_cost(
    transaction: &TypedTransaction,
    rpc: &EvmRpc,
) -> Result<i128, String> {
    let effective_gas_price = match &transaction {
        TypedTransaction::Legacy(tx) => tx.gas_price,
        TypedTransaction::Eip2930(tx) => tx.gas_price,
        TypedTransaction::Eip1559(tx) => {
            let base_fee = rpc.get_base_fee_per_gas().await.map_err(|e| e.to_string())?;
            tx.effective_gas_price(Some(base_fee as u64))
        }
        TypedTransaction::Eip4844(_tx) => unimplemented!("EIP-4844 is not supported"),
        TypedTransaction::Eip7702(_tx) => unimplemented!("EIP-7702 is not supported"),
    };
    let gas_limit = transaction.gas_limit();
    let cost: i128 = effective_gas_price as i128 * gas_limit as i128;
    Ok(cost)
}

pub fn format_transaction_cost(cost: i128) -> Result<String, String> {
    format_units(cost, "wei").map_err(|e| format!("failed to format cost: {e}"))
}

/// Decodes logs using the provided ABI map.
/// The ABI map should be a [Value::Array] of [Value::Object]s, where each object has keys "address" (storing an [EvmValue::address]) and "abis" (storing a [Value::array] or abi strings).
pub fn abi_decode_logs(abi_map: &Value, logs: &[Log]) -> Result<Vec<Value>, String> {
    let abi_map = AddressAbiMap::parse_value(abi_map)
        .map_err(|e| format!("invalid abis for transaction: {e}"))?;

    let logs = logs
        .iter()
        .filter_map(|log| {
            let log_address = log.address();

            let Some(abis) = abi_map.get(&log_address) else {
                return None;
            };

            let topics = log.inner.topics();
            let Some(first_topic) = topics.first() else { return None };
            let Some(matching_event) =
                abis.iter().find_map(|abi| abi.events().find(|e| e.selector().eq(first_topic)))
            else {
                return None;
            };

            let decoded = match matching_event
                .decode_log(&log.data())
                .map_err(|e| format!("failed to decode log: {e}"))
            {
                Ok(decoded) => decoded,
                Err(e) => return Some(Err(e)),
            };
            let mut entries = vec![];
            for (data, event) in decoded.body.iter().zip(matching_event.inputs.iter()) {
                let value = match sol_value_to_value(data) {
                    Ok(value) => value,
                    Err(e) => return Some(Err(e.message)),
                };
                entries.push((&event.name, value));
            }

            return Some(Ok(DecodedLog::to_value(
                &matching_event.name,
                &log_address,
                ObjectType::from(entries).to_value(),
            )));
        })
        .collect::<Result<Vec<Value>, String>>()?;
    Ok(logs)
}
