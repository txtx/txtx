use std::{path::PathBuf, str::FromStr};

use anchor_lang_idl::types::{
    Idl, IdlArrayLen, IdlDefinedFields, IdlGenericArg, IdlInstruction, IdlType, IdlTypeDef,
    IdlTypeDefGeneric, IdlTypeDefTy,
};
use solana_sdk::pubkey::Pubkey;
use txtx_addon_kit::{helpers::fs::FileLocation, types::types::Value};

pub struct IdlRef {
    pub idl: Idl,
    pub location: FileLocation,
}

impl IdlRef {
    pub fn new(location: FileLocation) -> Result<Self, String> {
        let idl_str =
            location.read_content_as_utf8().map_err(|e| format!("unable to read idl: {e}"))?;
        let idl = serde_json::from_str(&idl_str).map_err(|e| format!("invalid idl: {e}"))?;
        Ok(Self { idl, location })
    }

    pub fn from_idl(idl: Idl) -> Self {
        Self { idl, location: FileLocation::FileSystem { path: PathBuf::default() } }
    }

    pub fn get_discriminator(&self, instruction_name: &str) -> Result<Vec<u8>, String> {
        self.get_instruction(instruction_name).map(|i| i.discriminator.clone())
    }

    pub fn get_instruction(&self, instruction_name: &str) -> Result<&IdlInstruction, String> {
        self.idl
            .instructions
            .iter()
            .find(|i| i.name == instruction_name)
            .ok_or_else(|| format!("instruction not found: {instruction_name}"))
    }

    pub fn get_types(&self) -> Vec<IdlTypeDef> {
        self.idl.types.clone()
    }

    pub fn get_encoded_args(
        &self,
        instruction_name: &str,
        args: Vec<Value>,
    ) -> Result<Vec<u8>, String> {
        let instruction = self.get_instruction(instruction_name)?;
        if args.len() != instruction.args.len() {
            return Err(format!(
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
                    format!("error in argument at position {}: {}", user_arg_idx + 1, e)
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
        IdlType::U256 => todo!(),
        IdlType::I256 => todo!(),
        IdlType::Bytes => value.as_buffer_data().cloned().ok_or(mismatch_err("bytes")),
        IdlType::String => value
            .as_string()
            .and_then(|s| Some(borsh::to_vec(&s).map_err(|e| encode_err("string", e))))
            .transpose()?
            .ok_or(mismatch_err("string")),
        IdlType::Pubkey => {
            let thing = value.try_get_buffer_bytes_result().map(|b| {
                b.map(|b| {
                    if let Ok(pubkey_array) =
                        b.as_slice().try_into().map_err(|_| mismatch_err("pubkey"))
                    {
                        let pubkey = Pubkey::new_from_array(pubkey_array);
                        borsh::to_vec(&pubkey).map_err(|e| encode_err("pubkey", e))
                    } else {
                        Err(mismatch_err("pubkey"))
                    }
                })
                .transpose()?
                .ok_or(mismatch_err("pubkey"))
            })?;

            thing
        }
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
                let bytes = value.expect_buffer_bytes_result().map_err(|_| mismatch_err("vec"))?;
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
                        todo!()
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
