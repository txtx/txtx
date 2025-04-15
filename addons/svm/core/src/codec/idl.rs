use std::str::FromStr;

use crate::typing::anchor as anchor_lang_idl;
use crate::typing::SvmValue;
use anchor_lang_idl::types::{
    Idl, IdlArrayLen, IdlDefinedFields, IdlGenericArg, IdlInstruction, IdlInstructionAccount,
    IdlInstructionAccounts, IdlType, IdlTypeDef, IdlTypeDefGeneric, IdlTypeDefTy,
};
use solana_idl::Idl as ClassicIdl;
use solana_sdk::pubkey::Pubkey;
use spl_token::solana_program;
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::{
    helpers::fs::FileLocation,
    indexmap::IndexMap,
    types::types::{ObjectType, Value},
};

pub fn parse_idl_string(idl_str: &str) -> Result<Idl, Diagnostic> {
    let idl = match serde_json::from_str::<ClassicIdl>(&idl_str) {
        Ok(classic_idl) => {
            let instructions = classic_idl
                .instructions
                .iter()
                .map(|i| classic_instruction_to_anchor_instruction(&i, &classic_idl.types))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| {
                    diagnosed_error!("failed to convert classic idl to anchor idl: {e}")
                })?;

            let idl_serializer = match &classic_idl.metadata {
                Some(metadata) => {
                    match metadata.serializer.as_ref().and_then(|s| Some::<&str>(s.as_ref())) {
                        Some("bytemuck") => anchor_lang_idl::types::IdlSerialization::Bytemuck,
                        Some("bytemuckunsafe") => {
                            anchor_lang_idl::types::IdlSerialization::BytemuckUnsafe
                        }
                        Some("borsh") | None => anchor_lang_idl::types::IdlSerialization::Borsh,
                        Some(ser) => {
                            anchor_lang_idl::types::IdlSerialization::Custom(ser.to_string())
                        }
                    }
                }
                None => anchor_lang_idl::types::IdlSerialization::Borsh,
            };

            Idl {
                address: classic_idl.metadata.clone().unwrap().address.unwrap().clone(),
                metadata: classic_idl_to_anchor_metadata(&classic_idl),
                docs: vec![],
                instructions,
                accounts: classic_idl
                    .accounts
                    .iter()
                    .map(|a| classic_account_to_anchor_account(a.clone()))
                    .collect(),
                errors: classic_idl
                    .errors
                    .unwrap_or_default()
                    .iter()
                    .map(|e| anchor_lang_idl::types::IdlErrorCode {
                        code: e.code,
                        name: e.name.clone(),
                        msg: e.msg.clone(),
                    })
                    .collect(),
                types: classic_idl_types_to_anchor_types(&classic_idl.types, &idl_serializer)
                    .map_err(|e| {
                        diagnosed_error!(
                            "failed to convert classic idl types to anchor idl types: {e}"
                        )
                    })?,
                constants: classic_idl
                    .constants
                    .iter()
                    .map(|c| classic_const_to_anchor_const(c, &classic_idl.types))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| {
                        diagnosed_error!("failed to convert classic idl to anchor idl: {e}")
                    })?,
                events: vec![], // todo
            }
        }
        Err(_) => {
            serde_json::from_str(&idl_str).map_err(|e| diagnosed_error!("invalid idl: {e}"))?
        }
    };
    Ok(idl)
}

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
            let mut encoded_arg = encode_value_to_idl_type(arg, &idl_arg.ty, &idl_types, None)
                .map_err(|e| {
                    diagnosed_error!("error in argument at position {}: {}", user_arg_idx + 1, e)
                })?;
            encoded_args.append(&mut encoded_arg);
        }
        Ok(encoded_args)
    }
}

