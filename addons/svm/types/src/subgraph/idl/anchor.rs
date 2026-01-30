use anchor_lang_idl::types::{
    IdlArrayLen, IdlConst, IdlDefinedFields, IdlGenericArg, IdlInstruction,
    IdlInstructionAccountItem, IdlType, IdlTypeDef, IdlTypeDefGeneric, IdlTypeDefTy,
};
use solana_pubkey::Pubkey;
use txtx_addon_kit::{
    indexmap::IndexMap,
    types::types::{ObjectDefinition, ObjectProperty, ObjectType, Type, Value},
};

use crate::{SvmValue, I256, SVM_PUBKEY, U256};
use std::{fmt::Display, str::FromStr};

pub fn get_expected_type_from_idl_defined_fields(
    fields: &IdlDefinedFields,
    idl_types: &Vec<IdlTypeDef>,
    idl_constants: &Vec<IdlConst>,
    generic_args: &Vec<IdlGenericArg>,
    idl_type_def_generics: &Vec<IdlTypeDefGeneric>,
) -> Result<Type, String> {
    match fields {
        IdlDefinedFields::Named(idl_fields) => {
            let mut props = vec![];
            for field in idl_fields {
                let ty = idl_type_to_txtx_type(
                    field.ty.clone(),
                    idl_types,
                    idl_constants,
                    generic_args,
                    idl_type_def_generics,
                )
                .map_err(|e| {
                    format!("could not determine expected type for field '{}': {e}", field.name)
                })?;
                props.push(ObjectProperty {
                    documentation: field.docs.join(" "),
                    typing: ty,
                    optional: false,
                    tainting: false,
                    name: field.name.clone(),
                    internal: false,
                });
            }
            Ok(Type::object(ObjectDefinition::strict(props)))
        }
        IdlDefinedFields::Tuple(tuple_idl_types) => {
            let mut tuple_props = vec![];
            for idl_type in tuple_idl_types {
                let expected_type = idl_type_to_txtx_type(
                    idl_type.clone(),
                    idl_types,
                    idl_constants,
                    generic_args,
                    idl_type_def_generics,
                )?;
                tuple_props.push(ObjectProperty {
                    documentation: "".into(), // Tuples do not have documentation for fields
                    typing: expected_type,
                    optional: false,
                    tainting: false,
                    name: format!("field_{}", tuple_props.len()),
                    internal: false,
                });
            }
            Ok(Type::object(ObjectDefinition::tuple(tuple_props)))
        }
    }
}

pub fn get_expected_type_from_idl_type_def_ty(
    idl_type_def_ty: &IdlTypeDefTy,
    idl_types: &Vec<IdlTypeDef>,
    idl_constants: &Vec<IdlConst>,
    generic_args: &Vec<IdlGenericArg>,
    idl_type_def_generics: &Vec<IdlTypeDefGeneric>,
) -> Result<Type, String> {
    let ty = match idl_type_def_ty {
        IdlTypeDefTy::Struct { fields } => {
            if let Some(fields) = fields {
                get_expected_type_from_idl_defined_fields(
                    fields,
                    idl_types,
                    idl_constants,
                    generic_args,
                    idl_type_def_generics,
                )?
            } else {
                Type::object(ObjectDefinition::strict(vec![])) // Empty struct
            }
        }
        IdlTypeDefTy::Enum { variants } => {
            let mut props = vec![];
            for variant in variants {
                if let Some(ref fields) = variant.fields {
                    let expected_type = get_expected_type_from_idl_defined_fields(
                        fields,
                        idl_types,
                        idl_constants,
                        generic_args,
                        idl_type_def_generics,
                    )?;
                    props.push(ObjectProperty {
                        documentation: "".into(), // Enums do not have documentation for variants
                        typing: expected_type,
                        optional: false,
                        tainting: false,
                        name: variant.name.clone(),
                        internal: false,
                    });
                } else {
                    props.push(ObjectProperty {
                        documentation: "".into(),
                        typing: Type::null(), // No fields means unit type, which we represent as null
                        optional: false,
                        tainting: false,
                        name: variant.name.clone(),
                        internal: false,
                    });
                }
            }
            Type::object(ObjectDefinition::enum_type(props))
        }
        IdlTypeDefTy::Type { alias: _ } => todo!(),
    };
    Ok(ty)
}

