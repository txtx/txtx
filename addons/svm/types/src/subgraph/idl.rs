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
        IdlTypeDefTy::Type { alias } => todo!(),
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

/// Parses a byte array into a JSON value based on the expected IDL type.
/// **Not used internally, but is used by crate consumers.**
pub fn parse_bytes_to_value_with_expected_idl_type_def_ty(
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
                                        Ok(SvmValue::u8(v[0]))
                                    }
                                    IdlType::U16 => {
                                        let (v, rest) = data.split_at(2);
                                        data = rest;
                                        Ok(SvmValue::u16(
                                            u16::from_le_bytes(<[u8; 2]>::try_from(v).map_err(
                                                |e| format!("unable to decode u16: {e}"),
                                            )?)
                                            .into(),
                                        ))
                                    }
                                    IdlType::U32 => {
                                        let (v, rest) = data.split_at(4);
                                        data = rest;
                                        Ok(SvmValue::u32(
                                            u32::from_le_bytes(<[u8; 4]>::try_from(v).map_err(
                                                |e| format!("unable to decode u32: {e}"),
                                            )?)
                                            .into(),
                                        ))
                                    }
                                    IdlType::U64 => {
                                        let (v, rest) = data.split_at(8);
                                        data = rest;
                                        Ok(SvmValue::u64(
                                            u64::from_le_bytes(<[u8; 8]>::try_from(v).map_err(
                                                |e| format!("unable to decode u64: {e}"),
                                            )?)
                                            .into(),
                                        ))
                                    }
                                    IdlType::U128 => {
                                        let (v, rest) = data.split_at(16);
                                        data = rest;
                                        Ok(SvmValue::u128(
                                            u128::from_le_bytes(<[u8; 16]>::try_from(v).map_err(
                                                |e| format!("unable to decode u128: {e}"),
                                            )?)
                                            .try_into()
                                            .map_err(|e| {
                                                format!("unable to convert u128 to i128: {e}")
                                            })?,
                                        ))
                                    }
                                    IdlType::U256 => {
                                        let (v, rest) = data.split_at(32);
                                        data = rest;
                                        Ok(SvmValue::u256(
                                            v.try_into().map_err(|e| {
                                                format!("unable to decode u256: {e}")
                                            })?,
                                        ))
                                    }
                                    IdlType::I8 => {
                                        let (v, rest) = data.split_at(1);
                                        data = rest;
                                        Ok(SvmValue::i8(
                                            i8::try_from(v[0])
                                                .map_err(|e| format!("unable to decode i8: {e}"))?
                                                .into(),
                                        ))
                                    }
                                    IdlType::I16 => {
                                        let (v, rest) = data.split_at(2);
                                        data = rest;
                                        Ok(SvmValue::i16(
                                            i16::from_le_bytes(<[u8; 2]>::try_from(v).map_err(
                                                |e| format!("unable to decode i16: {e}"),
                                            )?)
                                            .into(),
                                        ))
                                    }
                                    IdlType::I32 => {
                                        let (v, rest) = data.split_at(4);
                                        data = rest;
                                        Ok(SvmValue::i32(
                                            i32::from_le_bytes(<[u8; 4]>::try_from(v).map_err(
                                                |e| format!("unable to decode i32: {e}"),
                                            )?)
                                            .into(),
                                        ))
                                    }
                                    IdlType::I64 => {
                                        let (v, rest) = data.split_at(8);
                                        data = rest;
                                        Ok(SvmValue::i64(
                                            i64::from_le_bytes(<[u8; 8]>::try_from(v).map_err(
                                                |e| format!("unable to decode i64: {e}"),
                                            )?)
                                            .into(),
                                        ))
                                    }
                                    IdlType::I128 => {
                                        let (v, rest) = data.split_at(16);
                                        data = rest;
                                        Ok(SvmValue::i128(i128::from_le_bytes(
                                            <[u8; 16]>::try_from(v).map_err(|e| {
                                                format!("unable to decode i128: {e}")
                                            })?,
                                        )))
                                    }
                                    IdlType::I256 => {
                                        let (v, rest) = data.split_at(32);
                                        data = rest;
                                        Ok(SvmValue::i256(
                                            v.try_into().map_err(|e| {
                                                format!("unable to decode i256: {e}")
                                            })?,
                                        ))
                                    }
                                    IdlType::F32 => {
                                        let (v, rest) = data.split_at(4);
                                        data = rest;
                                        Ok(SvmValue::f32(
                                            f32::from_le_bytes(<[u8; 4]>::try_from(v).map_err(
                                                |e| format!("unable to decode f32: {e}"),
                                            )?)
                                            .into(),
                                        ))
                                    }
                                    IdlType::F64 => {
                                        let (v, rest) = data.split_at(8);
                                        data = rest;
                                        Ok(SvmValue::f64(
                                            f64::from_le_bytes(<[u8; 8]>::try_from(v).map_err(
                                                |e| format!("unable to decode f64: {e}"),
                                            )?)
                                            .into(),
                                        ))
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
        IdlTypeDefTy::Type { alias } => parse_bytes_to_value_with_expected_idl_type(data, alias),
    }
}