pub fn encode_value_to_idl_type(
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
    let encode_err = |expected: &str, e| {
        format!("unable to encode value ({}) as borsh {}: {}", value.to_string(), expected, e)
    };

    match idl_type {
        IdlType::Bool => value
            .as_bool()
            .and_then(|b| Some(borsh::to_vec(&b).map_err(|e| encode_err("bool", e))))
            .transpose()?
            .ok_or(mismatch_err("bool")),
        IdlType::U8 => value
            .as_integer()
            .and_then(|i| Some(borsh::to_vec(&(i as u8)).map_err(|e| encode_err("u8", e))))
            .transpose()?
            .ok_or(mismatch_err("u8")),
        IdlType::I8 => value
            .as_integer()
            .and_then(|i| Some(borsh::to_vec(&(i as i8)).map_err(|e| encode_err("i8", e))))
            .transpose()?
            .ok_or(mismatch_err("i8")),
        IdlType::U16 => value
            .as_integer()
            .and_then(|i| Some(borsh::to_vec(&(i as u16)).map_err(|e| encode_err("u8", e))))
            .transpose()?
            .ok_or(mismatch_err("u16")),
        IdlType::I16 => value
            .as_integer()
            .and_then(|i| Some(borsh::to_vec(&(i as i16)).map_err(|e| encode_err("i16", e))))
            .transpose()?
            .ok_or(mismatch_err("i16")),
        IdlType::U32 => value
            .as_integer()
            .and_then(|i| Some(borsh::to_vec(&(i as u32)).map_err(|e| encode_err("u32", e))))
            .transpose()?
            .ok_or(mismatch_err("u32")),
        IdlType::I32 => value
            .as_integer()
            .and_then(|i| Some(borsh::to_vec(&(i as i32)).map_err(|e| encode_err("i32", e))))
            .transpose()?
            .ok_or(mismatch_err("i32")),
        IdlType::F32 => value
            .as_float()
            .and_then(|i| Some(borsh::to_vec(&(i as f32)).map_err(|e| encode_err("f32", e))))
            .transpose()?
            .ok_or(mismatch_err("f32")),
        IdlType::U64 => value
            .as_integer()
            .and_then(|i| Some(borsh::to_vec(&(i as u64)).map_err(|e| encode_err("u64", e))))
            .transpose()?
            .ok_or(mismatch_err("u64")),
        IdlType::I64 => value
            .as_integer()
            .and_then(|i| Some(borsh::to_vec(&(i as i64)).map_err(|e| encode_err("i64", e))))
            .transpose()?
            .ok_or(mismatch_err("i64")),
        IdlType::F64 => value
            .as_float()
            .and_then(|i| Some(borsh::to_vec(&(i as f64)).map_err(|e| encode_err("f64", e))))
            .transpose()?
            .ok_or(mismatch_err("f64")),
        IdlType::U128 => value
            .as_integer()
            .and_then(|i| Some(borsh::to_vec(&(i as u128)).map_err(|e| encode_err("u128", e))))
            .transpose()?
            .ok_or(mismatch_err("u128")),
        IdlType::I128 => value
            .as_integer()
            .and_then(|i| Some(borsh::to_vec(&i).map_err(|e| encode_err("i128", e))))
            .transpose()?
            .ok_or(mismatch_err("i128")),
        IdlType::U256 => return Err("IDL type U256 is not yet supported".to_string()),
        IdlType::I256 => return Err("IDL type I256 is not yet supported".to_string()),
        IdlType::Bytes => Ok(value.to_bytes().clone()),
        IdlType::String => value
            .as_string()
            .and_then(|s| Some(borsh::to_vec(&s).map_err(|e| encode_err("string", e))))
            .transpose()?
            .ok_or(mismatch_err("string")),
        IdlType::Pubkey => SvmValue::to_pubkey(value)
            .map_err(|_| mismatch_err("pubkey"))
            .map(|p| borsh::to_vec(&p))?
            .map_err(|e| encode_err("pubkey", e)),
        IdlType::Option(idl_type) => {
            if let Some(_) = value.as_null() {
                borsh::to_vec(&None::<u8>).map_err(|e| encode_err("Optional", e))
            } else {
                let encoded_arg = encode_value_to_idl_type(value, idl_type, idl_types, None)?;
                borsh::to_vec(&Some(encoded_arg)).map_err(|e| encode_err("Optional", e))
            }
        }
        IdlType::Vec(idl_type) => match value {
            Value::String(_) => {
                let bytes = value.get_buffer_bytes_result().map_err(|_| mismatch_err("vec"))?;
                match idl_type.as_ref() {
                    IdlType::U8 => bytes
                        .iter()
                        .map(|b| {
                            encode_value_to_idl_type(
                                &Value::integer(*b as i128),
                                idl_type,
                                idl_types,
                                None,
                            )
                        })
                        .collect::<Result<Vec<_>, _>>()
                        .map(|v| v.into_iter().flatten().collect::<Vec<_>>()),
                    _ => Err(mismatch_err("vec")),
                }
            }
            Value::Array(vec) => vec
                .iter()
                .map(|v| encode_value_to_idl_type(v, idl_type, idl_types, None))
                .collect::<Result<Vec<_>, _>>()
                .map(|v| v.into_iter().flatten().collect::<Vec<_>>()),
            Value::Buffer(bytes) => match idl_type.as_ref() {
                IdlType::U8 => bytes
                    .iter()
                    .map(|b| {
                        encode_value_to_idl_type(
                            &Value::integer(*b as i128),
                            idl_type,
                            idl_types,
                            None,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .map(|v| v.into_iter().flatten().collect::<Vec<_>>()),
                _ => Err(mismatch_err("vec")),
            },
            Value::Addon(addon_data) => match idl_type.as_ref() {
                IdlType::U8 => addon_data
                    .bytes
                    .iter()
                    .map(|b| borsh::to_vec(&b).map_err(|e| encode_err("u8", e)))
                    .collect::<Result<Vec<_>, _>>()
                    .map(|v| v.into_iter().flatten().collect::<Vec<_>>()),
                _ => Err(mismatch_err("vec")),
            },
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
                        .map(|v| encode_value_to_idl_type(v, idl_type, idl_types, None))
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
                    if let Some(fields) = fields {
                        let mut encoded_fields = vec![];
                        match fields {
                            IdlDefinedFields::Named(expected_fields) => {
                                let user_values_map =
                                    value.as_object().ok_or(mismatch_err("object"))?;
                                for field in expected_fields {
                                    let user_value =
                                        user_values_map.get(&field.name).ok_or_else(|| {
                                            format!("missing field '{}' in object", field.name)
                                        })?;

                                    let ty = parse_generic_expected_type(
                                        &field.ty,
                                        &typing.generics,
                                        generics,
                                    )?;

                                    let mut encoded_field = encode_value_to_idl_type(
                                        user_value,
                                        &ty,
                                        idl_types,
                                        Some(idl_type),
                                    )?;
                                    encoded_fields.append(&mut encoded_field);
                                }
                            }
                            IdlDefinedFields::Tuple(expected_tuple_types) => {
                                return Err(
                                    "Encoding tuple structs are not supported by txtx".to_string()
                                );
                                let user_values = value.as_array().ok_or(mismatch_err("array"))?;
                                let mut encoded_tuple_fields = vec![];
                                for (i, expected_type) in expected_tuple_types.iter().enumerate() {
                                    let user_value = user_values.get(i).ok_or_else(|| {
                                        format!("missing field value in {} index of array", i)
                                    })?;

                                    let ty = parse_generic_expected_type(
                                        expected_type,
                                        &typing.generics,
                                        generics,
                                    )?;

                                    let encoded_field = encode_value_to_idl_type(
                                        user_value,
                                        &ty,
                                        idl_types,
                                        Some(idl_type),
                                    )?;
                                    encoded_tuple_fields.push(encoded_field);
                                }
                                encoded_fields.append(
                                    &mut borsh::to_vec(&encoded_tuple_fields)
                                        .map_err(|e| encode_err("tuple", e))?,
                                );
                            }
                        }
                        encoded_fields
                    } else {
                        vec![]
                    }
                }
                IdlTypeDefTy::Enum { .. } => {
                    return Err("Encoding enums are not supported by txtx".to_string());
                }
                IdlTypeDefTy::Type { alias } => {
                    encode_value_to_idl_type(value, &alias, idl_types, Some(idl_type))?
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
                    encode_value_to_idl_type(value, &ty, idl_types, None)
                }
                IdlTypeDefGeneric::Const { ty, .. } => {
                    let ty =
                        IdlType::from_str(ty).map_err(|e| format!("invalid generic type: {e}"))?;
                    encode_value_to_idl_type(value, &ty, idl_types, None)
                }
            }
        }
        t => return Err(format!("IDL type {:?} is not yet supported", t)),
    }
}

/// Parses a byte array into a JSON value based on the expected IDL type.
/// **Not used internally, but is used by crate consumers.**
pub fn parse_bytes_to_value_with_expected_idl_type(
    mut data: &[u8],
    expected_type: &IdlTypeDefTy,
) -> Result<Value, String> {
    match &expected_type {
        IdlTypeDefTy::Struct { fields } => {
            if let Some(fields) = fields {
                match fields {
                    IdlDefinedFields::Named(idl_fields) => {
                        let mut map = IndexMap::new();
                        for field in idl_fields {
                            let field_name = field.name.clone();
                            let value =
                                match &field.ty {
                                    IdlType::U8 => {
                                        let (v, rest) = data.split_at(1);
                                        data = rest;
                                        Ok(Value::integer(
                                            u8::try_from(v[0])
                                                .map_err(|e| format!("unable to decode u8: {e}"))?
                                                .into(),
                                        ))
                                    }
                                    IdlType::U16 => {
                                        let (v, rest) = data.split_at(2);
                                        data = rest;
                                        Ok(Value::integer(
                                            u16::from_le_bytes(<[u8; 2]>::try_from(v).map_err(
                                                |e| format!("unable to decode u16: {e}"),
                                            )?)
                                            .into(),
                                        ))
                                    }
                                    IdlType::U32 => {
                                        let (v, rest) = data.split_at(4);
                                        data = rest;
                                        Ok(Value::integer(
                                            u32::from_le_bytes(<[u8; 4]>::try_from(v).map_err(
                                                |e| format!("unable to decode u32: {e}"),
                                            )?)
                                            .into(),
                                        ))
                                    }
                                    IdlType::U64 => {
                                        let (v, rest) = data.split_at(8);
                                        data = rest;
                                        Ok(Value::integer(
                                            u64::from_le_bytes(<[u8; 8]>::try_from(v).map_err(
                                                |e| format!("unable to decode u64: {e}"),
                                            )?)
                                            .into(),
                                        ))
                                    }
                                    IdlType::U128 => {
                                        let (v, rest) = data.split_at(16);
                                        data = rest;
                                        Ok(Value::integer(
                                            u128::from_le_bytes(<[u8; 16]>::try_from(v).map_err(
                                                |e| format!("unable to decode u128: {e}"),
                                            )?)
                                            .try_into()
                                            .map_err(|e| {
                                                format!("unable to convert u128 to i128: {e}")
                                            })?,
                                        ))
                                    }
                                    IdlType::I8 => {
                                        let (v, rest) = data.split_at(1);
                                        data = rest;
                                        Ok(Value::integer(
                                            i8::try_from(v[0])
                                                .map_err(|e| format!("unable to decode i8: {e}"))?
                                                .into(),
                                        ))
                                    }
                                    IdlType::I16 => {
                                        let (v, rest) = data.split_at(2);
                                        data = rest;
                                        Ok(Value::integer(
                                            i16::from_le_bytes(<[u8; 2]>::try_from(v).map_err(
                                                |e| format!("unable to decode i16: {e}"),
                                            )?)
                                            .into(),
                                        ))
                                    }
                                    IdlType::I32 => {
                                        let (v, rest) = data.split_at(4);
                                        data = rest;
                                        Ok(Value::integer(
                                            i32::from_le_bytes(<[u8; 4]>::try_from(v).map_err(
                                                |e| format!("unable to decode i32: {e}"),
                                            )?)
                                            .into(),
                                        ))
                                    }
                                    IdlType::I64 => {
                                        let (v, rest) = data.split_at(8);
                                        data = rest;
                                        Ok(Value::integer(
                                            i64::from_le_bytes(<[u8; 8]>::try_from(v).map_err(
                                                |e| format!("unable to decode i64: {e}"),
                                            )?)
                                            .into(),
                                        ))
                                    }
                                    IdlType::I128 => {
                                        let (v, rest) = data.split_at(16);
                                        data = rest;
                                        Ok(Value::integer(i128::from_le_bytes(
                                            <[u8; 16]>::try_from(v).map_err(|e| {
                                                format!("unable to decode i128: {e}")
                                            })?,
                                        )))
                                    }
                                    IdlType::Bool => {
                                        let (v, rest) = data.split_at(1);
                                        data = rest;
                                        Ok(Value::bool(v[0] != 0))
                                    }
                                    IdlType::Pubkey => {
                                        let (v, rest) = data.split_at(32);
                                        data = rest;
                                        Ok(SvmValue::pubkey(v.to_vec()))
                                    }
                                    IdlType::String => {
                                        let string_len = u32::from_le_bytes(
                                            <[u8; 4]>::try_from(&data[..4]).map_err(|e| {
                                                format!("unable to decode string length: {e}")
                                            })?,
                                        )
                                            as usize;
                                        data = &data[4..]; // Move past length bytes
                                        let (string_bytes, rest) = data.split_at(string_len);
                                        data = rest;
                                        let string_value =
                                            String::from_utf8_lossy(string_bytes).to_string();
                                        Ok(Value::string(string_value))
                                    }
                                    IdlType::Bytes => {
                                        let vec_len = u32::from_le_bytes(
                                            <[u8; 4]>::try_from(&data[..4]).map_err(|e| {
                                                format!("unable to decode bytes length: {e}")
                                            })?,
                                        )
                                            as usize;
                                        data = &data[4..]; // Move past length bytes
                                        let (vec_bytes, rest) = data.split_at(vec_len);
                                        data = rest;
                                        Ok(Value::buffer(vec_bytes.to_vec()))
                                    }
                                    _ => Err(format!("Unsupported type: {:?}", field.ty)),
                                }?;
                            map.insert(field_name, value);
                        }
                        Ok(ObjectType::from_map(map).to_value())
                    }
                    IdlDefinedFields::Tuple(idl_types) => todo!(),
                }
            } else {
                todo!()
            }
        }
        IdlTypeDefTy::Enum { variants } => todo!(),
        IdlTypeDefTy::Type { alias } => match alias {
            IdlType::Bool => {
                let value = borsh::from_slice::<bool>(&data)
                    .map_err(|e| format!("unable to decode bool: {e}"))?;
                Ok(Value::bool(value))
            }
            IdlType::U8 => Ok(Value::integer(
                borsh::from_slice::<u8>(&data)
                    .map_err(|e| format!("unable to decode u8: {e}"))?
                    .into(),
            )),
            IdlType::I8 => Ok(Value::integer(
                borsh::from_slice::<i8>(&data)
                    .map_err(|e| format!("unable to decode i8: {e}"))?
                    .into(),
            )),
            IdlType::U16 => Ok(Value::integer(
                borsh::from_slice::<u16>(&data)
                    .map_err(|e| format!("unable to decode u16: {e}"))?
                    .into(),
            )),
            IdlType::I16 => Ok(Value::integer(
                borsh::from_slice::<i16>(&data)
                    .map_err(|e| format!("unable to decode i16: {e}"))?
                    .into(),
            )),
            IdlType::U32 => Ok(Value::integer(
                borsh::from_slice::<u32>(&data)
                    .map_err(|e| format!("unable to decode u32: {e}"))?
                    .into(),
            )),
            IdlType::I32 => Ok(Value::integer(
                borsh::from_slice::<i32>(&data)
                    .map_err(|e| format!("unable to decode i32: {e}"))?
                    .into(),
            )),
            IdlType::U64 => Ok(Value::integer(
                borsh::from_slice::<u64>(&data)
                    .map_err(|e| format!("unable to decode u64: {e}"))?
                    .into(),
            )),
            IdlType::I64 => Ok(Value::integer(
                borsh::from_slice::<i64>(&data)
                    .map_err(|e| format!("unable to decode i64: {e}"))?
                    .into(),
            )),
            IdlType::F64 => Ok(Value::integer(
                borsh::from_slice::<i64>(&data)
                    .map_err(|e| format!("unable to decode i64: {e}"))?
                    .into(),
            )),
            // IdlType::I128 => Ok(JsonValue::Number(
            //     borsh::from_slice::<i128>(&data)
            //         .map_err(|e| format!("unable to decode i128: {e}"))?
            //         .into(),
            // )),
            IdlType::Bytes => {
                let bytes = borsh::from_slice::<Vec<u8>>(&data)
                    .map_err(|e| format!("unable to decode bytes: {e}"))?;

                Ok(Value::buffer(bytes))
            }
            IdlType::String => borsh::from_slice::<String>(&data)
                .map(Value::string)
                .map_err(|e| format!("unable to decode string: {e}")),
            IdlType::Pubkey => {
                let value = borsh::from_slice::<Pubkey>(&data)
                    .map_err(|e| format!("unable to decode pubkey: {e}"))?;
                Ok(SvmValue::pubkey(value.to_bytes().to_vec()))
            }
            IdlType::Option(idl_type) => todo!(),
            IdlType::Vec(idl_type) => {
                todo!()
            }
            IdlType::Array(idl_type, idl_array_len) => todo!(),
            IdlType::Defined { name, generics } => todo!(),
            IdlType::Generic(_) => todo!(),
            _ => return Err(format!("Unsupported type: {:?}", alias)),
        },
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

pub fn classic_idl_to_anchor_metadata(
    classic_idl: &ClassicIdl,
) -> anchor_lang_idl::types::IdlMetadata {
    anchor_lang_idl::types::IdlMetadata {
        name: classic_idl.name.to_string(),
        version: classic_idl.version.clone(),
        spec: classic_idl.version.clone(),
        description: None,
        repository: None,
        dependencies: vec![],
        contact: None,
        deployments: None,
    }
}

pub fn classic_instruction_to_anchor_instruction(
    classic_instruction: &solana_idl::IdlInstruction,
    classic_types: &Vec<solana_idl::IdlTypeDefinition>,
) -> Result<anchor_lang_idl::types::IdlInstruction, String> {
    let mut discriminator = vec![];
    if let Some(classic_discriminator) = &classic_instruction.discriminant {
        if let Some(bytes) = &classic_discriminator.bytes {
            discriminator = bytes.clone();
        }
    }
    if discriminator.is_empty() {
        discriminator = compute_discriminator("global", &classic_instruction.name);
    }

    Ok(anchor_lang_idl::types::IdlInstruction {
        name: classic_instruction.name.to_string(),
        accounts: classic_instruction
            .accounts
            .iter()
            .map(|a| classic_account_item_to_anchor_instruction_account(a))
            .collect(),
        args: classic_instruction
            .args
            .iter()
            .map(|a| classic_field_to_anchor_field(a, classic_types))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("error in instruction {}: {}", classic_instruction.name, e))?,
        docs: vec![],
        discriminator,
        returns: None,
    })
}

pub fn classic_account_item_to_anchor_instruction_account(
    classic_account: &solana_idl::IdlAccountItem,
) -> anchor_lang_idl::types::IdlInstructionAccountItem {
    match classic_account {
        solana_idl::IdlAccountItem::IdlAccount(a) => {
            anchor_lang_idl::types::IdlInstructionAccountItem::Single(IdlInstructionAccount {
                name: a.name.to_string(),
                docs: a.docs.clone().unwrap_or_default(),
                writable: a.is_mut,
                signer: a.is_signer,
                optional: a.optional,
                address: a.address.clone(),
                pda: None, // todo
                relations: vec![],
            })
        }
        solana_idl::IdlAccountItem::IdlAccounts(a) => {
            anchor_lang_idl::types::IdlInstructionAccountItem::Composite(IdlInstructionAccounts {
                name: a.name.to_string(),
                accounts: a
                    .accounts
                    .iter()
                    .map(|a| classic_account_item_to_anchor_instruction_account(a))
                    .collect(),
            })
        }
    }
}

pub fn classic_field_to_anchor_field(
    classic_field: &solana_idl::IdlField,
    classic_types: &Vec<solana_idl::IdlTypeDefinition>,
) -> Result<anchor_lang_idl::types::IdlField, String> {
    Ok(anchor_lang_idl::types::IdlField {
        name: classic_field.name.to_string(),
        ty: classic_type_to_anchor_type(&classic_field.ty, classic_types)
            .map_err(|e| format!("error in field {}: {}", classic_field.name, e))?,
        docs: classic_field.attrs.clone().unwrap_or_default(),
    })
}

pub fn classic_type_to_anchor_type(
    classic_type: &solana_idl::IdlType,
    classic_types: &Vec<solana_idl::IdlTypeDefinition>,
) -> Result<anchor_lang_idl::types::IdlType, String> {
    let ty =
        match classic_type {
            solana_idl::IdlType::Bool => anchor_lang_idl::types::IdlType::Bool,
            solana_idl::IdlType::U8 => anchor_lang_idl::types::IdlType::U8,
            solana_idl::IdlType::I8 => anchor_lang_idl::types::IdlType::I8,
            solana_idl::IdlType::U16 => anchor_lang_idl::types::IdlType::U16,
            solana_idl::IdlType::I16 => anchor_lang_idl::types::IdlType::I16,
            solana_idl::IdlType::U32 => anchor_lang_idl::types::IdlType::U32,
            solana_idl::IdlType::I32 => anchor_lang_idl::types::IdlType::I32,
            solana_idl::IdlType::F32 => anchor_lang_idl::types::IdlType::F32,
            solana_idl::IdlType::U64 => anchor_lang_idl::types::IdlType::U64,
            solana_idl::IdlType::I64 => anchor_lang_idl::types::IdlType::I64,
            solana_idl::IdlType::F64 => anchor_lang_idl::types::IdlType::F64,
            solana_idl::IdlType::U128 => anchor_lang_idl::types::IdlType::U128,
            solana_idl::IdlType::I128 => anchor_lang_idl::types::IdlType::I128,
            solana_idl::IdlType::Bytes => anchor_lang_idl::types::IdlType::Bytes,
            solana_idl::IdlType::String => anchor_lang_idl::types::IdlType::String,
            solana_idl::IdlType::PublicKey => anchor_lang_idl::types::IdlType::Pubkey,
            solana_idl::IdlType::Option(ty) => anchor_lang_idl::types::IdlType::Option(Box::new(
                classic_type_to_anchor_type(ty, classic_types)?,
            )),
            solana_idl::IdlType::Vec(ty) => anchor_lang_idl::types::IdlType::Vec(Box::new(
                classic_type_to_anchor_type(ty, classic_types)?,
            )),
            solana_idl::IdlType::Array(ty, len) => anchor_lang_idl::types::IdlType::Array(
                Box::new(classic_type_to_anchor_type(ty, classic_types)?),
                IdlArrayLen::Value(*len),
            ),
            solana_idl::IdlType::COption(idl_type) => anchor_lang_idl::types::IdlType::Option(
                Box::new(classic_type_to_anchor_type(idl_type, classic_types)?),
            ),
            solana_idl::IdlType::Defined(type_name) => {
                let type_def = classic_types
                    .iter()
                    .find(|t| t.name.eq(type_name))
                    .ok_or(format!("unable to find type definition for {} in idl", type_name))?;
                return Err(
                "Defined types are not yet supported when converting from classic to anchor IDL"
                    .to_string(),
            );
            }
            solana_idl::IdlType::Tuple(vec) => {
                return Err(
                    "Tuple types are not yet supported when converting from classic to anchor IDL"
                        .to_string(),
                )
            }
            solana_idl::IdlType::HashMap(idl_type, idl_type1) => return Err(
                "HashMap types are not yet supported when converting from classic to anchor IDL"
                    .to_string(),
            ),
            solana_idl::IdlType::BTreeMap(idl_type, idl_type1) => return Err(
                "BTreeMap types are not yet supported when converting from classic to anchor IDL"
                    .to_string(),
            ),
            solana_idl::IdlType::HashSet(idl_type) => return Err(
                "HashSet types are not yet supported when converting from classic to anchor IDL"
                    .to_string(),
            ),
            solana_idl::IdlType::BTreeSet(idl_type) => return Err(
                "BTreeSet types are not yet supported when converting from classic to anchor IDL"
                    .to_string(),
            ),
        };
    Ok(ty)
}

pub fn classic_account_to_anchor_account(
    classic_account: solana_idl::IdlTypeDefinition,
) -> anchor_lang_idl::types::IdlAccount {
    anchor_lang_idl::types::IdlAccount {
        name: classic_account.name.clone(),
        discriminator: compute_discriminator("account", &classic_account.name),
    }
}

pub fn classic_idl_type_def_ty_to_anchor_type_def_ty(
    classic_type_def_ty: &solana_idl::IdlTypeDefinitionTy,
    classic_idl_types: &Vec<solana_idl::IdlTypeDefinition>,
) -> Result<anchor_lang_idl::types::IdlTypeDefTy, String> {
    match &classic_type_def_ty {
        solana_idl::IdlTypeDefinitionTy::Struct { fields } => {
            Ok(anchor_lang_idl::types::IdlTypeDefTy::Struct {
                fields: Some(anchor_lang_idl::types::IdlDefinedFields::Tuple(
                    fields
                        .iter()
                        .map(|field| classic_type_to_anchor_type(&field.ty, &classic_idl_types))
                        .collect::<Result<Vec<_>, _>>()?,
                )),
            })
        }
        solana_idl::IdlTypeDefinitionTy::Enum { variants } => {
            Ok(anchor_lang_idl::types::IdlTypeDefTy::Enum {
                variants: variants
                    .iter()
                    .map(|v| {
                        Ok(anchor_lang_idl::types::IdlEnumVariant {
                            name: v.name.clone(),
                            fields: match &v.fields {
                                Some(fields) => match fields {
                                    solana_idl::EnumFields::Named(fields) => {
                                        Some(anchor_lang_idl::types::IdlDefinedFields::Named(
                                            fields
                                                .iter()
                                                .map(|field| {
                                                    Ok(anchor_lang_idl::types::IdlField {
                                                        name: field.name.clone(),
                                                        ty: classic_type_to_anchor_type(
                                                            &field.ty,
                                                            &classic_idl_types,
                                                        )?,
                                                        docs: vec![],
                                                    })
                                                })
                                                .collect::<Result<
                                                    Vec<anchor_lang_idl::types::IdlField>,
                                                    String,
                                                >>(
                                                )?,
                                        ))
                                    }
                                    solana_idl::EnumFields::Tuple(fields) => {
                                        Some(anchor_lang_idl::types::IdlDefinedFields::Tuple(
                                            fields
                                                .iter()
                                                .map(|ty| {
                                                    classic_type_to_anchor_type(
                                                        &ty,
                                                        &classic_idl_types,
                                                    )
                                                })
                                                .collect::<Result<Vec<_>, _>>()?,
                                        ))
                                    }
                                },
                                None => None,
                            },
                        })
                    })
                    .collect::<Result<Vec<anchor_lang_idl::types::IdlEnumVariant>, String>>()?,
            })
        }
    }
}

pub fn classic_idl_type_def_to_anchor_type_def(
    classic_type_def: &solana_idl::IdlTypeDefinition,
    classic_idl_types: &Vec<solana_idl::IdlTypeDefinition>,
    idl_serializer: &anchor_lang_idl::types::IdlSerialization,
) -> Result<anchor_lang_idl::types::IdlTypeDef, String> {
    Ok(anchor_lang_idl::types::IdlTypeDef {
        name: classic_type_def.name.clone(),
        docs: vec![],
        serialization: idl_serializer.clone(),
        repr: None,
        generics: vec![], // todo
        ty: classic_idl_type_def_ty_to_anchor_type_def_ty(&classic_type_def.ty, classic_idl_types)
            .map_err(|e| format!("error in type {}: {}", classic_type_def.name, e))?,
    })
}

pub fn compute_discriminator(prefix: &str, input: &str) -> Vec<u8> {
    let prefixed_input = format!("{}:{}", prefix, input);
    let mut result = [0u8; 8];
    result.copy_from_slice(&solana_program::hash::hash(prefixed_input.as_bytes()).to_bytes()[..8]);
    let result = result.to_vec();
    result
}

pub fn classic_idl_types_to_anchor_types(
    classic_types: &Vec<solana_idl::IdlTypeDefinition>,
    idl_serializer: &anchor_lang_idl::types::IdlSerialization,
) -> Result<Vec<anchor_lang_idl::types::IdlTypeDef>, String> {
    classic_types
        .iter()
        .map(|t| classic_idl_type_def_to_anchor_type_def(t, classic_types, idl_serializer))
        .collect()
}

pub fn classic_const_to_anchor_const(
    classic_const: &solana_idl::IdlConst,
    classic_types: &Vec<solana_idl::IdlTypeDefinition>,
) -> Result<anchor_lang_idl::types::IdlConst, String> {
    Ok(anchor_lang_idl::types::IdlConst {
        name: classic_const.name.clone(),
        ty: classic_type_to_anchor_type(&classic_const.ty, &classic_types)?,
        value: classic_const.value.clone(),
        docs: vec![],
    })
}