pub fn idl_type_to_txtx_type(
    idl_type: IdlType,
    idl_types: &Vec<IdlTypeDef>,
    idl_constants: &Vec<IdlConst>,
    generic_args: &Vec<IdlGenericArg>,
    idl_type_def_generics: &Vec<IdlTypeDefGeneric>,
) -> Result<Type, String> {
    let res = match idl_type {
        IdlType::Bool => Type::bool(),
        IdlType::U8 => Type::addon(crate::SVM_U8),
        IdlType::U16 => Type::addon(crate::SVM_U16),
        IdlType::U32 => Type::addon(crate::SVM_U32),
        IdlType::U64 => Type::addon(crate::SVM_U64),
        IdlType::U128 => Type::addon(crate::SVM_U128),
        IdlType::U256 => Type::addon(crate::SVM_U256),
        IdlType::I8 => Type::addon(crate::SVM_I8),
        IdlType::I16 => Type::addon(crate::SVM_I16),
        IdlType::I32 => Type::addon(crate::SVM_I32),
        IdlType::I64 => Type::addon(crate::SVM_I64),
        IdlType::I128 => Type::addon(crate::SVM_I128),
        IdlType::I256 => Type::addon(crate::SVM_I256),
        IdlType::F32 => Type::addon(crate::SVM_F32),
        IdlType::F64 => Type::addon(crate::SVM_F64),
        IdlType::Bytes => Type::buffer(),
        IdlType::String => Type::string(),
        IdlType::Pubkey => Type::addon(SVM_PUBKEY),
        IdlType::Option(idl_type) => Type::typed_null(idl_type_to_txtx_type(
            *idl_type,
            idl_types,
            idl_constants,
            generic_args,
            idl_type_def_generics,
        )?),
        IdlType::Vec(idl_type) => Type::array(idl_type_to_txtx_type(
            *idl_type,
            idl_types,
            idl_constants,
            generic_args,
            idl_type_def_generics,
        )?),
        IdlType::Array(idl_type, ..) => Type::array(idl_type_to_txtx_type(
            *idl_type,
            idl_types,
            idl_constants,
            generic_args,
            idl_type_def_generics,
        )?),
        IdlType::Defined { name, generics } => {
            let Some(matching_idl_type) = idl_types.iter().find(|t| t.name == name) else {
                return Err(format!("unable to find defined type '{}'", name));
            };
            let expected_type = get_expected_type_from_idl_type_def_ty(
                &matching_idl_type.ty,
                idl_types,
                idl_constants,
                &generics,
                &matching_idl_type.generics,
            )?;
            expected_type
        }
        IdlType::Generic(generic_name) => {
            let index_of_matching_generic = idl_type_def_generics
                .iter()
                .position(|g| match g {
                    IdlTypeDefGeneric::Type { name } => name.eq(&generic_name),
                    IdlTypeDefGeneric::Const { name, .. } => name.eq(&generic_name),
                })
                .ok_or(format!("unable to find generic type '{}'", generic_name))?;

            let generic_arg = generic_args
                .get(index_of_matching_generic)
                .ok_or(format!("unable to find generic argument for '{}'", generic_name))?;

            match generic_arg {
                IdlGenericArg::Type { ty } => idl_type_to_txtx_type(
                    ty.clone(),
                    idl_types,
                    idl_constants,
                    generic_args,
                    idl_type_def_generics,
                )
                .map_err(|e| format!("unable to resolve generic type '{}': {}", generic_name, e))?,
                IdlGenericArg::Const { .. } => todo!(),
            }
        }
        _ => todo!(),
    };
    Ok(res)
}

