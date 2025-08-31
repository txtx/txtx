use std::collections::VecDeque;
use std::num::NonZeroUsize;

use alloy::dyn_abi::{DynSolValue, FunctionExt, Word};
use alloy::dyn_abi::parser::TypeSpecifier;
use alloy::json_abi::{Constructor, JsonAbi, Param};
use alloy::primitives::U256;
use alloy::hex;
use error_stack::{Report, ResultExt};
use txtx_addon_kit::types::types::Value;

use crate::errors::{EvmError, EvmResult, CodecError};
use crate::typing::{
    EvmValue, EVM_SIM_RESULT, EVM_KNOWN_SOL_PARAM,
};

// For backward compatibility

pub fn value_to_abi_function_args(
    function_name: &str,
    value: &Value,
    abi: &JsonAbi,
) -> EvmResult<Vec<DynSolValue>> {
    // Try to find the function
    let functions = abi.function(function_name);
    
    if functions.is_none() {
        // Function not found - provide helpful context
        let available_functions: Vec<String> = abi.functions.keys().cloned().collect();
        let mut error = Report::new(EvmError::Codec(
            CodecError::FunctionNotFound { name: function_name.to_string() }
        ));
        
        error = error.attach_printable(format!("Function '{}' not found in ABI", function_name));
        
        if !available_functions.is_empty() {
            error = error.attach_printable(format!("Available functions: {}", available_functions.join(", ")));
            
            // Check for similar names (case mismatch, typos)
            for available in &available_functions {
                if available.to_lowercase() == function_name.to_lowercase() {
                    error = error.attach_printable(format!(
                        "Did you mean '{}'? (case-sensitive)", available
                    ));
                }
            }
        }
        
        return Err(error);
    }
    
    let function = functions.unwrap().first()
        .ok_or_else(|| Report::new(EvmError::Codec(
            CodecError::FunctionNotFound { name: function_name.to_string() }
        )))?;

    let values = value.as_array()
        .ok_or_else(|| Report::new(EvmError::Codec(
            CodecError::InvalidType { 
                expected: "array".to_string(),
                received: value.get_type().to_string()
            }
        )))
        .attach_printable("Function arguments must be an array")?;

    if values.len() != function.inputs.len() {
        let mut error = Report::new(EvmError::Codec(
            CodecError::ArgumentCountMismatch {
                expected: function.inputs.len(),
                got: values.len()
            }
        ));
        
        error = error.attach_printable(format!(
            "Function '{}' expects {} arguments, got {}", 
            function_name, 
            function.inputs.len(), 
            values.len()
        ));
        
        // Show expected vs provided arguments
        error = error.attach_printable("\nExpected arguments:");
        for (i, param) in function.inputs.iter().enumerate() {
            let status = if i < values.len() { "✓" } else { "✗ missing" };
            error = error.attach_printable(format!(
                "  [{}] {}: {} {}", 
                i, 
                if param.name.is_empty() { "arg" } else { &param.name },
                param.ty,
                status
            ));
        }
        
        if values.len() > function.inputs.len() {
            error = error.attach_printable(format!(
                "\n  Extra arguments provided: {} additional",
                values.len() - function.inputs.len()
            ));
        }
        
        return Err(error);
    }
    
    value_to_abi_params(values, &function.inputs)
        .attach_printable(format!("Encoding arguments for function '{}'", function_name))
}

pub fn value_to_abi_constructor_args(
    value: &Value,
    abi_constructor: &Constructor,
) -> EvmResult<Vec<DynSolValue>> {
    let values = value.as_array()
        .ok_or_else(|| Report::new(EvmError::Codec(
            CodecError::InvalidType {
                expected: "array".to_string(),
                received: value.get_type().to_string()
            }
        )))
        .attach_printable("Constructor arguments must be an array")?;

    if values.len() != abi_constructor.inputs.len() {
        return Err(Report::new(EvmError::Codec(
            CodecError::ArgumentCountMismatch {
                expected: abi_constructor.inputs.len(),
                got: values.len()
            }
        )))
        .attach_printable(format!("Constructor expects {} arguments", abi_constructor.inputs.len()));
    }

    value_to_abi_params(values, &abi_constructor.inputs)
        .attach_printable("Encoding constructor arguments")
}

pub fn value_to_abi_params(
    values: &Vec<Value>,
    params: &Vec<Param>,
) -> EvmResult<Vec<DynSolValue>> {
    let mut sol_values = vec![];
    for (i, param) in params.iter().enumerate() {
        let value = values.get(i)
            .ok_or_else(|| Report::new(EvmError::Codec(
                CodecError::ArgumentCountMismatch {
                    expected: params.len(),
                    got: i
                }
            )))?;
        let sol_value = value_to_abi_param(value, param)
            .attach_printable(format!("Encoding parameter #{} ({})", i + 1, param.name))?;
        sol_values.push(sol_value);
    }
    Ok(sol_values)
}