pub fn parse_bytes_to_value_with_expected_idl_type(
    data: &[u8],
    expected_type: &IdlType,
) -> Result<Value, String> {
    match expected_type {
        IdlType::Bool => {
            let value = borsh::from_slice::<bool>(&data)
                .map_err(|e| format!("unable to decode bool: {e}"))?;
            Ok(Value::bool(value))
        }
        IdlType::U8 => Ok(SvmValue::u8(
            borsh::from_slice::<u8>(&data).map_err(|e| format!("unable to decode u8: {e}"))?,
        )),
        IdlType::U16 => Ok(SvmValue::u16(
            borsh::from_slice::<u16>(&data).map_err(|e| format!("unable to decode u16: {e}"))?,
        )),
        IdlType::U32 => Ok(SvmValue::u32(
            borsh::from_slice::<u32>(&data).map_err(|e| format!("unable to decode u32: {e}"))?,
        )),
        IdlType::U64 => Ok(SvmValue::u64(
            borsh::from_slice::<u64>(&data).map_err(|e| format!("unable to decode u64: {e}"))?,
        )),
        IdlType::U128 => Ok(SvmValue::u128(
            borsh::from_slice::<u128>(&data).map_err(|e| format!("unable to decode u128: {e}"))?,
        )),
        IdlType::U256 => Ok(SvmValue::u256(
            borsh::from_slice::<[u8; 32]>(&data)
                .map_err(|e| format!("unable to decode u256: {e}"))?,
        )),
        IdlType::I8 => Ok(SvmValue::i8(
            borsh::from_slice::<i8>(&data).map_err(|e| format!("unable to decode i8: {e}"))?,
        )),
        IdlType::I16 => Ok(SvmValue::i16(
            borsh::from_slice::<i16>(&data).map_err(|e| format!("unable to decode i16: {e}"))?,
        )),
        IdlType::I32 => Ok(SvmValue::i32(
            borsh::from_slice::<i32>(&data).map_err(|e| format!("unable to decode i32: {e}"))?,
        )),
        IdlType::I64 => Ok(SvmValue::i64(
            borsh::from_slice::<i64>(&data).map_err(|e| format!("unable to decode i64: {e}"))?,
        )),
        IdlType::I128 => Ok(SvmValue::i128(
            borsh::from_slice::<i128>(&data).map_err(|e| format!("unable to decode i128: {e}"))?,
        )),
        IdlType::I256 => Ok(SvmValue::i256(
            borsh::from_slice::<[u8; 32]>(&data)
                .map_err(|e| format!("unable to decode i256: {e}"))?,
        )),
        IdlType::F32 => Ok(SvmValue::f32(
            borsh::from_slice::<f32>(&data).map_err(|e| format!("unable to decode f32: {e}"))?,
        )),
        IdlType::F64 => Ok(SvmValue::f64(
            borsh::from_slice::<f64>(&data).map_err(|e| format!("unable to decode f64: {e}"))?,
        )),
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
        _ => return Err(format!("Unsupported type: {:?}", expected_type)),
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
) -> Vec<(String, Pubkey)> {
    let flat_idl_account_names = flatten_accounts(&idl_instruction.accounts);

    flat_idl_account_names
        .into_iter()
        .zip(instruction_account_indices.iter())
        .map(|(name, &index)| (name, message_account_keys[index as usize]))
        .collect()
}