pub fn parse_bytes_to_value_with_expected_idl_type_def_ty(
    data: &[u8],
    expected_type: &IdlTypeDefTy,
    idl_types: &Vec<IdlTypeDef>,
    generic_args: &Vec<IdlGenericArg>,
    idl_type_def_generics: &Vec<IdlTypeDefGeneric>,
) -> Result<Value, String> {
    let (value, rest) = parse_bytes_to_value_with_expected_idl_type_def_ty_with_leftover_bytes(
        data,
        expected_type,
        idl_types,
        generic_args,
        idl_type_def_generics,
    )?;
    if !rest.is_empty() {
        // If the remaining bytes are all zeros, this is okay - it indicates that an account was initialized
        // with a certain amount of space, and the data didn't fill that space, so the rest is filled with zeroes.
        // if there are any other values, return an error
        if rest.iter().any(|&byte| byte != 0) {
            return Err(format!(
                "expected no leftover bytes after parsing type {:?}, but found {} bytes of non-zero data",
                expected_type
                ,rest.len()
            ));
        }
    }
    Ok(value)
}

pub fn parse_bytes_to_value_with_expected_idl_type_def_ty_with_leftover_bytes<'a>(
    mut data: &'a [u8],
    expected_type: &IdlTypeDefTy,
    idl_types: &Vec<IdlTypeDef>,
    generic_args: &Vec<IdlGenericArg>,
    idl_type_def_generics: &Vec<IdlTypeDefGeneric>,
) -> Result<(Value, &'a [u8]), String> {
    match &expected_type {
        IdlTypeDefTy::Struct { fields } => {
            parse_bytes_to_expected_idl_defined_fields_with_leftover_bytes(
                data,
                fields,
                idl_types,
                generic_args,
                idl_type_def_generics,
            )
        }
        IdlTypeDefTy::Enum { variants } => {
            let (variant, rest) =
                data.split_at_checked(1).ok_or("not enough bytes to decode enum variant index")?;

            let variant_index = variant[0] as usize;
            if variant_index >= variants.len() {
                return Err(format!(
                    "invalid enum variant index: {} for enum with {} variants",
                    variant_index,
                    variants.len()
                ));
            }
            data = rest; // Update data to the remaining bytes after parsing the variant index
            let variant = &variants[variant_index];
            let (value, rest) = parse_bytes_to_expected_idl_defined_fields_with_leftover_bytes(
                data,
                &variant.fields,
                idl_types,
                generic_args,
                idl_type_def_generics,
            )?;
            Ok((ObjectType::from([(&variant.name, value)]).to_value(), rest))
        }
        IdlTypeDefTy::Type { alias } => {
            parse_bytes_to_value_with_expected_idl_type_with_leftover_bytes(
                data,
                alias,
                idl_types,
                generic_args,
                idl_type_def_generics,
            )
        }
    }
}

pub fn parse_bytes_to_expected_idl_defined_fields_with_leftover_bytes<'a>(
    data: &'a [u8],
    fields: &Option<IdlDefinedFields>,
    idl_types: &Vec<IdlTypeDef>,
    generic_args: &Vec<IdlGenericArg>,
    idl_type_def_generics: &Vec<IdlTypeDefGeneric>,
) -> Result<(Value, &'a [u8]), String> {
    if let Some(fields) = fields {
        match fields {
            IdlDefinedFields::Named(idl_fields) => {
                let mut map = IndexMap::new();
                let mut remaining_data = data;
                for field in idl_fields {
                    let field_name = field.name.clone();
                    let (value, rest) =
                        parse_bytes_to_value_with_expected_idl_type_with_leftover_bytes(
                            remaining_data,
                            &field.ty,
                            idl_types,
                            generic_args,
                            idl_type_def_generics,
                        )?;
                    remaining_data = rest; // Update remaining data to the leftover bytes after parsing this field
                    map.insert(field_name, value);
                }
                Ok((ObjectType::from_map(map).to_value(), remaining_data))
            }
            IdlDefinedFields::Tuple(tuple_types) => {
                let mut map = IndexMap::new();
                let mut remaining_data = data;

                for (i, idl_type) in tuple_types.iter().enumerate() {
                    let field_name = format!("field_{i}");
                    let (value, rest) =
                        parse_bytes_to_value_with_expected_idl_type_with_leftover_bytes(
                            remaining_data,
                            &idl_type,
                            idl_types,
                            &generic_args,
                            idl_type_def_generics,
                        )?;
                    remaining_data = rest; // Update remaining data to the leftover bytes after parsing this field
                    map.insert(field_name, value);
                }
                Ok((ObjectType::from_map(map).to_value(), remaining_data))
            }
        }
    } else {
        Ok((Value::null(), data))
    }
}