pub fn value_to_abi_param(value: &Value, param: &Param) -> EvmResult<DynSolValue> {
    if let Some(addon_data) = value.as_addon_data() {
        if addon_data.id == EVM_SIM_RESULT {
            let (result, fn_spec) = EvmValue::to_sim_result(value)
                .map_err(|e| Report::new(EvmError::Codec(
                    CodecError::AbiDecodingFailed(format!("Failed to extract simulation result: {:?}", e))
                )))?;
            if let Some(fn_spec) = fn_spec {
                let res = fn_spec.abi_decode_output(&result)
                    .map_err(|e| Report::new(EvmError::Codec(
                        CodecError::AbiDecodingFailed(format!("Failed to decode function output: {}", e))
                    )))
                    .attach_printable("Decoding simulation result")?;
                if res.len() == 1 {
                    return Ok(res.get(0).unwrap().clone());
                } else {
                    return Ok(DynSolValue::Tuple(res));
                }
            }
        } else if addon_data.id == EVM_KNOWN_SOL_PARAM {
            let (value, param) = EvmValue::to_known_sol_param(value)
                .map_err(|e| Report::new(EvmError::Codec(
                    CodecError::InvalidValue {
                        value_type: "EVM_KNOWN_SOL_PARAM".to_string(),
                        target_type: param.ty.clone()
                    }
                )))?;
            return value_to_abi_param(&value, &param)
                .attach_printable("Encoding known Solidity parameter");
        }
    }

    let type_specifier = TypeSpecifier::try_from(param.ty.as_str())
        .map_err(|e| Report::new(EvmError::Codec(
            CodecError::TypeSpecifierParseFailed(format!("{}: {}", param.ty, e))
        )))
        .attach_printable(format!(
            "Converting {} to ABI type {}",
            value.get_type().to_string(),
            param.ty.as_str()
        ))?;
    
    let is_array = type_specifier.sizes.len() > 0;

    if is_array {
        let values = value.as_array()
            .ok_or_else(|| Report::new(EvmError::Codec(
                CodecError::InvalidType {
                    expected: "array".to_string(),
                    received: value.get_type().to_string()
                }
            )))
            .attach_printable(format!(
                "Converting {} to ABI type {}",
                value.get_type().to_string(),
                param.ty.as_str()
            ))?;
        value_to_array_abi_type(values, &mut VecDeque::from(type_specifier.sizes), &param)
            .attach_printable(format!(
                "Converting {} to ABI type {}",
                value.get_type().to_string(),
                param.ty.as_str()
            ))
    } else {
        value_to_primitive_abi_type(value, &param)
            .attach_printable(format!(
                "Converting {} to ABI type {}",
                value.get_type().to_string(),
                param.ty.as_str()
            ))
    }
}

