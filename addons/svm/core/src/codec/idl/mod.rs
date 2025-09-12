pub mod convert_idl;

use std::str::FromStr;

use crate::typing::anchor as anchor_lang_idl;
use crate::typing::SvmValue;
use anchor_lang_idl::types::{
    Idl, IdlArrayLen, IdlDefinedFields, IdlGenericArg, IdlInstruction, IdlType, IdlTypeDef,
    IdlTypeDefGeneric, IdlTypeDefTy,
};
use convert_idl::classic_idl_to_anchor_idl;
use solana_sdk::pubkey::Pubkey;
use std::fmt::Display;
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::{helpers::fs::FileLocation, indexmap::IndexMap, types::types::Value};
use txtx_addon_network_svm_types::I256;
use txtx_addon_network_svm_types::U256;

#[derive(Debug, Clone)]
pub struct IdlRef {
    pub idl: Idl,
    pub location: Option<FileLocation>,
}

impl IdlRef {
    pub fn from_location(location: FileLocation) -> Result<Self, Diagnostic> {
        let idl_str = location
            .read_content_as_utf8()
            .map_err(|e| diagnosed_error!("unable to read idl: {e}"))?;
        let idl = parse_idl_string(&idl_str)?;
        Ok(Self { idl, location: Some(location) })
    }