pub fn parse_bytes_to_value_with_expected_idl_type_with_leftover_bytes<'a>(
    data: &'a [u8],
    expected_type: &IdlType,
    idl_types: &Vec<IdlTypeDef>,
    generic_args: &Vec<IdlGenericArg>,
    idl_type_def_generics: &Vec<IdlTypeDefGeneric>,
) -> Result<(Value, &'a [u8]), String> {
    let err = |ty: &str, e: &dyn Display| format!("unable to decode {ty}: {e}");
    let bytes_err = |ty: &str| err(ty, &"not enough bytes");

    match expected_type {
        IdlType::U8 => {
            let (v, rest) = data.split_at_checked(1).ok_or(bytes_err("u8"))?;
            Ok((SvmValue::u8(v[0]), rest))
        }
        IdlType::U16 => {
            let (v, rest) = data.split_at_checked(2).ok_or(bytes_err("u16"))?;

            Ok((
                SvmValue::u16(
                    u16::from_le_bytes(<[u8; 2]>::try_from(v).map_err(|e| err("u16", &e))?).into(),
                ),
                rest,
            ))
        }
        IdlType::U32 => {
            let (v, rest) = data.split_at_checked(4).ok_or(bytes_err("u32"))?;

            Ok((
                SvmValue::u32(
                    u32::from_le_bytes(<[u8; 4]>::try_from(v).map_err(|e| err("u32", &e))?).into(),
                ),
                rest,
            ))
        }
        IdlType::U64 => {
            let (v, rest) = data.split_at_checked(8).ok_or(bytes_err("u64"))?;

            Ok((
                SvmValue::u64(
                    u64::from_le_bytes(<[u8; 8]>::try_from(v).map_err(|e| err("u64", &e))?).into(),
                ),
                rest,
            ))
        }
        IdlType::U128 => {
            let (v, rest) = data.split_at_checked(16).ok_or(bytes_err("u128"))?;

            Ok((
                SvmValue::u128(u128::from_le_bytes(
                    <[u8; 16]>::try_from(v).map_err(|e| err("u128", &e))?,
                )),
                rest,
            ))
        }
        IdlType::U256 => {
            let (v, rest) = data.split_at_checked(32).ok_or(bytes_err("u256"))?;

            Ok((SvmValue::u256(v.try_into().map_err(|e| err("u256", &e))?), rest))
        }
        IdlType::I8 => {
            let (v, rest) = data.split_at_checked(1).ok_or(bytes_err("i8"))?;

            Ok((
                SvmValue::i8(
                    i8::from_le_bytes(<[u8; 1]>::try_from(v).map_err(|e| err("i8", &e))?).into(),
                ),
                rest,
            ))
        }
        IdlType::I16 => {
            let (v, rest) = data.split_at_checked(2).ok_or(bytes_err("i16"))?;

            Ok((
                SvmValue::i16(
                    i16::from_le_bytes(<[u8; 2]>::try_from(v).map_err(|e| err("i16", &e))?).into(),
                ),
                rest,
            ))
        }
        IdlType::I32 => {
            let (v, rest) = data.split_at_checked(4).ok_or(bytes_err("i32"))?;

            Ok((
                SvmValue::i32(
                    i32::from_le_bytes(<[u8; 4]>::try_from(v).map_err(|e| err("i32", &e))?).into(),
                ),
                rest,
            ))
        }
        IdlType::I64 => {
            let (v, rest) = data.split_at_checked(8).ok_or(bytes_err("i64"))?;

            Ok((
                SvmValue::i64(
                    i64::from_le_bytes(<[u8; 8]>::try_from(v).map_err(|e| err("i64", &e))?).into(),
                ),
                rest,
            ))
        }
        IdlType::I128 => {
            let (v, rest) = data.split_at_checked(16).ok_or(bytes_err("i128"))?;

            Ok((
                SvmValue::i128(i128::from_le_bytes(
                    <[u8; 16]>::try_from(v).map_err(|e| err("i128", &e))?,
                )),
                rest,
            ))
        }
        IdlType::I256 => {
            let (v, rest) = data.split_at_checked(32).ok_or(bytes_err("i256"))?;

            Ok((SvmValue::i256(v.try_into().map_err(|e| err("i256", &e))?), rest))
        }
        IdlType::F32 => {
            let (v, rest) = data.split_at_checked(4).ok_or(bytes_err("f32"))?;

            Ok((
                SvmValue::f32(
                    f32::from_le_bytes(<[u8; 4]>::try_from(v).map_err(|e| err("f32", &e))?).into(),
                ),
                rest,
            ))
        }
        IdlType::F64 => {
            let (v, rest) = data.split_at_checked(8).ok_or(bytes_err("f64"))?;

            Ok((
                SvmValue::f64(
                    f64::from_le_bytes(<[u8; 8]>::try_from(v).map_err(|e| err("f64", &e))?).into(),
                ),
                rest,
            ))
        }
        IdlType::Bool => {
            let (v, rest) = data.split_at_checked(1).ok_or(bytes_err("bool"))?;

            Ok((Value::bool(v[0] != 0), rest))
        }
        IdlType::Pubkey => {
            let (v, rest) = data.split_at_checked(32).ok_or(bytes_err("pubkey"))?;

            Ok((SvmValue::pubkey(v.to_vec()), rest))
        }
        IdlType::String => {
            let (string_len, rest) = data.split_at_checked(4).ok_or(bytes_err("string length"))?;
            let string_len = u32::from_le_bytes(
                <[u8; 4]>::try_from(string_len).map_err(|e| err("string length", &e))?,
            ) as usize;

            let (string_bytes, rest) =
                rest.split_at_checked(string_len).ok_or(bytes_err("string"))?;

            let string_value = String::from_utf8_lossy(string_bytes).to_string();
            Ok((Value::string(string_value), rest))
        }
        IdlType::Bytes => {
            let (vec_len, rest) = data.split_at_checked(4).ok_or(bytes_err("bytes length"))?;
            let vec_len = u32::from_le_bytes(
                <[u8; 4]>::try_from(vec_len).map_err(|e| err("bytes length", &e))?,
            ) as usize;

            let (vec_bytes, rest) = rest.split_at_checked(vec_len).ok_or(bytes_err("bytes"))?;

            Ok((Value::buffer(vec_bytes.to_vec()), rest))
        }
        IdlType::Option(idl_type) => {
            let (is_some, rest) = data.split_at_checked(1).ok_or(bytes_err("option"))?;
            if is_some[0] == 0 {
                Ok((Value::null(), rest))
            } else {
                parse_bytes_to_value_with_expected_idl_type_with_leftover_bytes(
                    rest,
                    idl_type,
                    idl_types,
                    generic_args,
                    idl_type_def_generics,
                )
            }
        }
        IdlType::Vec(idl_type) => {
            let (vec_len, rest) = data.split_at_checked(4).ok_or(bytes_err("vec length"))?;
            let vec_len = u32::from_le_bytes(
                <[u8; 4]>::try_from(vec_len).map_err(|e| err("vec length", &e))?,
            ) as usize;

            let mut vec_values = Vec::with_capacity(vec_len);
            let mut remaining_data = rest;

            for _ in 0..vec_len {
                let (value, rest) =
                    parse_bytes_to_value_with_expected_idl_type_with_leftover_bytes(
                        remaining_data,
                        idl_type,
                        idl_types,
                        generic_args,
                        idl_type_def_generics,
                    )?;
                vec_values.push(value);
                remaining_data = rest;
            }

            Ok((Value::array(vec_values), remaining_data))
        }
        IdlType::Array(idl_type, idl_array_len) => match idl_array_len {
            anchor_lang_idl::types::IdlArrayLen::Generic(_) => todo!(),
            anchor_lang_idl::types::IdlArrayLen::Value(len) => {
                let mut vec_values = Vec::with_capacity(*len);
                let mut remaining_data = data;
                for _ in 0..*len {
                    let (value, rest) =
                        parse_bytes_to_value_with_expected_idl_type_with_leftover_bytes(
                            remaining_data,
                            idl_type,
                            idl_types,
                            generic_args,
                            idl_type_def_generics,
                        )?;
                    vec_values.push(value);
                    remaining_data = rest;
                }
                Ok((Value::array(vec_values), remaining_data))
            }
        },
        IdlType::Defined { name, generics } => {
            let Some(matching_idl_type) = idl_types.iter().find(|t| t.name == *name) else {
                return Err(err(name, &"not found in IDL types"));
            };

            let (value, rest) =
                parse_bytes_to_value_with_expected_idl_type_def_ty_with_leftover_bytes(
                    data,
                    &matching_idl_type.ty,
                    idl_types,
                    generics,
                    &matching_idl_type.generics,
                )?;

            Ok((value, rest))
        }
        IdlType::Generic(generic_name) => {
            let index_of_matching_generic = idl_type_def_generics
                .iter()
                .position(|g| match g {
                    IdlTypeDefGeneric::Type { name } => name.eq(generic_name),
                    IdlTypeDefGeneric::Const { name, .. } => name.eq(generic_name),
                })
                .ok_or(err(
                    "generic",
                    &format!("unable to find generic type '{}'", generic_name),
                ))?;

            let matching_idl_type_def_generic =
                idl_type_def_generics.get(index_of_matching_generic).unwrap();

            let generic_arg = generic_args.get(index_of_matching_generic).ok_or(err(
                "generic",
                &format!("unable to find generic argument for '{}'", generic_name),
            ))?;

            match generic_arg {
                IdlGenericArg::Type { ty } => {
                    parse_bytes_to_value_with_expected_idl_type_with_leftover_bytes(
                        data,
                        ty,
                        idl_types,
                        generic_args,
                        idl_type_def_generics,
                    )
                }
                IdlGenericArg::Const { value: _value } => {
                    let IdlTypeDefGeneric::Const { ty, .. } = matching_idl_type_def_generic else {
                        return Err(err(
                            "generic const",
                            &format!(
                                "expected const generic, found type generic '{}'",
                                generic_name
                            ),
                        ));
                    };

                    let _idl_type = IdlType::from_str(ty).map_err(|e| {
                        err(
                            "generic const",
                            &format!(
                                "unknown IDL type from generic const '{}': {}",
                                generic_name, e
                            ),
                        )
                    })?;

                    // To handle a const generic, we need to be able to convert a String and an expected IdlType to a Value
                    // This is possible, but is a lot of code for an edge case
                    return Err(format!("Generic consts are not supported yet"));
                }
            }
        }
        _ => Err(format!("unsupported IDL type: {:?}", expected_type)),
    }
}

