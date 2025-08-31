use alloy::dyn_abi::{DynSolValue, EventExt};
use alloy_rpc_types::Log;
use error_stack::{Report, ResultExt};
use txtx_addon_kit::types::types::{ObjectType, Value};

use crate::codec::contract_deployment::AddressAbiMap;
use crate::errors::{EvmError, EvmResult, CodecError};
use crate::typing::{DecodedLog, EvmValue};

/// Decodes logs using the provided ABI map.
/// The ABI map should be a [Value::Array] of [Value::Object]s, where each object has keys "address" (storing an [EvmValue::address]) and "abis" (storing a [Value::array] or abi strings).
pub fn abi_decode_logs(abi_map: &Value, logs: &[Log]) -> EvmResult<Vec<Value>> {
    let abi_map = AddressAbiMap::parse_value(abi_map)
        .map_err(|e| Report::new(EvmError::Codec(
            CodecError::AbiDecodingFailed(format!("Invalid ABI map: {}", e))
        )))
        .attach_printable("Parsing ABI map for log decoding")?;

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
                .map_err(|e| Report::new(EvmError::Codec(
                    CodecError::AbiDecodingFailed(format!("Failed to decode log: {}", e))
                )))
                .attach_printable(format!("Decoding event '{}' at address {}", 
                    matching_event.name, log_address))
            {
                Ok(decoded) => decoded,
                Err(e) => return Some(Err(e)),
            };
            
            let mut entries = vec![];
            for (data, event) in decoded.body.iter().zip(matching_event.inputs.iter()) {
                let value = match sol_value_to_value(data)
                    .attach_printable(format!("Converting event parameter '{}'", event.name)) 
                {
                    Ok(value) => value,
                    Err(e) => return Some(Err(e)),
                };
                entries.push((&event.name, value));
            }

            Some(Ok(DecodedLog::to_value(
                &matching_event.name,
                &log_address,
                ObjectType::from(entries).to_value(),
            )))
        })
        .collect::<Result<Vec<Value>, _>>()?;
    Ok(logs)
}

pub fn sol_value_to_value(sol_value: &DynSolValue) -> EvmResult<Value> {
    let context = format!("Converting Solidity value of type {:?}", sol_value_type_name(sol_value));
    
    let value = match sol_value {
        DynSolValue::Bool(value) => Value::bool(*value),
        DynSolValue::Int(value, bits) => {
            Value::integer(value.as_i64() as i128)
        },
        DynSolValue::Uint(value, bits) => {
            let res: Result<u64, _> = value.try_into();
            match res {
                Ok(v) => Value::integer(v as i128),
                Err(_) => Value::string(value.to_string()),
            }
        },
        DynSolValue::FixedBytes(bytes, size) => {
            return Err(Report::new(EvmError::Codec(
                CodecError::UnsupportedAbiType(format!("bytes{}", size))
            )))
            .attach_printable("FixedBytes conversion not yet implemented");
        },
        DynSolValue::Address(value) => EvmValue::address(&value),
        DynSolValue::Function(_) => {
            return Err(Report::new(EvmError::Codec(
                CodecError::UnsupportedAbiType("function".to_string())
            )))
            .attach_printable("Function type conversion not yet implemented");
        },
        DynSolValue::Bytes(bytes) => {
            return Err(Report::new(EvmError::Codec(
                CodecError::UnsupportedAbiType("bytes".to_string())
            )))
            .attach_printable("Dynamic bytes conversion not yet implemented");
        },
        DynSolValue::String(value) => Value::string(value.clone()),
        DynSolValue::Array(values) => {
            let converted = values.iter()
                .enumerate()
                .map(|(i, v)| sol_value_to_value(v)
                    .attach_printable(format!("Converting array element #{}", i)))
                .collect::<Result<Vec<_>, _>>()
                .attach_printable(context.clone())?;
            Value::array(converted)
        },
        DynSolValue::FixedArray(values) => {
            let converted = values.iter()
                .enumerate()
                .map(|(i, v)| sol_value_to_value(v)
                    .attach_printable(format!("Converting fixed array element #{}", i)))
                .collect::<Result<Vec<_>, _>>()
                .attach_printable(context.clone())?;
            Value::array(converted)
        },
        DynSolValue::Tuple(values) => {
            return Err(Report::new(EvmError::Codec(
                CodecError::UnsupportedAbiType("tuple".to_string())
            )))
            .attach_printable("Tuple conversion not yet implemented");
        },
        DynSolValue::CustomStruct { name, prop_names, tuple } => {
            let converted_values = tuple
                .iter()
                .enumerate()
                .map(|(i, v)| sol_value_to_value(v)
                    .attach_printable(format!("Converting struct field #{}", i)))
                .collect::<Result<Vec<_>, _>>()
                .attach_printable(format!("Converting struct '{}'", name))?;
            
            let obj = ObjectType::from_map(
                converted_values
                    .iter()
                    .zip(prop_names)
                    .map(|(v, k)| (k.clone(), v.clone()))
                    .collect(),
            );
            
            ObjectType::from(vec![(
                &name,
                obj.to_value(),
            )])
            .to_value()
        },
    };
    Ok(value)
}

// Helper function to get a descriptive name for DynSolValue types
fn sol_value_type_name(value: &DynSolValue) -> String {
    match value {
        DynSolValue::Bool(_) => "bool".to_string(),
        DynSolValue::Int(_, bits) => format!("int{}", bits),
        DynSolValue::Uint(_, bits) => format!("uint{}", bits),
        DynSolValue::FixedBytes(_, size) => format!("bytes{}", size),
        DynSolValue::Address(_) => "address".to_string(),
        DynSolValue::Function(_) => "function".to_string(),
        DynSolValue::Bytes(_) => "bytes".to_string(),
        DynSolValue::String(_) => "string".to_string(),
        DynSolValue::Array(_) => "array".to_string(),
        DynSolValue::FixedArray(_) => "fixed_array".to_string(),
        DynSolValue::Tuple(_) => "tuple".to_string(),
        DynSolValue::CustomStruct { name, .. } => format!("struct {}", name),
    }
}