    pub fn from_idl(idl: Idl) -> Self {
        Self { idl, location: None }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Diagnostic> {
        let idl = parse_idl_bytes(bytes)?;
        Ok(Self { idl, location: None })
    }

    pub fn from_str(idl_str: &str) -> Result<Self, Diagnostic> {
        let idl = parse_idl_string(idl_str)?;
        Ok(Self { idl, location: None })
    }

    pub fn get_program_pubkey(&self) -> Result<Pubkey, Diagnostic> {
        Pubkey::from_str(&self.idl.address)
            .map_err(|e| diagnosed_error!("invalid pubkey in program IDL: {e}"))
    }

    pub fn get_discriminator(&self, instruction_name: &str) -> Result<Vec<u8>, Diagnostic> {
        self.get_instruction(instruction_name).map(|i| i.discriminator.clone())
    }

    pub fn get_instruction(&self, instruction_name: &str) -> Result<&IdlInstruction, Diagnostic> {
        self.idl
            .instructions
            .iter()
            .find(|i| i.name == instruction_name)
            .ok_or_else(|| diagnosed_error!("instruction '{instruction_name}' not found in IDL"))
    }

    pub fn get_types(&self) -> Vec<IdlTypeDef> {
        self.idl.types.clone()
    }

    /// Encodes the arguments for a given instruction into a map of argument names to byte arrays.
    pub fn get_encoded_args_map(
        &self,
        instruction_name: &str,
        args: Vec<Value>,
    ) -> Result<IndexMap<String, Vec<u8>>, Diagnostic> {
        let instruction = self.get_instruction(instruction_name)?;
        if args.len() != instruction.args.len() {
            return Err(diagnosed_error!(
                "{} arguments provided for instruction {}, which expects {} arguments",
                args.len(),
                instruction_name,
                instruction.args.len()
            ));
        }
        if args.is_empty() {
            return Ok(IndexMap::new());
        }

        let idl_types = self.get_types();

        let mut encoded_args = IndexMap::new();
        for (user_arg_idx, arg) in args.iter().enumerate() {
            let idl_arg = instruction.args.get(user_arg_idx).unwrap();
            let encoded_arg = borsh_encode_value_to_idl_type(arg, &idl_arg.ty, &idl_types, None)
                .map_err(|e| {
                    diagnosed_error!("error in argument at position {}: {}", user_arg_idx + 1, e)
                })?;
            encoded_args.insert(idl_arg.name.clone(), encoded_arg);
        }
        Ok(encoded_args)
    }

    /// Encodes the arguments for a given instruction into a flat byte array.
    pub fn get_encoded_args(
        &self,
        instruction_name: &str,
        args: Vec<Value>,
    ) -> Result<Vec<u8>, Diagnostic> {
        let instruction = self.get_instruction(instruction_name)?;
        if args.len() != instruction.args.len() {
            return Err(diagnosed_error!(
                "{} arguments provided for instruction {}, which expects {} arguments",
                args.len(),
                instruction_name,
                instruction.args.len()
            ));
        }
        if args.is_empty() {
            return Ok(vec![]);
        }

        let idl_types = self.get_types();

        let mut encoded_args = vec![];
        for (user_arg_idx, arg) in args.iter().enumerate() {
            let idl_arg = instruction.args.get(user_arg_idx).unwrap();
            let mut encoded_arg = borsh_encode_value_to_idl_type(
                arg,
                &idl_arg.ty,
                &idl_types,
                None,
            )
            .map_err(|e| {
                diagnosed_error!("error in argument at position {}: {}", user_arg_idx + 1, e)
            })?;
            encoded_args.append(&mut encoded_arg);
        }
        Ok(encoded_args)
    }
}

fn parse_idl_string(idl_str: &str) -> Result<Idl, Diagnostic> {
    let idl = match serde_json::from_str(&idl_str) {
        Ok(anchor_idl) => anchor_idl,
        Err(e) => match serde_json::from_str(&idl_str) {
            Ok(classic_idl) => classic_idl_to_anchor_idl(classic_idl)?,
            Err(_) => {
                return Err(diagnosed_error!("invalid idl: {e}"));
            }
        },
    };
    Ok(idl)
}

fn parse_idl_bytes(idl_bytes: &[u8]) -> Result<Idl, Diagnostic> {
    let idl = match serde_json::from_slice(&idl_bytes) {
        Ok(anchor_idl) => anchor_idl,
        Err(e) => match serde_json::from_slice(&idl_bytes) {
            Ok(classic_idl) => classic_idl_to_anchor_idl(classic_idl)?,
            Err(_) => {
                return Err(diagnosed_error!("invalid idl: {e}"));
            }
        },
    };
    Ok(idl)
}

fn borsh_encode_value_to_idl_type(
    value: &Value,
    idl_type: &IdlType,
    idl_types: &Vec<IdlTypeDef>,
    defined_parent_context: Option<&IdlType>,
) -> Result<Vec<u8>, String> {
    let mismatch_err = |expected: &str| {
        format!(
            "invalid value for idl type: expected {}, found {}",
            expected,
            value.get_type().to_string()
        )
    };
    let encode_err = |expected: &str, e: &dyn Display| {
        format!("unable to encode value ({}) as borsh {}: {}", value.to_string(), expected, e)
    };

    match value {
        Value::Buffer(bytes) => return borsh_encode_bytes_to_idl_type(bytes, idl_type, idl_types),
        Value::Addon(addon_data) => {
            return borsh_encode_bytes_to_idl_type(&addon_data.bytes, idl_type, idl_types)
        }
        _ => {}
    }

    match idl_type {
        IdlType::Bool => value
            .as_bool()
            .and_then(|b| Some(borsh::to_vec(&b).map_err(|e| encode_err("bool", &e))))
            .transpose()?
            .ok_or(mismatch_err("bool")),
        IdlType::U8 => SvmValue::to_number::<u8>(value)
            .and_then(|num| borsh::to_vec(&num).map_err(|e| e.to_string()))
            .map_err(|e| encode_err("u8", &e)),
        IdlType::U16 => SvmValue::to_number::<u16>(value)
            .and_then(|num| borsh::to_vec(&num).map_err(|e| e.to_string()))
            .map_err(|e| encode_err("u16", &e)),
        IdlType::U32 => SvmValue::to_number::<u32>(value)
            .and_then(|num| borsh::to_vec(&num).map_err(|e| e.to_string()))
            .map_err(|e| encode_err("u32", &e)),
        IdlType::U64 => SvmValue::to_number::<u64>(value)
            .and_then(|num| borsh::to_vec(&num).map_err(|e| e.to_string()))
            .map_err(|e| encode_err("u64", &e)),
        IdlType::U128 => SvmValue::to_number::<u128>(value)
            .and_then(|num| borsh::to_vec(&num).map_err(|e| e.to_string()))
            .map_err(|e| encode_err("u128", &e)),
        IdlType::U256 => SvmValue::to_number::<U256>(value)
            .and_then(|num| borsh::to_vec(&num.0).map_err(|e| e.to_string()))
            .map_err(|e| encode_err("u256", &e)),
        IdlType::I8 => SvmValue::to_number::<i8>(value)
            .and_then(|num| borsh::to_vec(&num).map_err(|e| e.to_string()))
            .map_err(|e| encode_err("i8", &e)),
        IdlType::I16 => SvmValue::to_number::<i16>(value)
            .and_then(|num| borsh::to_vec(&num).map_err(|e| e.to_string()))
            .map_err(|e| encode_err("i16", &e)),
        IdlType::I32 => SvmValue::to_number::<i32>(value)
            .and_then(|num| borsh::to_vec(&num).map_err(|e| e.to_string()))
            .map_err(|e| encode_err("i32", &e)),
        IdlType::I64 => SvmValue::to_number::<i64>(value)
            .and_then(|num| borsh::to_vec(&num).map_err(|e| e.to_string()))
            .map_err(|e| encode_err("i64", &e)),
        IdlType::I128 => SvmValue::to_number::<i128>(value)
            .and_then(|num| borsh::to_vec(&num).map_err(|e| e.to_string()))
            .map_err(|e| encode_err("i128", &e)),
        IdlType::I256 => SvmValue::to_number::<I256>(value)
            .and_then(|num| borsh::to_vec(&num.0).map_err(|e| e.to_string()))
            .map_err(|e| encode_err("i256", &e)),
        IdlType::F32 => SvmValue::to_number::<f32>(value)
            .and_then(|num| borsh::to_vec(&num).map_err(|e| e.to_string()))
            .map_err(|e| encode_err("f32", &e)),
        IdlType::F64 => SvmValue::to_number::<f64>(value)
            .and_then(|num| borsh::to_vec(&num).map_err(|e| e.to_string()))
            .map_err(|e| encode_err("f64", &e)),
        IdlType::Bytes => Ok(value.to_be_bytes().clone()),
        IdlType::String => value
            .as_string()
            .and_then(|s| Some(borsh::to_vec(&s).map_err(|e| encode_err("string", &e))))
            .transpose()?
            .ok_or(mismatch_err("string")),
        IdlType::Pubkey => SvmValue::to_pubkey(value)
            .map_err(|_| mismatch_err("pubkey"))
            .map(|p| borsh::to_vec(&p))?
            .map_err(|e| encode_err("pubkey", &e)),
        IdlType::Option(idl_type) => {
            if let Some(_) = value.as_null() {
                borsh::to_vec(&None::<u8>).map_err(|e| encode_err("Optional", &e))
            } else {
                let encoded_arg = borsh_encode_value_to_idl_type(value, idl_type, idl_types, None)?;
                borsh::to_vec(&Some(encoded_arg)).map_err(|e| encode_err("Optional", &e))
            }
        }
        IdlType::Vec(idl_type) => match value {
            Value::String(_) => {
                let bytes = value.get_buffer_bytes_result().map_err(|_| mismatch_err("vec"))?;
                borsh_encode_bytes_to_idl_type(&bytes, idl_type, idl_types)
            }
            Value::Array(vec) => vec
                .iter()
                .map(|v| borsh_encode_value_to_idl_type(v, idl_type, idl_types, None))
                .collect::<Result<Vec<_>, _>>()
                .map(|v| v.into_iter().flatten().collect::<Vec<_>>()),
            _ => Err(mismatch_err("vec")),
        },
        IdlType::Array(idl_type, idl_array_len) => {
            let expected_length = match idl_array_len {
                IdlArrayLen::Generic(generic_len) => {
                    let Some(&IdlType::Defined {
                        name: defined_parent_name,
                        generics: defined_parent_generics,
                    }) = defined_parent_context.as_ref()
                    else {
                        return Err(format!(
                            "generic array length does not contain parent type name"
                        ));
                    };

                    let type_def_generics = &idl_types
                        .iter()
                        .find(|t| t.name.eq(defined_parent_name))
                        .ok_or_else(|| {
                            format!(
                                "unable to find type definition for {} in idl",
                                defined_parent_name
                            )
                        })?
                        .generics;

                    let IdlType::Defined { name, .. } = parse_generic_expected_type(
                        &IdlType::Generic(generic_len.to_string()),
                        &type_def_generics,
                        &defined_parent_generics,
                    )?
                    else {
                        return Err(format!("unable to parse generic array length"));
                    };
                    &name
                        .parse::<usize>()
                        .map_err(|e| format!("unable to parse generic array length: {}", e))?
                }
                IdlArrayLen::Value(len) => len,
            };
            let array = value
                .as_array()
                .map(|a| {
                    if expected_length != &a.len() {
                        return Err(format!(
                            "invalid value for idl type: expected array length of {}, found {}",
                            expected_length,
                            a.len()
                        ));
                    }
                    a.iter()
                        .map(|v| borsh_encode_value_to_idl_type(v, idl_type, idl_types, None))
                        .collect::<Result<Vec<_>, _>>()
                })
                .transpose()?
                .map(|v| v.into_iter().flatten().collect::<Vec<_>>())
                .ok_or(mismatch_err("vec"));
            array
        }
        IdlType::Defined { name, generics } => {
            let typing = idl_types
                .iter()
                .find(|t| &t.name == name)
                .ok_or_else(|| format!("unable to find type definition for {} in idl", name))?;
            let fields = match &typing.ty {
                IdlTypeDefTy::Struct { fields } => {
                    if let Some(idl_defined_fields) = fields {
                        borsh_encode_value_to_idl_defined_fields(
                            idl_defined_fields,
                            value,
                            idl_type,
                            idl_types,
                            generics,
                            &typing.generics,
                        )
                        .map_err(|e| format!("unable to encode value as borsh struct: {}", e))?
                    } else {
                        vec![]
                    }
                }
                IdlTypeDefTy::Enum { variants } => {
                    let enum_value = value.as_object().ok_or(mismatch_err("object"))?;
                    let enum_variant = enum_value.get("variant").ok_or_else(|| {
                        format!(
                            "unable to encode value ({}) as borsh enum: missing variant field",
                            value.to_string(),
                        )
                    })?.as_string().ok_or_else(|| {
                        format!(
                            "unable to encode value ({}) as borsh enum: expected variant field to be a string",
                            value.to_string(),
                        )
                    })?;

                    let (variant_index, expected_variant) = variants
                        .iter()
                        .enumerate()
                        .find(|(_, v)| v.name.eq(enum_variant))
                        .ok_or_else(|| {
                            format!(
                                "unable to encode value ({}) as borsh enum: unknown variant {}",
                                value.to_string(),
                                enum_variant
                            )
                        })?;

                    let mut encoded = vec![variant_index as u8];

                    let type_def_generics = idl_types
                        .iter()
                        .find(|t| &t.name == name)
                        .map(|t| t.generics.clone())
                        .unwrap_or_default();

                    match &expected_variant.fields {
                        Some(idl_defined_fields) => {
                            let enum_variant_value = enum_value.get("value").ok_or_else(|| {
                                format!(
                                    "unable to encode value ({}) as borsh enum: missing 'value' field",
                                    value.to_string(),
                                )
                            })?;
                            let mut encoded_fields = borsh_encode_value_to_idl_defined_fields(
                                &idl_defined_fields,
                                enum_variant_value,
                                idl_type,
                                idl_types,
                                &vec![],
                                &type_def_generics,
                            )
                            .map_err(|e| {
                                format!("unable to encode value as borsh struct: {}", e)
                            })?;

                            encoded.append(&mut encoded_fields);
                            encoded
                        }
                        None => encoded,
                    }
                }
                IdlTypeDefTy::Type { alias } => {
                    borsh_encode_value_to_idl_type(value, &alias, idl_types, Some(idl_type))?
                }
            };
            Ok(fields)
        }
        IdlType::Generic(generic) => {
            let idl_generic = idl_types.iter().find_map(|t| {
                t.generics.iter().find_map(|g| {
                    let is_match = match g {
                        IdlTypeDefGeneric::Type { name } => name == generic,
                        IdlTypeDefGeneric::Const { name, .. } => name == generic,
                    };
                    if is_match {
                        Some(g)
                    } else {
                        None
                    }
                })
            });
            let Some(idl_generic) = idl_generic else {
                return Err(format!("unable to find generic {} in idl", generic));
            };
            match idl_generic {
                IdlTypeDefGeneric::Type { name } => {
                    let ty = IdlType::from_str(name)
                        .map_err(|e| format!("invalid generic type: {e}"))?;
                    borsh_encode_value_to_idl_type(value, &ty, idl_types, None)
                }
                IdlTypeDefGeneric::Const { ty, .. } => {
                    let ty =
                        IdlType::from_str(ty).map_err(|e| format!("invalid generic type: {e}"))?;
                    borsh_encode_value_to_idl_type(value, &ty, idl_types, None)
                }
            }
        }
        t => return Err(format!("IDL type {:?} is not yet supported", t)),
    }
}

fn borsh_encode_value_to_idl_defined_fields(
    idl_defined_fields: &IdlDefinedFields,
    value: &Value,
    idl_type: &IdlType,
    idl_types: &Vec<IdlTypeDef>,
    generics: &Vec<IdlGenericArg>,
    type_def_generics: &Vec<IdlTypeDefGeneric>,
) -> Result<Vec<u8>, String> {
    let mismatch_err = |expected: &str| {
        format!(
            "invalid value for idl type: expected {}, found {}",
            expected,
            value.get_type().to_string()
        )
    };
    let encode_err = |expected: &str, e| {
        format!("unable to encode value ({}) as borsh {}: {}", value.to_string(), expected, e)
    };
    let mut encoded_fields = vec![];
    match idl_defined_fields {
        IdlDefinedFields::Named(expected_fields) => {
            let mut user_values_map = value.as_object().ok_or(mismatch_err("object"))?.clone();
            for field in expected_fields {
                let user_value = user_values_map
                    .swap_remove(&field.name)
                    .ok_or_else(|| format!("missing field '{}' in object", field.name))?;

                let ty = parse_generic_expected_type(&field.ty, &type_def_generics, generics)?;

                let mut encoded_field =
                    borsh_encode_value_to_idl_type(&user_value, &ty, idl_types, Some(idl_type))
                        .map_err(|e| format!("failed to encode field '{}': {}", field.name, e))?;
                encoded_fields.append(&mut encoded_field);
            }
            if !user_values_map.is_empty() {
                return Err(format!(
                    "extra fields found in object: {}",
                    user_values_map.keys().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                ));
            }
        }
        IdlDefinedFields::Tuple(expected_tuple_types) => {
            let user_values = value.as_array().ok_or(mismatch_err("array"))?;
            let mut encoded_tuple_fields = vec![];

            if user_values.len() != expected_tuple_types.len() {
                return Err(format!(
                    "invalid value for idl type: expected tuple length of {}, found {}",
                    expected_tuple_types.len(),
                    user_values.len()
                ));
            }
            for (i, expected_type) in expected_tuple_types.iter().enumerate() {
                let user_value = user_values
                    .get(i)
                    .ok_or_else(|| format!("missing field value in {} index of array", i))?;

                let ty = parse_generic_expected_type(expected_type, &type_def_generics, generics)?;

                let encoded_field =
                    borsh_encode_value_to_idl_type(user_value, &ty, idl_types, Some(idl_type))
                        .map_err(|e| format!("failed to encode field #{}: {}", i + 1, e))?;
                encoded_tuple_fields.push(encoded_field);
            }
            encoded_fields.append(
                &mut borsh::to_vec(&encoded_tuple_fields).map_err(|e| encode_err("tuple", e))?,
            );
        }
    }
    Ok(encoded_fields)
}

// todo
fn borsh_encode_bytes_to_idl_type(
    bytes: &Vec<u8>,
    idl_type: &IdlType,
    idl_types: &Vec<IdlTypeDef>,
) -> Result<Vec<u8>, String> {
    match idl_type {
        IdlType::U8 => bytes
            .iter()
            .map(|b| {
                borsh_encode_value_to_idl_type(
                    &Value::integer(*b as i128),
                    idl_type,
                    idl_types,
                    None,
                )
            })
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.into_iter().flatten().collect::<Vec<_>>()),
        _ => todo!(),
    }
}

fn parse_generic_expected_type(
    expected_type: &IdlType,
    type_def_generics: &Vec<IdlTypeDefGeneric>,
    generic_args: &Vec<IdlGenericArg>,
) -> Result<IdlType, String> {
    let ty = if let IdlType::Generic(generic) = &expected_type {
        let Some(generic_pos) = type_def_generics.iter().position(|g| match g {
            IdlTypeDefGeneric::Type { name } => name.eq(generic),
            IdlTypeDefGeneric::Const { name, .. } => name.eq(generic),
        }) else {
            return Err(format!("unable to find generic {} in idl", generic));
        };
        let generic = generic_args
            .get(generic_pos)
            .ok_or(format!("unable to find generic {} in idl", generic))?;
        match generic {
            IdlGenericArg::Type { ty } => ty,
            IdlGenericArg::Const { value } => {
                &IdlType::from_str(value).map_err(|e| format!("invalid generic type: {e}"))?
            }
        }
    } else {
        &expected_type
    };
    Ok(ty.clone())
}
