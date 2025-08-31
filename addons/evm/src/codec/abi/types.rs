use alloy::dyn_abi::{DynSolValue, Word};
use alloy::primitives::U256;
use error_stack::{Report, ResultExt};
use txtx_addon_kit::types::types::Value;

use crate::errors::{EvmError, EvmResult, CodecError};
use crate::typing::{
    EvmValue, EVM_UINT256, EVM_ADDRESS, EVM_BYTES, EVM_BYTES32,
    EVM_UINT32, EVM_UINT8, EVM_FUNCTION_CALL, EVM_INIT_CODE,
    EVM_KNOWN_SOL_PARAM,
};

pub fn value_to_sol_value(value: &Value) -> EvmResult<DynSolValue> {
    let context = format!("Converting {} to Solidity value", value.get_type().to_string());
    
    let sol_value = match value {
        Value::Bool(value) => DynSolValue::Bool(value.clone()),
        Value::Integer(value) => DynSolValue::Uint(U256::from(*value), 256),
        Value::String(value) => DynSolValue::String(value.clone()),
        Value::Float(_value) => {
            return Err(Report::new(EvmError::Codec(
                CodecError::UnsupportedAbiType("float".to_string())
            )))
            .attach_printable("Float values are not supported in Solidity");
        },
        Value::Buffer(bytes) => DynSolValue::Bytes(bytes.clone()),
        Value::Null => {
            return Err(Report::new(EvmError::Codec(
                CodecError::InvalidType {
                    expected: "non-null value".to_string(),
                    received: "null".to_string()
                }
            )))
            .attach_printable("Null values cannot be converted to Solidity");
        },
        Value::Object(_object) => {
            return Err(Report::new(EvmError::Codec(
                CodecError::UnsupportedAbiType("object".to_string())
            )))
            .attach_printable("Object conversion to Solidity not yet implemented");
        },
        Value::Array(array) => {
            let sol_values = array.iter()
                .enumerate()
                .map(|(i, v)| value_to_sol_value(v)
                    .attach_printable(format!("Converting array element #{}", i)))
                .collect::<Result<Vec<_>, _>>()
                .attach_printable(context.clone())?;
            DynSolValue::Array(sol_values)
        },
        Value::Addon(addon) => {
            if addon.id == EVM_UINT256 {
                let bytes = addon.bytes.clone();
                let padding = if bytes.len() < 32 { 32 - bytes.len() } else { 0 };
                let mut padded = vec![0u8; padding];
                padded.extend(bytes);
                let value = U256::from_be_bytes::<32>(
                    padded.as_slice().try_into()
                        .map_err(|_| Report::new(EvmError::Codec(
                            CodecError::InvalidType {
                                expected: "32 bytes for uint256".to_string(),
                                received: format!("{} bytes", padded.len())
                            }
                        )))
                        .attach_printable("Converting to uint256")?
                );
                DynSolValue::Uint(value, 256)
            } else if addon.id == EVM_ADDRESS {
                let value = EvmValue::to_address(value)
                    .map_err(|e| Report::new(EvmError::Codec(
                        CodecError::InvalidAddress(format!("{:?}", e))
                    )))
                    .attach_printable("Converting to address")?;
                DynSolValue::Address(value)
            } else if addon.id == EVM_BYTES {
                DynSolValue::Bytes(addon.bytes.clone())
            } else if addon.id == EVM_BYTES32 {
                let mut bytes32 = [0u8; 32];
                let copy_len = addon.bytes.len().min(32);
                bytes32[..copy_len].copy_from_slice(&addon.bytes[..copy_len]);
                DynSolValue::FixedBytes(Word::from(bytes32), 32)
            } else if addon.id == EVM_UINT32 {
                let bytes = addon.bytes.clone();
                if bytes.len() < 4 {
                    let mut padded = vec![0u8; 4 - bytes.len()];
                    padded.extend(bytes);
                    let value = u32::from_be_bytes(
                        padded.as_slice().try_into()
                            .map_err(|_| Report::new(EvmError::Codec(
                                CodecError::InvalidType {
                                    expected: "4 bytes for uint32".to_string(),
                                    received: format!("{} bytes", padded.len())
                                }
                            )))?
                    );
                    DynSolValue::Uint(U256::from(value), 32)
                } else {
                    let value = u32::from_be_bytes(
                        bytes[0..4].try_into()
                            .map_err(|_| Report::new(EvmError::Codec(
                                CodecError::InvalidType {
                                    expected: "4 bytes for uint32".to_string(),
                                    received: format!("{} bytes", bytes.len())
                                }
                            )))?
                    );
                    DynSolValue::Uint(U256::from(value), 32)
                }
            } else if addon.id == EVM_UINT8 {
                let value = if addon.bytes.is_empty() { 0 } else { addon.bytes[0] };
                DynSolValue::Uint(U256::from(value), 8)
            } else if addon.id == EVM_FUNCTION_CALL {
                // TODO: Properly parse function call data structure
                DynSolValue::Bytes(addon.bytes.clone())
            } else if addon.id == EVM_INIT_CODE {
                // TODO: Properly parse init code data structure  
                DynSolValue::Bytes(addon.bytes.clone())
            } else if addon.id == EVM_KNOWN_SOL_PARAM {
                let (value, _param) = EvmValue::to_known_sol_param(value)
                    .map_err(|e| Report::new(EvmError::Codec(
                        CodecError::InvalidValue {
                            value_type: "EVM_KNOWN_SOL_PARAM".to_string(),
                            target_type: "sol_value".to_string()
                        }
                    )))
                    .attach_printable("Extracting known Solidity parameter")?;
                value_to_sol_value(&value)
                    .attach_printable("Converting known parameter to Solidity value")?
            } else {
                return Err(Report::new(EvmError::Codec(
                    CodecError::UnsupportedAbiType(format!("addon type {}", addon.id))
                )))
                .attach_printable(format!(
                    "Converting Value type {} to DynSolValue",
                    value.get_type().to_string()
                ));
            }
        }
    };
    Ok(sol_value)
}
// ============================================================================
// Backward compatibility wrapper functions
// ============================================================================

/// Backward compatibility wrapper for value_to_sol_value
pub fn value_to_sol_value_compat(value: &Value) -> Result<DynSolValue, String> {
    value_to_sol_value(value)
        .map_err(|e| format!("{}", e))
}
