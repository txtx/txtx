pub mod foundry;

use crate::commands::actions::get_expected_address;
use crate::constants::{GAS_PRICE, MAX_FEE_PER_GAS, MAX_PRIORITY_FEE_PER_GAS};
use crate::rpc::EVMRpc;
use crate::typing::{BYTES, BYTES32, ETH_ADDRESS};
use alloy::dyn_abi::{DynSolValue, Word};
use alloy::hex::FromHex;
use alloy::network::TransactionBuilder;
use alloy::primitives::{Address, TxKind, U256};
use alloy::rpc::types::TransactionRequest;
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::types::types::{PrimitiveValue, Value};
use txtx_addon_kit::types::ValueStore;

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
            other => Err(diagnosed_error!(
                "invalid Ethereum Transaction type: {}",
                other
            )),
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
    rpc: EVMRpc,
    args: &ValueStore,
    fields: CommonTransactionFields,
) -> Result<TransactionRequest, String> {
    let from = get_expected_address(&fields.from)
        .map_err(|e| format!("failed to parse to address: {e}"))?;
    let to = if let Some(to) = fields.to {
        Some(get_expected_address(&to).map_err(|e| format!("failed to parse to address: {e}"))?)
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
    Ok(tx)
}

async fn build_unsigned_legacy_transaction(
    rpc: &EVMRpc,
    args: &ValueStore,
    fields: &FilledCommonTransactionFields,
) -> Result<TransactionRequest, String> {
    let gas_price = args.get_value(GAS_PRICE).map(|v| v.expect_uint());

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
    rpc: &EVMRpc,
    args: &ValueStore,
    fields: &FilledCommonTransactionFields,
) -> Result<TransactionRequest, String> {
    let max_fee_per_gas = args.get_value(MAX_FEE_PER_GAS).map(|v| v.expect_uint());
    let max_priority_fee_per_gas = args
        .get_value(MAX_PRIORITY_FEE_PER_GAS)
        .map(|v| v.expect_uint());

    let (max_fee_per_gas, max_priority_fee_per_gas) =
        if max_fee_per_gas.is_none() || max_priority_fee_per_gas.is_none() {
            let fees = rpc
                .estimate_eip1559_fees()
                .await
                .map_err(|e| e.to_string())?;

            (
                max_fee_per_gas
                    .and_then(|f| Some(f as u128))
                    .unwrap_or(fees.max_fee_per_gas),
                max_priority_fee_per_gas
                    .and_then(|f| Some(f as u128))
                    .unwrap_or(fees.max_priority_fee_per_gas),
            )
        } else {
            (
                max_fee_per_gas.unwrap() as u128,
                max_priority_fee_per_gas.unwrap() as u128,
            )
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
    rpc: &EVMRpc,
    mut tx: TransactionRequest,
    gas_limit: Option<u64>,
) -> Result<TransactionRequest, String> {
    if let Some(gas_limit) = gas_limit {
        tx = tx.with_gas_limit(gas_limit.into());
    } else {
        let gas_limit = rpc.estimate_gas(&tx).await.map_err(|e| e.to_string())?;
        tx = tx.with_gas_limit(gas_limit.into());
    }
    Ok(tx)
}

pub fn get_typed_transaction_bytes(tx: &TransactionRequest) -> Result<Vec<u8>, String> {
    serde_json::to_vec(&tx).map_err(|e| format!("failed to serialized transaction: {}", e))
}

pub fn value_to_sol_value(value: &Value) -> Result<DynSolValue, String> {
    let sol_value = match value {
        Value::Primitive(PrimitiveValue::Bool(value)) => DynSolValue::Bool(value.clone()),
        Value::Primitive(PrimitiveValue::SignedInteger(value)) => todo!(),
        Value::Primitive(PrimitiveValue::UnsignedInteger(value)) => {
            DynSolValue::Uint(U256::from(*value), 256)
        }
        Value::Primitive(PrimitiveValue::String(value)) => DynSolValue::String(value.clone()),
        Value::Primitive(PrimitiveValue::Float(value)) => todo!(),
        Value::Primitive(PrimitiveValue::Buffer(value)) => DynSolValue::Bytes(value.bytes.clone()),
        Value::Primitive(PrimitiveValue::Null) => {
            todo!()
        }
        Value::Object(_) => todo!(),
        Value::Array(_) => todo!(),
        Value::Addon(addon) => {
            if addon.typing.id == ETH_ADDRESS.clone().id {
                DynSolValue::Address(Address::from_slice(&addon.value.expect_buffer_bytes()))
            } else if addon.typing.id == BYTES32.clone().id {
                let value = addon.value.as_buffer_data().unwrap();
                let word = Word::from_slice(&value.bytes);
                DynSolValue::FixedBytes(word, 32)
            } else if addon.typing.id == BYTES.clone().id {
                let value = addon.value.as_buffer_data().unwrap();
                DynSolValue::Bytes(value.bytes.clone())
            } else {
                todo!()
            }
        }
    };
    Ok(sol_value)
}

#[allow(dead_code)]
pub fn sol_value_to_value(sol_value: &DynSolValue) -> Result<Value, String> {
    let value = match sol_value {
        DynSolValue::Bool(value) => Value::bool(*value),
        DynSolValue::Int(value, _) => Value::int(value.as_i64()),
        DynSolValue::Uint(value, _) => Value::uint(value.to::<u64>()),
        DynSolValue::FixedBytes(_, _) => todo!(),
        DynSolValue::Address(value) => Value::buffer(value.0 .0.to_vec(), ETH_ADDRESS.clone()),
        DynSolValue::Function(_) => todo!(),
        DynSolValue::Bytes(_) => todo!(),
        DynSolValue::String(value) => Value::string(value.clone()),
        DynSolValue::Array(_) => todo!(),
        DynSolValue::FixedArray(_) => todo!(),
        DynSolValue::Tuple(_) => todo!(),
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
