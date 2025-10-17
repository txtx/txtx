use anchor_lang_idl::types::{
    IdlConst, IdlDefinedFields, IdlGenericArg, IdlInstruction, IdlInstructionAccountItem, IdlType,
    IdlTypeDef, IdlTypeDefGeneric, IdlTypeDefTy,
};
use solana_pubkey::Pubkey;
use txtx_addon_kit::{
    indexmap::IndexMap,
    types::types::{ObjectDefinition, ObjectProperty, ObjectType, Type, Value},
};

use crate::{SvmValue, SVM_PUBKEY};
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
        IdlType::Option(idl_type) => idl_type_to_txtx_type(
            *idl_type,
            idl_types,
            idl_constants,
            generic_args,
            idl_type_def_generics,
        )?,
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