pub fn value_to_primitive_abi_type(
    value: &Value,
    param: &Param,
) -> EvmResult<DynSolValue> {
    let type_specifier = TypeSpecifier::try_from(param.ty.as_str())
        .map_err(|e| Report::new(EvmError::Codec(
            CodecError::TypeSpecifierParseFailed(format!("{}: {}", param.ty, e))
        )))
        .attach_printable(format!(
            "Converting {} to primitive ABI type {}",
            value.get_type().to_string(),
            param.ty.as_str()
        ))?;

    let sol_value = match type_specifier.stem.span() {
        "address" => {
            let addr = EvmValue::to_address(value)
                .map_err(|e| {
                    let mut error = Report::new(EvmError::Codec(
                        CodecError::InvalidAddress(format!("{:?}", e))
                    ));
                    // Add the original value that failed
                    if let Some(s) = value.as_string() {
                        error = error.attach_printable(format!("Invalid address value: '{}'", s));
                    } else {
                        error = error.attach_printable(format!("Invalid address value: {:?}", value));
                    }
                    error
                })
                .attach_printable(format!(
                    "Converting {} to primitive ABI type {}",
                    value.get_type().to_string(),
                    param.ty.as_str()
                ))?;
            DynSolValue::Address(addr)
        },
        "uint8" => {
            let bytes = value.to_bytes();
            let uint = U256::try_from_be_slice(&bytes)
                .ok_or_else(|| Report::new(EvmError::Codec(
                    CodecError::InvalidType {
                        expected: "uint8".to_string(),
                        received: format!("{} bytes", bytes.len())
                    }
                )))
                .attach_printable(format!(
                    "Converting {} to primitive ABI type {}",
                    value.get_type().to_string(),
                    param.ty.as_str()
                ))?;
            
            // Check if value fits in uint8
            if uint > U256::from(255u32) {
                return Err(Report::new(EvmError::Codec(
                    CodecError::InvalidValue {
                        value_type: format!("uint256({})", uint),
                        target_type: "uint8".to_string()
                    }
                )))
                .attach_printable(format!("Value {} exceeds maximum for uint8 (255)", uint))
                .attach_printable("uint8 range: 0 to 255");
            }
            
            DynSolValue::Uint(uint, 8)
        },
        "uint16" => {
            let bytes = value.to_bytes();
            let uint = U256::try_from_be_slice(&bytes)
                .ok_or_else(|| Report::new(EvmError::Codec(
                    CodecError::InvalidType {
                        expected: "uint16".to_string(),
                        received: format!("{} bytes", bytes.len())
                    }
                )))
                .attach_printable(format!(
                    "Converting {} to primitive ABI type {}",
                    value.get_type().to_string(),
                    param.ty.as_str()
                ))?;
            DynSolValue::Uint(uint, 16)
        },
        "uint32" => {
            let bytes = value.to_bytes();
            let uint = U256::try_from_be_slice(&bytes)
                .ok_or_else(|| Report::new(EvmError::Codec(
                    CodecError::InvalidType {
                        expected: "uint32".to_string(),
                        received: format!("{} bytes", bytes.len())
                    }
                )))
                .attach_printable(format!(
                    "Converting {} to primitive ABI type {}",
                    value.get_type().to_string(),
                    param.ty.as_str()
                ))?;
            DynSolValue::Uint(uint, 32)
        },
        "uint64" => {
            let bytes = value.to_bytes();
            let uint = U256::try_from_be_slice(&bytes)
                .ok_or_else(|| Report::new(EvmError::Codec(
                    CodecError::InvalidType {
                        expected: "uint64".to_string(),
                        received: format!("{} bytes", bytes.len())
                    }
                )))
                .attach_printable(format!(
                    "Converting {} to primitive ABI type {}",
                    value.get_type().to_string(),
                    param.ty.as_str()
                ))?;
            DynSolValue::Uint(uint, 64)
        },
        "uint96" => {
            let bytes = value.to_bytes();
            let uint = U256::try_from_be_slice(&bytes)
                .ok_or_else(|| Report::new(EvmError::Codec(
                    CodecError::InvalidType {
                        expected: "uint96".to_string(),
                        received: format!("{} bytes", bytes.len())
                    }
                )))
                .attach_printable(format!(
                    "Converting {} to primitive ABI type {}",
                    value.get_type().to_string(),
                    param.ty.as_str()
                ))?;
            DynSolValue::Uint(uint, 96)
        },
        "uint256" => {
            let bytes = value.to_bytes();
            let uint = U256::try_from_be_slice(&bytes)
                .ok_or_else(|| Report::new(EvmError::Codec(
                    CodecError::InvalidType {
                        expected: "uint256".to_string(),
                        received: format!("{} bytes", bytes.len())
                    }
                )))
                .attach_printable(format!(
                    "Converting {} to primitive ABI type {}",
                    value.get_type().to_string(),
                    param.ty.as_str()
                ))?;
            DynSolValue::Uint(uint, 256)
        },
        "bytes" => DynSolValue::Bytes(value.to_bytes()),
        "bytes32" => {
            let bytes = value.to_bytes();
            if bytes.len() != 32 {
                let mut error = Report::new(EvmError::Codec(
                    CodecError::InvalidValue {
                        value_type: format!("bytes{}", bytes.len()),
                        target_type: "bytes32".to_string()
                    }
                ));
                
                error = error.attach_printable(format!(
                    "bytes32 requires exactly 32 bytes, got {} bytes", 
                    bytes.len()
                ));
                
                if bytes.len() < 32 {
                    error = error.attach_printable(format!(
                        "Value: 0x{}", 
                        hex::encode(&bytes)
                    ));
                    error = error.attach_printable(
                        "Consider padding with zeros to reach 32 bytes"
                    );
                }
                
                return Err(error);
            }
            DynSolValue::FixedBytes(Word::from_slice(&bytes), 32)
        },
        "bool" => {
            let b = value.as_bool()
                .ok_or_else(|| Report::new(EvmError::Codec(
                    CodecError::InvalidType {
                        expected: "bool".to_string(),
                        received: value.get_type().to_string()
                    }
                )))
                .attach_printable(format!(
                    "Converting {} to primitive ABI type {}",
                    value.get_type().to_string(),
                    param.ty.as_str()
                ))?;
            DynSolValue::Bool(b)
        },
        "string" => DynSolValue::String(value.to_string()),
        "tuple" => {
            let mut tuple = vec![];
            let values = value.as_array()
                .ok_or_else(|| Report::new(EvmError::Codec(
                    CodecError::InvalidType {
                        expected: "array for tuple".to_string(),
                        received: value.get_type().to_string()
                    }
                )))
                .attach_printable(format!(
                    "Converting {} to primitive ABI type {}",
                    value.get_type().to_string(),
                    param.ty.as_str()
                ))?
                .clone();
            for (i, component) in param.components.iter().enumerate() {
                let value = values.get(i)
                    .ok_or_else(|| Report::new(EvmError::Codec(
                        CodecError::ArgumentCountMismatch {
                            expected: param.components.len(),
                            got: values.len()
                        }
                    )))
                    .attach_printable(format!("Tuple component #{}", i + 1))?;
                tuple.push(value_to_abi_param(value, &component)
                    .attach_printable(format!("Encoding tuple component #{} ({})", i + 1, component.name))?);
            }
            DynSolValue::Tuple(tuple)
        },
        "struct" => value_to_struct_abi_type(value, param)
            .attach_printable(format!(
                "Converting {} to primitive ABI type {}",
                value.get_type().to_string(),
                param.ty.as_str()
            ))?,
        _ => return Err(Report::new(EvmError::Codec(
            CodecError::UnsupportedAbiType(param.ty.clone())
        )))
        .attach_printable(format!(
            "Converting {} to primitive ABI type {}",
            value.get_type().to_string(),
            param.ty.as_str()
        )),
    };
    Ok(sol_value)
}