/// Flattens nested account items into a flat ordered list
fn flatten_accounts(accounts: &[IdlInstructionAccountItem]) -> Vec<String> {
    let mut result = Vec::new();
    for item in accounts {
        match item {
            IdlInstructionAccountItem::Single(account) => {
                result.push(account.name.clone());
            }
            IdlInstructionAccountItem::Composite(nested) => {
                // Prepend the parent name as a prefix if desired
                result.extend(flatten_accounts(&nested.accounts));
            }
        }
    }
    result
}

/// Given a message account key list and a CompiledInstruction, return a mapping from IDL account names to pubkeys
pub fn match_idl_accounts(
    idl_instruction: &IdlInstruction,
    instruction_account_indices: &[u8],
    message_account_keys: &[Pubkey],
) -> Vec<(String, Pubkey, usize)> {
    let flat_idl_account_names = flatten_accounts(&idl_instruction.accounts);

    flat_idl_account_names
        .into_iter()
        .zip(instruction_account_indices.iter())
        .map(|(name, &index)| (name, message_account_keys[index as usize], index as usize))
        .collect()
}

pub fn borsh_encode_value_to_idl_type(
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

                    // Handle two enum formats:
                    // 1. {"variant": "VariantName", "value": ...} (explicit format)
                    // 2. {"VariantName": ...} (decoded format from parse_bytes_to_value)
                    let (enum_variant, enum_variant_value) = if let Some(variant_field) =
                        enum_value.get("variant")
                    {
                        // Format 1: explicit variant field
                        let variant_name = variant_field.as_string().ok_or_else(|| {
                            format!(
                                "unable to encode value ({}) as borsh enum: expected variant field to be a string",
                                value.to_string(),
                            )
                        })?;
                        let variant_value = enum_value.get("value").ok_or_else(|| {
                            format!(
                                "unable to encode value ({}) as borsh enum: missing 'value' field",
                                value.to_string(),
                            )
                        })?;
                        (variant_name, variant_value)
                    } else {
                        // Format 2: variant name as object key
                        if enum_value.len() != 1 {
                            return Err(format!(
                                "unable to encode value ({}) as borsh enum: expected exactly one field (the variant name)",
                                value.to_string(),
                            ));
                        }
                        let (variant_name, variant_value) =
                            enum_value.iter().next().ok_or_else(|| {
                                format!(
                                    "unable to encode value ({}) as borsh enum: empty object",
                                    value.to_string(),
                                )
                            })?;
                        (variant_name.as_str(), variant_value)
                    };

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

fn borsh_encode_bytes_to_idl_type(
    bytes: &Vec<u8>,
    idl_type: &IdlType,
    idl_types: &Vec<IdlTypeDef>,
) -> Result<Vec<u8>, String> {
    match idl_type {
        // Primitive numeric types - deserialize from bytes
        IdlType::U8 => {
            if bytes.len() != 1 {
                return Err(format!("expected 1 byte for u8, found {}", bytes.len()));
            }
            Ok(bytes.clone())
        }
        IdlType::U16 => {
            if bytes.len() != 2 {
                return Err(format!("expected 2 bytes for u16, found {}", bytes.len()));
            }
            Ok(bytes.clone())
        }
        IdlType::U32 => {
            if bytes.len() != 4 {
                return Err(format!("expected 4 bytes for u32, found {}", bytes.len()));
            }
            Ok(bytes.clone())
        }
        IdlType::U64 => {
            if bytes.len() != 8 {
                return Err(format!("expected 8 bytes for u64, found {}", bytes.len()));
            }
            Ok(bytes.clone())
        }
        IdlType::U128 => {
            if bytes.len() != 16 {
                return Err(format!("expected 16 bytes for u128, found {}", bytes.len()));
            }
            Ok(bytes.clone())
        }
        IdlType::U256 => {
            if bytes.len() != 32 {
                return Err(format!("expected 32 bytes for u256, found {}", bytes.len()));
            }
            Ok(bytes.clone())
        }
        IdlType::I8 => {
            if bytes.len() != 1 {
                return Err(format!("expected 1 byte for i8, found {}", bytes.len()));
            }
            Ok(bytes.clone())
        }
        IdlType::I16 => {
            if bytes.len() != 2 {
                return Err(format!("expected 2 bytes for i16, found {}", bytes.len()));
            }
            Ok(bytes.clone())
        }
        IdlType::I32 => {
            if bytes.len() != 4 {
                return Err(format!("expected 4 bytes for i32, found {}", bytes.len()));
            }
            Ok(bytes.clone())
        }
        IdlType::I64 => {
            if bytes.len() != 8 {
                return Err(format!("expected 8 bytes for i64, found {}", bytes.len()));
            }
            Ok(bytes.clone())
        }
        IdlType::I128 => {
            if bytes.len() != 16 {
                return Err(format!("expected 16 bytes for i128, found {}", bytes.len()));
            }
            Ok(bytes.clone())
        }
        IdlType::I256 => {
            if bytes.len() != 32 {
                return Err(format!("expected 32 bytes for i256, found {}", bytes.len()));
            }
            Ok(bytes.clone())
        }
        IdlType::F32 => {
            if bytes.len() != 4 {
                return Err(format!("expected 4 bytes for f32, found {}", bytes.len()));
            }
            Ok(bytes.clone())
        }
        IdlType::F64 => {
            if bytes.len() != 8 {
                return Err(format!("expected 8 bytes for f64, found {}", bytes.len()));
            }
            Ok(bytes.clone())
        }
        IdlType::Bool => {
            if bytes.len() != 1 {
                return Err(format!("expected 1 byte for bool, found {}", bytes.len()));
            }
            Ok(bytes.clone())
        }
        IdlType::Pubkey => {
            if bytes.len() != 32 {
                return Err(format!("expected 32 bytes for Pubkey, found {}", bytes.len()));
            }
            Ok(bytes.clone())
        }
        IdlType::String => {
            // Assume bytes are UTF-8 encoded string, encode as borsh string
            let s = std::str::from_utf8(bytes)
                .map_err(|e| format!("invalid UTF-8 for string: {}", e))?;
            borsh::to_vec(&s).map_err(|e| format!("failed to encode string: {}", e))
        }
        IdlType::Bytes => {
            // Return raw bytes as-is
            Ok(bytes.clone())
        }
        IdlType::Vec(inner_type) => {
            // Encode as vector - each element from inner type
            match &**inner_type {
                IdlType::U8 => {
                    // Vec<u8> - encode as borsh vector
                    borsh::to_vec(bytes).map_err(|e| format!("failed to encode Vec<u8>: {}", e))
                }
                _ => {
                    // For other types, try to split bytes and encode each element
                    Err(format!(
                        "cannot convert raw bytes to Vec<{:?}>; bytes can only be directly converted to Vec<u8>",
                        inner_type
                    ))
                }
            }
        }
        IdlType::Array(inner_type, array_len) => {
            let expected_len = match array_len {
                IdlArrayLen::Value(len) => *len,
                IdlArrayLen::Generic(_) => {
                    return Err(format!("cannot determine array length from generic"));
                }
            };

            match &**inner_type {
                IdlType::U8 => {
                    // [u8; N] - validate length and return bytes
                    if bytes.len() != expected_len {
                        return Err(format!(
                            "expected {} bytes for array, found {}",
                            expected_len,
                            bytes.len()
                        ));
                    }
                    Ok(bytes.clone())
                }
                _ => {
                    // For other types, would need to know element size
                    Err(format!(
                        "cannot convert raw bytes to [{:?}; {}]; bytes can only be directly converted to [u8; N]",
                        inner_type, expected_len
                    ))
                }
            }
        }
        IdlType::Option(inner_type) => {
            // If bytes are empty, encode as None
            if bytes.is_empty() {
                borsh::to_vec(&None::<u8>).map_err(|e| format!("failed to encode None: {}", e))
            } else {
                // Otherwise encode as Some with inner bytes
                let inner_encoded = borsh_encode_bytes_to_idl_type(bytes, inner_type, idl_types)?;
                borsh::to_vec(&Some(inner_encoded))
                    .map_err(|e| format!("failed to encode Option: {}", e))
            }
        }
        IdlType::Defined { name, .. } => {
            // For defined types, we can't directly convert from bytes without knowing the structure
            Err(format!(
                "cannot convert raw bytes to defined type '{}'; use structured value instead",
                name
            ))
        }
        IdlType::Generic(name) => Err(format!(
            "cannot convert raw bytes to generic type '{}'; type must be resolved first",
            name
        )),
        t => Err(format!("IDL type {:?} is not yet supported for bytes encoding", t)),
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