pub fn value_to_array_abi_type(
    values: &Vec<Value>,
    sizes: &mut VecDeque<Option<NonZeroUsize>>,
    param: &Param,
) -> EvmResult<DynSolValue> {
    let Some(size) = sizes.pop_back() else {
        return Err(Report::new(EvmError::Codec(
            CodecError::ArrayDimensionMismatch
        )))
        .attach_printable(format!("Array dimension mismatch for type {}", param.ty));
    };
    
    let mut arr = vec![];
    if let Some(size) = size {
        let size = size.get();
        if values.len() != size {
            return Err(Report::new(EvmError::Codec(
                CodecError::InvalidArrayLength {
                    expected: size,
                    got: values.len()
                }
            )))
            .attach_printable(format!("Fixed array of type {}", param.ty));
        }

        for i in 0..size {
            if sizes.len() > 0 {
                let new_value = values[i].clone();
                let new_values = new_value.as_array()
                    .ok_or_else(|| Report::new(EvmError::Codec(
                        CodecError::InvalidType {
                            expected: "array".to_string(),
                            received: new_value.get_type().to_string()
                        }
                    )))
                    .attach_printable(format!("Array element #{}", i))?;

                arr.push(value_to_array_abi_type(&new_values, sizes, param)
                    .attach_printable(format!("Encoding nested array element #{}", i))?);
            } else {
                arr.push(value_to_primitive_abi_type(&values[i], param)
                    .attach_printable(format!("Encoding array element #{}", i))?);
            }
        }

        Ok(DynSolValue::FixedArray(arr))
    } else {
        for (i, value) in values.iter().enumerate() {
            if sizes.len() > 0 {
                let new_value = value.clone();
                let new_values = new_value.as_array()
                    .ok_or_else(|| Report::new(EvmError::Codec(
                        CodecError::InvalidType {
                            expected: "array".to_string(),
                            received: new_value.get_type().to_string()
                        }
                    )))
                    .attach_printable(format!("Dynamic array element #{}", i))?;
                arr.push(value_to_array_abi_type(&new_values, sizes, param)
                    .attach_printable(format!("Encoding nested dynamic array element #{}", i))?);
            } else {
                arr.push(value_to_primitive_abi_type(value, param)
                    .attach_printable(format!("Encoding dynamic array element #{}", i))?);
            }
        }

        Ok(DynSolValue::Array(arr))
    }
}

pub fn value_to_struct_abi_type(value: &Value, param: &Param) -> EvmResult<DynSolValue> {
    let mut prop_names = vec![];
    let mut tuple = vec![];
    for (i, component) in param.components.iter().enumerate() {
        let component_name = component.name.clone();
        let component_value = value_to_abi_param(value, &component)
            .attach_printable(format!("Encoding struct component '{}' (#{}) of type {}", 
                component_name, i + 1, component.ty))?;
        tuple.push(component_value);
        prop_names.push(component_name);
    }
    Ok(DynSolValue::CustomStruct { 
        name: param.name.clone(), 
        prop_names, 
        tuple 
    })
}




