//! Shank IDL codec functions for converting between Shank IDL types, txtx types, and bytes.
//!
//! Since shank_idl doesn't export its internal types directly, we define local types that mirror
//! the shank_idl structure for use in function signatures.

use serde::{Deserialize, Serialize};
use txtx_addon_kit::{
    indexmap::IndexMap,
    types::types::{ObjectDefinition, ObjectProperty, ObjectType, Type, Value},
};

use crate::{SvmValue, SVM_PUBKEY};
use std::fmt::Display;

// ============================================================================
// Local type definitions that mirror shank_idl internal types
// These are needed because shank_idl doesn't export its internal modules
// ============================================================================

/// Shank IDL type representation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ShankIdlType {
    Bool,
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
    Bytes,
    String,
    #[serde(rename = "publicKey")]
    PublicKey,
    #[serde(untagged)]
    Option(ShankIdlTypeOption),
    #[serde(untagged)]
    FixedSizeOption(ShankIdlTypeFixedSizeOption),
    #[serde(untagged)]
    Vec(ShankIdlTypeVec),
    #[serde(untagged)]
    Array(ShankIdlTypeArray),
    #[serde(untagged)]
    Tuple(ShankIdlTypeTuple),
    #[serde(untagged)]
    Defined(ShankIdlTypeDefined),
    #[serde(untagged)]
    HashMap(ShankIdlTypeHashMap),
    #[serde(untagged)]
    BTreeMap(ShankIdlTypeBTreeMap),
    #[serde(untagged)]
    HashSet(ShankIdlTypeHashSet),
    #[serde(untagged)]
    BTreeSet(ShankIdlTypeBTreeSet),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShankIdlTypeOption {
    pub option: Box<ShankIdlType>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ShankIdlTypeFixedSizeOption {
    pub fixed_size_option: ShankIdlTypeFixedSizeOptionInner,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShankIdlTypeFixedSizeOptionInner {
    pub inner: Box<ShankIdlType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sentinel: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShankIdlTypeVec {
    pub vec: Box<ShankIdlType>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShankIdlTypeArray {
    pub array: (Box<ShankIdlType>, usize),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShankIdlTypeTuple {
    pub tuple: Vec<ShankIdlType>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShankIdlTypeDefined {
    pub defined: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ShankIdlTypeHashMap {
    pub hash_map: (Box<ShankIdlType>, Box<ShankIdlType>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ShankIdlTypeBTreeMap {
    pub b_tree_map: (Box<ShankIdlType>, Box<ShankIdlType>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ShankIdlTypeHashSet {
    pub hash_set: Box<ShankIdlType>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ShankIdlTypeBTreeSet {
    pub b_tree_set: Box<ShankIdlType>,
}

/// Shank IDL field definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShankIdlField {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: ShankIdlType,
}

/// Shank IDL type definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShankIdlTypeDef {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: ShankIdlTypeDefTy,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pod_sentinel: Option<Vec<u8>>,
}

/// Shank IDL type definition type (struct or enum)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ShankIdlTypeDefTy {
    Struct { fields: Vec<ShankIdlField> },
    Enum { variants: Vec<ShankIdlEnumVariant> },
}

/// Shank IDL enum variant
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShankIdlEnumVariant {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<ShankEnumFields>,
}

/// Shank enum fields (named or tuple)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ShankEnumFields {
    Named(Vec<ShankIdlField>),
    Tuple(Vec<ShankIdlType>),
}

/// Shank IDL constant
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShankIdlConst {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: ShankIdlType,
    pub value: String,
}

// ============================================================================
// Helper function to convert from shank_idl::idl::Idl to our local types
// ============================================================================

/// Extracts type definitions from a Shank IDL by serializing and deserializing
pub fn extract_shank_types(idl: &shank_idl::idl::Idl) -> Result<Vec<ShankIdlTypeDef>, String> {
    // Serialize the IDL types to JSON and deserialize to our local types
    let types_json = serde_json::to_string(&idl.types)
        .map_err(|e| format!("failed to serialize IDL types: {}", e))?;
    let mut types: Vec<ShankIdlTypeDef> = serde_json::from_str(&types_json)
        .map_err(|e| format!("failed to deserialize IDL types: {}", e))?;

    // Also include accounts as they can be referenced as types
    let accounts_json = serde_json::to_string(&idl.accounts)
        .map_err(|e| format!("failed to serialize IDL accounts: {}", e))?;
    let accounts: Vec<ShankIdlTypeDef> = serde_json::from_str(&accounts_json)
        .map_err(|e| format!("failed to deserialize IDL accounts: {}", e))?;

    types.extend(accounts);
    Ok(types)
}

/// Extracts instruction argument type from a Shank IDL instruction
pub fn extract_shank_instruction_arg_type(
    idl: &shank_idl::idl::Idl,
    instruction_name: &str,
    arg_index: usize,
) -> Result<ShankIdlType, String> {
    let instruction = idl
        .instructions
        .iter()
        .find(|i| i.name == instruction_name)
        .ok_or_else(|| format!("instruction '{}' not found", instruction_name))?;

    let arg = instruction.args.get(arg_index).ok_or_else(|| {
        format!("argument {} not found in instruction '{}'", arg_index, instruction_name)
    })?;

    let arg_json = serde_json::to_string(&arg.ty)
        .map_err(|e| format!("failed to serialize arg type: {}", e))?;
    serde_json::from_str(&arg_json).map_err(|e| format!("failed to deserialize arg type: {}", e))
}

// ============================================================================
// Discriminator-based account detection
// ============================================================================

use std::collections::BTreeMap;

/// Mapping from discriminator byte value to account type name
#[derive(Debug, Clone)]
pub struct DiscriminatorMapping {
    /// The name of the enum type used as discriminator
    pub enum_type_name: String,
    /// Maps variant index (discriminator byte) to account name
    pub variant_to_account: BTreeMap<u8, String>,
}

/// Find the account_discriminator constant in a Shank IDL.
/// Returns the enum type name it points to, if found.
pub fn find_account_discriminator_const(idl: &shank_idl::idl::Idl) -> Option<String> {
    for constant in &idl.constants {
        if constant.name == "account_discriminator" {
            // The value is typically a quoted string like "\"AccountType\""
            // Strip the quotes if present
            let value = constant.value.trim();
            return Some(if value.starts_with('"') && value.ends_with('"') {
                value[1..value.len() - 1].to_string()
            } else {
                value.to_string()
            });
        }
    }
    None
}

/// Build a mapping from discriminator values to account names.
/// This looks for an enum type with the given name and maps its variant indices
/// to account names that match the variant names.
pub fn build_discriminator_mapping(
    idl: &shank_idl::idl::Idl,
    enum_type_name: &str,
) -> Option<DiscriminatorMapping> {
    let types = extract_shank_types(idl).ok()?;

    // Find the enum type
    let enum_type = types.iter().find(|t| t.name == enum_type_name)?;

    // Make sure it's an enum
    let variants = match &enum_type.ty {
        ShankIdlTypeDefTy::Enum { variants } => variants,
        _ => return None,
    };

    // Build a set of account names for quick lookup
    let account_names: std::collections::HashSet<String> =
        idl.accounts.iter().map(|a| a.name.clone()).collect();

    // Map variant index to account name (if the account exists)
    let mut variant_to_account = BTreeMap::new();
    for (index, variant) in variants.iter().enumerate() {
        // Check if there's an account with the same name as the variant
        if account_names.contains(&variant.name) {
            variant_to_account.insert(index as u8, variant.name.clone());
        }
    }

    if variant_to_account.is_empty() {
        return None;
    }

    Some(DiscriminatorMapping { enum_type_name: enum_type_name.to_string(), variant_to_account })
}

/// Find the byte offset of a discriminator field within a struct's fields.
/// Returns the offset in bytes where the discriminator enum field is located.
fn find_discriminator_field_offset(
    fields: &[ShankIdlField],
    enum_type_name: &str,
    idl_types: &[ShankIdlTypeDef],
) -> Result<usize, String> {
    let mut offset = 0;
    for field in fields {
        // Check if this field is the discriminator enum type
        if let ShankIdlType::Defined(def) = &field.ty {
            if def.defined == enum_type_name {
                return Ok(offset);
            }
        }
        // Add this field's size to the offset
        let field_size = get_shank_type_size_with_types(&field.ty, idl_types)?;
        offset += field_size;
    }
    Err(format!("discriminator field of type '{}' not found in struct", enum_type_name))
}

/// Find the account type by reading the discriminator byte from the data.
///
/// This function locates the discriminator field within each candidate account struct
/// and reads the discriminator byte at the correct offset. This handles cases where
/// the discriminator is not the first field (e.g., `struct Pool { authority: Pubkey, account_type: AccountType }`).
///
/// Returns the account name if a matching discriminator is found.
pub fn find_account_by_discriminator(idl: &shank_idl::idl::Idl, data: &[u8]) -> Option<String> {
    if data.is_empty() {
        return None;
    }

    // Get the discriminator enum name from the IDL constant
    let enum_type_name = find_account_discriminator_const(idl)?;

    // Get all type definitions (needed for offset calculation)
    let types = extract_shank_types(idl).ok()?;

    // Build the discriminator mapping (variant index -> account name)
    let mapping = build_discriminator_mapping(idl, &enum_type_name)?;

    // Build reverse mapping (account name -> expected variant index)
    let account_to_variant: BTreeMap<String, u8> =
        mapping.variant_to_account.iter().map(|(idx, name)| (name.clone(), *idx)).collect();

    // Try each account that participates in the discriminator pattern
    for account in &idl.accounts {
        // Check if this account is in our discriminator mapping
        let Some(&expected_variant) = account_to_variant.get(&account.name) else {
            continue;
        };

        // Find this account's type definition to get its fields
        let Some(account_type) = types.iter().find(|t| t.name == account.name) else {
            continue;
        };

        // Get the struct fields
        let fields = match &account_type.ty {
            ShankIdlTypeDefTy::Struct { fields } => fields,
            _ => continue,
        };

        // Calculate the offset of the discriminator field
        let Ok(offset) = find_discriminator_field_offset(fields, &enum_type_name, &types) else {
            continue;
        };

        // Check if we have enough data to read at this offset
        if offset >= data.len() {
            continue;
        }

        // Read the discriminator byte at the calculated offset
        let discriminator = data[offset];

        // Check if this matches the expected variant for this account
        if discriminator == expected_variant {
            return Some(account.name.clone());
        }
    }

    None
}

// ============================================================================
// Type conversion functions
// ============================================================================

/// Converts a Shank IDL type to a txtx Type.
pub fn shank_idl_type_to_txtx_type(
    idl_type: &ShankIdlType,
    idl_types: &[ShankIdlTypeDef],
    _idl_constants: &[ShankIdlConst],
) -> Result<Type, String> {
    let res = match idl_type {
        ShankIdlType::Bool => Type::bool(),
        ShankIdlType::U8 => Type::addon(crate::SVM_U8),
        ShankIdlType::U16 => Type::addon(crate::SVM_U16),
        ShankIdlType::U32 => Type::addon(crate::SVM_U32),
        ShankIdlType::U64 => Type::addon(crate::SVM_U64),
        ShankIdlType::U128 => Type::addon(crate::SVM_U128),
        ShankIdlType::I8 => Type::addon(crate::SVM_I8),
        ShankIdlType::I16 => Type::addon(crate::SVM_I16),
        ShankIdlType::I32 => Type::addon(crate::SVM_I32),
        ShankIdlType::I64 => Type::addon(crate::SVM_I64),
        ShankIdlType::I128 => Type::addon(crate::SVM_I128),
        ShankIdlType::Bytes => Type::buffer(),
        ShankIdlType::String => Type::string(),
        ShankIdlType::PublicKey => Type::addon(SVM_PUBKEY),
        ShankIdlType::Option(opt) => {
            Type::typed_null(shank_idl_type_to_txtx_type(&opt.option, idl_types, _idl_constants)?)
        }
        ShankIdlType::FixedSizeOption(opt) => Type::typed_null(shank_idl_type_to_txtx_type(
            &opt.fixed_size_option.inner,
            idl_types,
            _idl_constants,
        )?),
        ShankIdlType::Vec(vec) => {
            Type::array(shank_idl_type_to_txtx_type(&vec.vec, idl_types, _idl_constants)?)
        }
        ShankIdlType::Array(arr) => {
            Type::array(shank_idl_type_to_txtx_type(&arr.array.0, idl_types, _idl_constants)?)
        }
        ShankIdlType::Tuple(tuple) => {
            let mut props = vec![];
            for (i, ty) in tuple.tuple.iter().enumerate() {
                let inner_type = shank_idl_type_to_txtx_type(ty, idl_types, _idl_constants)?;
                props.push(ObjectProperty {
                    documentation: "".into(),
                    typing: inner_type,
                    optional: false,
                    tainting: false,
                    name: format!("field_{}", i),
                    internal: false,
                });
            }
            Type::object(ObjectDefinition::tuple(props))
        }
        ShankIdlType::Defined(def) => {
            let Some(matching_type) = idl_types.iter().find(|t| t.name == def.defined) else {
                return Err(format!("unable to find defined type '{}'", def.defined));
            };
            get_expected_type_from_shank_idl_type_def_ty(
                &matching_type.ty,
                idl_types,
                _idl_constants,
            )?
        }
        ShankIdlType::HashMap(map) => {
            let value_type =
                shank_idl_type_to_txtx_type(&map.hash_map.1, idl_types, _idl_constants)?;
            Type::array(value_type)
        }
        ShankIdlType::BTreeMap(map) => {
            let value_type =
                shank_idl_type_to_txtx_type(&map.b_tree_map.1, idl_types, _idl_constants)?;
            Type::array(value_type)
        }
        ShankIdlType::HashSet(set) => {
            Type::array(shank_idl_type_to_txtx_type(&set.hash_set, idl_types, _idl_constants)?)
        }
        ShankIdlType::BTreeSet(set) => {
            Type::array(shank_idl_type_to_txtx_type(&set.b_tree_set, idl_types, _idl_constants)?)
        }
    };
    Ok(res)
}

/// Converts a Shank IDL type definition to a txtx Type.
pub fn get_expected_type_from_shank_idl_type_def_ty(
    idl_type_def_ty: &ShankIdlTypeDefTy,
    idl_types: &[ShankIdlTypeDef],
    idl_constants: &[ShankIdlConst],
) -> Result<Type, String> {
    let ty = match idl_type_def_ty {
        ShankIdlTypeDefTy::Struct { fields } => {
            let mut props = vec![];
            for field in fields {
                let field_type = shank_idl_type_to_txtx_type(&field.ty, idl_types, idl_constants)
                    .map_err(|e| {
                    format!("could not determine expected type for field '{}': {e}", field.name)
                })?;
                props.push(ObjectProperty {
                    documentation: "".into(),
                    typing: field_type,
                    optional: false,
                    tainting: false,
                    name: field.name.clone(),
                    internal: false,
                });
            }
            Type::object(ObjectDefinition::strict(props))
        }
        ShankIdlTypeDefTy::Enum { variants } => {
            let mut props = vec![];
            for variant in variants {
                let variant_type = if let Some(ref fields) = variant.fields {
                    get_expected_type_from_shank_enum_fields(fields, idl_types, idl_constants)?
                } else {
                    Type::null()
                };
                props.push(ObjectProperty {
                    documentation: "".into(),
                    typing: variant_type,
                    optional: false,
                    tainting: false,
                    name: variant.name.clone(),
                    internal: false,
                });
            }
            Type::object(ObjectDefinition::enum_type(props))
        }
    };
    Ok(ty)
}

fn get_expected_type_from_shank_enum_fields(
    fields: &ShankEnumFields,
    idl_types: &[ShankIdlTypeDef],
    idl_constants: &[ShankIdlConst],
) -> Result<Type, String> {
    match fields {
        ShankEnumFields::Named(idl_fields) => {
            let mut props = vec![];
            for field in idl_fields {
                let field_type = shank_idl_type_to_txtx_type(&field.ty, idl_types, idl_constants)?;
                props.push(ObjectProperty {
                    documentation: "".into(),
                    typing: field_type,
                    optional: false,
                    tainting: false,
                    name: field.name.clone(),
                    internal: false,
                });
            }
            Ok(Type::object(ObjectDefinition::strict(props)))
        }
        ShankEnumFields::Tuple(types) => {
            let mut props = vec![];
            for (i, ty) in types.iter().enumerate() {
                let inner_type = shank_idl_type_to_txtx_type(ty, idl_types, idl_constants)?;
                props.push(ObjectProperty {
                    documentation: "".into(),
                    typing: inner_type,
                    optional: false,
                    tainting: false,
                    name: format!("field_{}", i),
                    internal: false,
                });
            }
            Ok(Type::object(ObjectDefinition::tuple(props)))
        }
    }
}

// ============================================================================
// Byte parsing functions (bytes -> Value)
// ============================================================================

/// Parses bytes to a Value using a Shank IDL type definition, consuming all bytes.
pub fn parse_bytes_to_value_with_shank_idl_type_def_ty(
    data: &[u8],
    expected_type: &ShankIdlTypeDefTy,
    idl_types: &[ShankIdlTypeDef],
) -> Result<Value, String> {
    let (value, rest) = parse_bytes_to_value_with_shank_idl_type_def_ty_with_leftover_bytes(
        data,
        expected_type,
        idl_types,
    )?;
    if !rest.is_empty() && rest.iter().any(|&byte| byte != 0) {
        return Err(format!(
            "expected no leftover bytes after parsing type, but found {} bytes of non-zero data",
            rest.len()
        ));
    }
    Ok(value)
}

/// Parses bytes to a Value using a Shank IDL type definition, returning leftover bytes.
pub fn parse_bytes_to_value_with_shank_idl_type_def_ty_with_leftover_bytes<'a>(
    data: &'a [u8],
    expected_type: &ShankIdlTypeDefTy,
    idl_types: &[ShankIdlTypeDef],
) -> Result<(Value, &'a [u8]), String> {
    match expected_type {
        ShankIdlTypeDefTy::Struct { fields } => {
            parse_bytes_to_shank_struct_with_leftover_bytes(data, fields, idl_types)
        }
        ShankIdlTypeDefTy::Enum { variants } => {
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
            let variant = &variants[variant_index];
            let (value, rest) =
                parse_bytes_to_shank_enum_variant_with_leftover_bytes(rest, variant, idl_types)?;
            Ok((ObjectType::from([(&variant.name, value)]).to_value(), rest))
        }
    }
}

fn parse_bytes_to_shank_struct_with_leftover_bytes<'a>(
    data: &'a [u8],
    fields: &[ShankIdlField],
    idl_types: &[ShankIdlTypeDef],
) -> Result<(Value, &'a [u8]), String> {
    let mut map = IndexMap::new();
    let mut remaining_data = data;
    for field in fields {
        let (value, rest) = parse_bytes_to_value_with_shank_idl_type_with_leftover_bytes(
            remaining_data,
            &field.ty,
            idl_types,
        )?;
        remaining_data = rest;
        map.insert(field.name.clone(), value);
    }
    Ok((ObjectType::from_map(map).to_value(), remaining_data))
}

fn parse_bytes_to_shank_enum_variant_with_leftover_bytes<'a>(
    data: &'a [u8],
    variant: &ShankIdlEnumVariant,
    idl_types: &[ShankIdlTypeDef],
) -> Result<(Value, &'a [u8]), String> {
    match &variant.fields {
        Some(ShankEnumFields::Named(fields)) => {
            let mut map = IndexMap::new();
            let mut remaining_data = data;
            for field in fields {
                let (value, rest) = parse_bytes_to_value_with_shank_idl_type_with_leftover_bytes(
                    remaining_data,
                    &field.ty,
                    idl_types,
                )?;
                remaining_data = rest;
                map.insert(field.name.clone(), value);
            }
            Ok((ObjectType::from_map(map).to_value(), remaining_data))
        }
        Some(ShankEnumFields::Tuple(types)) => {
            let mut values = Vec::with_capacity(types.len());
            let mut remaining_data = data;
            for ty in types {
                let (value, rest) = parse_bytes_to_value_with_shank_idl_type_with_leftover_bytes(
                    remaining_data,
                    ty,
                    idl_types,
                )?;
                remaining_data = rest;
                values.push(value);
            }
            Ok((Value::array(values), remaining_data))
        }
        None => Ok((Value::null(), data)),
    }
}

/// Parses bytes to a Value using a Shank IDL type, returning leftover bytes.
pub fn parse_bytes_to_value_with_shank_idl_type_with_leftover_bytes<'a>(
    data: &'a [u8],
    expected_type: &ShankIdlType,
    idl_types: &[ShankIdlTypeDef],
) -> Result<(Value, &'a [u8]), String> {
    let err = |ty: &str, e: &dyn Display| format!("unable to decode {ty}: {e}");
    let bytes_err = |ty: &str| err(ty, &"not enough bytes");

    match expected_type {
        ShankIdlType::U8 => {
            let (v, rest) = data.split_at_checked(1).ok_or(bytes_err("u8"))?;
            Ok((SvmValue::u8(v[0]), rest))
        }
        ShankIdlType::U16 => {
            let (v, rest) = data.split_at_checked(2).ok_or(bytes_err("u16"))?;
            Ok((
                SvmValue::u16(u16::from_le_bytes(
                    <[u8; 2]>::try_from(v).map_err(|e| err("u16", &e))?,
                )),
                rest,
            ))
        }
        ShankIdlType::U32 => {
            let (v, rest) = data.split_at_checked(4).ok_or(bytes_err("u32"))?;
            Ok((
                SvmValue::u32(u32::from_le_bytes(
                    <[u8; 4]>::try_from(v).map_err(|e| err("u32", &e))?,
                )),
                rest,
            ))
        }
        ShankIdlType::U64 => {
            let (v, rest) = data.split_at_checked(8).ok_or(bytes_err("u64"))?;
            Ok((
                SvmValue::u64(u64::from_le_bytes(
                    <[u8; 8]>::try_from(v).map_err(|e| err("u64", &e))?,
                )),
                rest,
            ))
        }
        ShankIdlType::U128 => {
            let (v, rest) = data.split_at_checked(16).ok_or(bytes_err("u128"))?;
            Ok((
                SvmValue::u128(u128::from_le_bytes(
                    <[u8; 16]>::try_from(v).map_err(|e| err("u128", &e))?,
                )),
                rest,
            ))
        }
        ShankIdlType::I8 => {
            let (v, rest) = data.split_at_checked(1).ok_or(bytes_err("i8"))?;
            Ok((SvmValue::i8(i8::from_le_bytes([v[0]])), rest))
        }
        ShankIdlType::I16 => {
            let (v, rest) = data.split_at_checked(2).ok_or(bytes_err("i16"))?;
            Ok((
                SvmValue::i16(i16::from_le_bytes(
                    <[u8; 2]>::try_from(v).map_err(|e| err("i16", &e))?,
                )),
                rest,
            ))
        }
        ShankIdlType::I32 => {
            let (v, rest) = data.split_at_checked(4).ok_or(bytes_err("i32"))?;
            Ok((
                SvmValue::i32(i32::from_le_bytes(
                    <[u8; 4]>::try_from(v).map_err(|e| err("i32", &e))?,
                )),
                rest,
            ))
        }
        ShankIdlType::I64 => {
            let (v, rest) = data.split_at_checked(8).ok_or(bytes_err("i64"))?;
            Ok((
                SvmValue::i64(i64::from_le_bytes(
                    <[u8; 8]>::try_from(v).map_err(|e| err("i64", &e))?,
                )),
                rest,
            ))
        }
        ShankIdlType::I128 => {
            let (v, rest) = data.split_at_checked(16).ok_or(bytes_err("i128"))?;
            Ok((
                SvmValue::i128(i128::from_le_bytes(
                    <[u8; 16]>::try_from(v).map_err(|e| err("i128", &e))?,
                )),
                rest,
            ))
        }
        ShankIdlType::Bool => {
            let (v, rest) = data.split_at_checked(1).ok_or(bytes_err("bool"))?;
            Ok((Value::bool(v[0] != 0), rest))
        }
        ShankIdlType::PublicKey => {
            let (v, rest) = data.split_at_checked(32).ok_or(bytes_err("pubkey"))?;
            Ok((SvmValue::pubkey(v.to_vec()), rest))
        }
        ShankIdlType::String => {
            let (string_len, rest) = data.split_at_checked(4).ok_or(bytes_err("string length"))?;
            let string_len = u32::from_le_bytes(
                <[u8; 4]>::try_from(string_len).map_err(|e| err("string length", &e))?,
            ) as usize;
            let (string_bytes, rest) =
                rest.split_at_checked(string_len).ok_or(bytes_err("string"))?;
            let string_value = String::from_utf8_lossy(string_bytes).to_string();
            Ok((Value::string(string_value), rest))
        }
        ShankIdlType::Bytes => {
            let (vec_len, rest) = data.split_at_checked(4).ok_or(bytes_err("bytes length"))?;
            let vec_len = u32::from_le_bytes(
                <[u8; 4]>::try_from(vec_len).map_err(|e| err("bytes length", &e))?,
            ) as usize;
            let (vec_bytes, rest) = rest.split_at_checked(vec_len).ok_or(bytes_err("bytes"))?;
            Ok((Value::buffer(vec_bytes.to_vec()), rest))
        }
        ShankIdlType::Option(opt) => {
            let (is_some, rest) = data.split_at_checked(1).ok_or(bytes_err("option"))?;
            if is_some[0] == 0 {
                Ok((Value::null(), rest))
            } else {
                parse_bytes_to_value_with_shank_idl_type_with_leftover_bytes(
                    rest,
                    &opt.option,
                    idl_types,
                )
            }
        }
        ShankIdlType::FixedSizeOption(opt) => {
            let inner_size = get_shank_type_size(&opt.fixed_size_option.inner)?;
            let (inner_bytes, rest) =
                data.split_at_checked(inner_size).ok_or(bytes_err("fixed_size_option"))?;

            if let Some(ref sentinel_bytes) = opt.fixed_size_option.sentinel {
                if inner_bytes == sentinel_bytes.as_slice() {
                    return Ok((Value::null(), rest));
                }
            }

            let (value, _) = parse_bytes_to_value_with_shank_idl_type_with_leftover_bytes(
                inner_bytes,
                &opt.fixed_size_option.inner,
                idl_types,
            )?;
            Ok((value, rest))
        }
        ShankIdlType::Vec(vec) => {
            let (vec_len, rest) = data.split_at_checked(4).ok_or(bytes_err("vec length"))?;
            let vec_len = u32::from_le_bytes(
                <[u8; 4]>::try_from(vec_len).map_err(|e| err("vec length", &e))?,
            ) as usize;

            let mut vec_values = Vec::with_capacity(vec_len);
            let mut remaining_data = rest;

            for _ in 0..vec_len {
                let (value, rest) = parse_bytes_to_value_with_shank_idl_type_with_leftover_bytes(
                    remaining_data,
                    &vec.vec,
                    idl_types,
                )?;
                vec_values.push(value);
                remaining_data = rest;
            }

            Ok((Value::array(vec_values), remaining_data))
        }
        ShankIdlType::Array(arr) => {
            let len = arr.array.1;
            let mut vec_values = Vec::with_capacity(len);
            let mut remaining_data = data;
            for _ in 0..len {
                let (value, rest) = parse_bytes_to_value_with_shank_idl_type_with_leftover_bytes(
                    remaining_data,
                    &arr.array.0,
                    idl_types,
                )?;
                vec_values.push(value);
                remaining_data = rest;
            }
            Ok((Value::array(vec_values), remaining_data))
        }
        ShankIdlType::Tuple(tuple) => {
            let mut values = Vec::with_capacity(tuple.tuple.len());
            let mut remaining_data = data;
            for ty in &tuple.tuple {
                let (value, rest) = parse_bytes_to_value_with_shank_idl_type_with_leftover_bytes(
                    remaining_data,
                    ty,
                    idl_types,
                )?;
                values.push(value);
                remaining_data = rest;
            }
            Ok((Value::array(values), remaining_data))
        }
        ShankIdlType::Defined(def) => {
            let matching_type = idl_types
                .iter()
                .find(|t| t.name == def.defined)
                .ok_or(err(&def.defined, &"not found in IDL types"))?;

            parse_bytes_to_value_with_shank_idl_type_def_ty_with_leftover_bytes(
                data,
                &matching_type.ty,
                idl_types,
            )
        }
        ShankIdlType::HashMap(map) => {
            let (map_len, rest) = data.split_at_checked(4).ok_or(bytes_err("hashmap length"))?;
            let map_len = u32::from_le_bytes(
                <[u8; 4]>::try_from(map_len).map_err(|e| err("hashmap length", &e))?,
            ) as usize;

            let mut result_map = IndexMap::new();
            let mut remaining_data = rest;

            for _ in 0..map_len {
                let (key, rest) = parse_bytes_to_value_with_shank_idl_type_with_leftover_bytes(
                    remaining_data,
                    &map.hash_map.0,
                    idl_types,
                )?;
                let (value, rest) = parse_bytes_to_value_with_shank_idl_type_with_leftover_bytes(
                    rest,
                    &map.hash_map.1,
                    idl_types,
                )?;
                remaining_data = rest;
                result_map.insert(key.to_string(), value);
            }

            Ok((ObjectType::from_map(result_map).to_value(), remaining_data))
        }
        ShankIdlType::BTreeMap(map) => {
            let (map_len, rest) = data.split_at_checked(4).ok_or(bytes_err("btreemap length"))?;
            let map_len = u32::from_le_bytes(
                <[u8; 4]>::try_from(map_len).map_err(|e| err("btreemap length", &e))?,
            ) as usize;

            let mut result_map = IndexMap::new();
            let mut remaining_data = rest;

            for _ in 0..map_len {
                let (key, rest) = parse_bytes_to_value_with_shank_idl_type_with_leftover_bytes(
                    remaining_data,
                    &map.b_tree_map.0,
                    idl_types,
                )?;
                let (value, rest) = parse_bytes_to_value_with_shank_idl_type_with_leftover_bytes(
                    rest,
                    &map.b_tree_map.1,
                    idl_types,
                )?;
                remaining_data = rest;
                result_map.insert(key.to_string(), value);
            }

            Ok((ObjectType::from_map(result_map).to_value(), remaining_data))
        }
        ShankIdlType::HashSet(set) => {
            let (set_len, rest) = data.split_at_checked(4).ok_or(bytes_err("hashset length"))?;
            let set_len = u32::from_le_bytes(
                <[u8; 4]>::try_from(set_len).map_err(|e| err("hashset length", &e))?,
            ) as usize;

            let mut values = Vec::with_capacity(set_len);
            let mut remaining_data = rest;

            for _ in 0..set_len {
                let (value, rest) = parse_bytes_to_value_with_shank_idl_type_with_leftover_bytes(
                    remaining_data,
                    &set.hash_set,
                    idl_types,
                )?;
                values.push(value);
                remaining_data = rest;
            }

            Ok((Value::array(values), remaining_data))
        }
        ShankIdlType::BTreeSet(set) => {
            let (set_len, rest) = data.split_at_checked(4).ok_or(bytes_err("btreeset length"))?;
            let set_len = u32::from_le_bytes(
                <[u8; 4]>::try_from(set_len).map_err(|e| err("btreeset length", &e))?,
            ) as usize;

            let mut values = Vec::with_capacity(set_len);
            let mut remaining_data = rest;

            for _ in 0..set_len {
                let (value, rest) = parse_bytes_to_value_with_shank_idl_type_with_leftover_bytes(
                    remaining_data,
                    &set.b_tree_set,
                    idl_types,
                )?;
                values.push(value);
                remaining_data = rest;
            }

            Ok((Value::array(values), remaining_data))
        }
    }
}

/// Returns the size in bytes of a Shank IDL type (for fixed-size types).
fn get_shank_type_size(ty: &ShankIdlType) -> Result<usize, String> {
    get_shank_type_size_with_types(ty, &[])
}

/// Returns the size in bytes of a Shank IDL type, with access to type definitions
/// for resolving defined types.
fn get_shank_type_size_with_types(
    ty: &ShankIdlType,
    idl_types: &[ShankIdlTypeDef],
) -> Result<usize, String> {
    match ty {
        ShankIdlType::Bool | ShankIdlType::U8 | ShankIdlType::I8 => Ok(1),
        ShankIdlType::U16 | ShankIdlType::I16 => Ok(2),
        ShankIdlType::U32 | ShankIdlType::I32 => Ok(4),
        ShankIdlType::U64 | ShankIdlType::I64 => Ok(8),
        ShankIdlType::U128 | ShankIdlType::I128 => Ok(16),
        ShankIdlType::PublicKey => Ok(32),
        ShankIdlType::Array(arr) => {
            let inner_size = get_shank_type_size_with_types(&arr.array.0, idl_types)?;
            Ok(inner_size * arr.array.1)
        }
        ShankIdlType::Defined(def) => {
            let type_def = idl_types
                .iter()
                .find(|t| t.name == def.defined)
                .ok_or_else(|| format!("cannot determine fixed size for type {:?}", ty))?;
            match &type_def.ty {
                ShankIdlTypeDefTy::Enum { variants } => {
                    // Simple enum without data fields = 1 byte discriminator
                    if variants.iter().all(|v| v.fields.is_none()) {
                        Ok(1)
                    } else {
                        Err(format!(
                            "cannot determine fixed size for enum with data: {}",
                            def.defined
                        ))
                    }
                }
                ShankIdlTypeDefTy::Struct { fields } => {
                    let mut size = 0;
                    for field in fields {
                        size += get_shank_type_size_with_types(&field.ty, idl_types)?;
                    }
                    Ok(size)
                }
            }
        }
        _ => Err(format!("cannot determine fixed size for type {:?}", ty)),
    }
}

// ============================================================================
// Encoding functions (Value -> bytes)
// ============================================================================

/// Encodes a txtx Value to bytes using a Shank IDL type.
pub fn borsh_encode_value_to_shank_idl_type(
    value: &Value,
    idl_type: &ShankIdlType,
    idl_types: &[ShankIdlTypeDef],
) -> Result<Vec<u8>, String> {
    let mismatch_err = |expected: &str| {
        format!("invalid value for idl type: expected {}, found {:?}", expected, value.get_type())
    };
    let encode_err = |expected: &str, e: &dyn Display| {
        format!("unable to encode value ({}) as borsh {}: {}", value.to_string(), expected, e)
    };

    // Handle Buffer and Addon values by encoding their bytes directly
    match value {
        Value::Buffer(bytes) => {
            return borsh_encode_bytes_to_shank_idl_type(bytes, idl_type, idl_types)
        }
        Value::Addon(addon_data) => {
            return borsh_encode_bytes_to_shank_idl_type(&addon_data.bytes, idl_type, idl_types)
        }
        _ => {}
    }

    match idl_type {
        ShankIdlType::Bool => value
            .as_bool()
            .and_then(|b| Some(borsh::to_vec(&b).map_err(|e| encode_err("bool", &e))))
            .transpose()?
            .ok_or(mismatch_err("bool")),
        ShankIdlType::U8 => SvmValue::to_number::<u8>(value)
            .and_then(|num| borsh::to_vec(&num).map_err(|e| e.to_string()))
            .map_err(|e| encode_err("u8", &e)),
        ShankIdlType::U16 => SvmValue::to_number::<u16>(value)
            .and_then(|num| borsh::to_vec(&num).map_err(|e| e.to_string()))
            .map_err(|e| encode_err("u16", &e)),
        ShankIdlType::U32 => SvmValue::to_number::<u32>(value)
            .and_then(|num| borsh::to_vec(&num).map_err(|e| e.to_string()))
            .map_err(|e| encode_err("u32", &e)),
        ShankIdlType::U64 => SvmValue::to_number::<u64>(value)
            .and_then(|num| borsh::to_vec(&num).map_err(|e| e.to_string()))
            .map_err(|e| encode_err("u64", &e)),
        ShankIdlType::U128 => SvmValue::to_number::<u128>(value)
            .and_then(|num| borsh::to_vec(&num).map_err(|e| e.to_string()))
            .map_err(|e| encode_err("u128", &e)),
        ShankIdlType::I8 => SvmValue::to_number::<i8>(value)
            .and_then(|num| borsh::to_vec(&num).map_err(|e| e.to_string()))
            .map_err(|e| encode_err("i8", &e)),
        ShankIdlType::I16 => SvmValue::to_number::<i16>(value)
            .and_then(|num| borsh::to_vec(&num).map_err(|e| e.to_string()))
            .map_err(|e| encode_err("i16", &e)),
        ShankIdlType::I32 => SvmValue::to_number::<i32>(value)
            .and_then(|num| borsh::to_vec(&num).map_err(|e| e.to_string()))
            .map_err(|e| encode_err("i32", &e)),
        ShankIdlType::I64 => SvmValue::to_number::<i64>(value)
            .and_then(|num| borsh::to_vec(&num).map_err(|e| e.to_string()))
            .map_err(|e| encode_err("i64", &e)),
        ShankIdlType::I128 => SvmValue::to_number::<i128>(value)
            .and_then(|num| borsh::to_vec(&num).map_err(|e| e.to_string()))
            .map_err(|e| encode_err("i128", &e)),
        ShankIdlType::Bytes => Ok(value.to_be_bytes().clone()),
        ShankIdlType::String => value
            .as_string()
            .and_then(|s| Some(borsh::to_vec(&s).map_err(|e| encode_err("string", &e))))
            .transpose()?
            .ok_or(mismatch_err("string")),
        ShankIdlType::PublicKey => SvmValue::to_pubkey(value)
            .map_err(|_| mismatch_err("pubkey"))
            .map(|p| borsh::to_vec(&p))?
            .map_err(|e| encode_err("pubkey", &e)),
        ShankIdlType::Option(opt) => {
            if value.as_null().is_some() {
                Ok(vec![0u8]) // None discriminator
            } else {
                let encoded_inner =
                    borsh_encode_value_to_shank_idl_type(value, &opt.option, idl_types)?;
                let mut result = vec![1u8]; // Some discriminator
                result.extend(encoded_inner);
                Ok(result)
            }
        }
        ShankIdlType::FixedSizeOption(opt) => {
            if value.as_null().is_some() {
                if let Some(ref sentinel_bytes) = opt.fixed_size_option.sentinel {
                    Ok(sentinel_bytes.clone())
                } else {
                    let size = get_shank_type_size(&opt.fixed_size_option.inner)?;
                    Ok(vec![0u8; size])
                }
            } else {
                borsh_encode_value_to_shank_idl_type(value, &opt.fixed_size_option.inner, idl_types)
            }
        }
        ShankIdlType::Vec(vec) => match value {
            Value::String(_) => {
                let bytes = value.get_buffer_bytes_result().map_err(|_| mismatch_err("vec"))?;
                borsh_encode_bytes_to_shank_idl_type(&bytes, &vec.vec, idl_types)
            }
            Value::Array(arr) => {
                let mut result = (arr.len() as u32).to_le_bytes().to_vec();
                for v in arr.iter() {
                    let encoded = borsh_encode_value_to_shank_idl_type(v, &vec.vec, idl_types)?;
                    result.extend(encoded);
                }
                Ok(result)
            }
            _ => Err(mismatch_err("vec")),
        },
        ShankIdlType::Array(arr) => {
            let array = value.as_array().ok_or(mismatch_err("array"))?;
            let expected_len = arr.array.1;
            if array.len() != expected_len {
                return Err(format!(
                    "invalid value for idl type: expected array length of {}, found {}",
                    expected_len,
                    array.len()
                ));
            }
            let mut result = vec![];
            for v in array.iter() {
                let encoded = borsh_encode_value_to_shank_idl_type(v, &arr.array.0, idl_types)?;
                result.extend(encoded);
            }
            Ok(result)
        }
        ShankIdlType::Tuple(tuple) => {
            let array = value.as_array().ok_or(mismatch_err("tuple"))?;
            if array.len() != tuple.tuple.len() {
                return Err(format!(
                    "invalid value for idl type: expected tuple length of {}, found {}",
                    tuple.tuple.len(),
                    array.len()
                ));
            }
            let mut result = vec![];
            for (v, ty) in array.iter().zip(tuple.tuple.iter()) {
                let encoded = borsh_encode_value_to_shank_idl_type(v, ty, idl_types)?;
                result.extend(encoded);
            }
            Ok(result)
        }
        ShankIdlType::Defined(def) => {
            let typing = idl_types.iter().find(|t| t.name == def.defined).ok_or_else(|| {
                format!("unable to find type definition for {} in idl", def.defined)
            })?;

            borsh_encode_value_to_shank_idl_type_def_ty(value, &typing.ty, idl_types)
        }
        ShankIdlType::HashMap(map) => {
            let obj = value.as_object().ok_or(mismatch_err("hashmap"))?;
            let mut result = (obj.len() as u32).to_le_bytes().to_vec();
            for (k, v) in obj.iter() {
                let key_value = Value::string(k.clone());
                let encoded_key =
                    borsh_encode_value_to_shank_idl_type(&key_value, &map.hash_map.0, idl_types)?;
                let encoded_value =
                    borsh_encode_value_to_shank_idl_type(v, &map.hash_map.1, idl_types)?;
                result.extend(encoded_key);
                result.extend(encoded_value);
            }
            Ok(result)
        }
        ShankIdlType::BTreeMap(map) => {
            let obj = value.as_object().ok_or(mismatch_err("btreemap"))?;
            let mut result = (obj.len() as u32).to_le_bytes().to_vec();
            for (k, v) in obj.iter() {
                let key_value = Value::string(k.clone());
                let encoded_key =
                    borsh_encode_value_to_shank_idl_type(&key_value, &map.b_tree_map.0, idl_types)?;
                let encoded_value =
                    borsh_encode_value_to_shank_idl_type(v, &map.b_tree_map.1, idl_types)?;
                result.extend(encoded_key);
                result.extend(encoded_value);
            }
            Ok(result)
        }
        ShankIdlType::HashSet(set) => {
            let array = value.as_array().ok_or(mismatch_err("hashset"))?;
            let mut result = (array.len() as u32).to_le_bytes().to_vec();
            for v in array.iter() {
                let encoded = borsh_encode_value_to_shank_idl_type(v, &set.hash_set, idl_types)?;
                result.extend(encoded);
            }
            Ok(result)
        }
        ShankIdlType::BTreeSet(set) => {
            let array = value.as_array().ok_or(mismatch_err("btreeset"))?;
            let mut result = (array.len() as u32).to_le_bytes().to_vec();
            for v in array.iter() {
                let encoded = borsh_encode_value_to_shank_idl_type(v, &set.b_tree_set, idl_types)?;
                result.extend(encoded);
            }
            Ok(result)
        }
    }
}

fn borsh_encode_value_to_shank_idl_type_def_ty(
    value: &Value,
    idl_type_def_ty: &ShankIdlTypeDefTy,
    idl_types: &[ShankIdlTypeDef],
) -> Result<Vec<u8>, String> {
    match idl_type_def_ty {
        ShankIdlTypeDefTy::Struct { fields } => {
            let mut encoded_fields = vec![];
            let user_values_map = value.as_object().ok_or_else(|| {
                format!("expected object for struct, found {:?}", value.get_type())
            })?;

            for field in fields {
                let user_value = user_values_map
                    .get(&field.name)
                    .ok_or_else(|| format!("missing field '{}' in object", field.name))?;
                let encoded =
                    borsh_encode_value_to_shank_idl_type(user_value, &field.ty, idl_types)
                        .map_err(|e| format!("failed to encode field '{}': {}", field.name, e))?;
                encoded_fields.extend(encoded);
            }
            Ok(encoded_fields)
        }
        ShankIdlTypeDefTy::Enum { variants } => {
            let enum_value = value
                .as_object()
                .ok_or_else(|| format!("expected object for enum, found {:?}", value.get_type()))?;

            // Handle two enum formats:
            // 1. {"variant": "VariantName", "value": ...} (explicit format)
            // 2. {"VariantName": ...} (decoded format from parse_bytes_to_value)
            let (enum_variant_name, enum_variant_value) = if let Some(variant_field) =
                enum_value.get("variant")
            {
                let variant_name = variant_field
                    .as_string()
                    .ok_or_else(|| "expected variant field to be a string".to_string())?;
                let variant_value =
                    enum_value.get("value").ok_or_else(|| "missing 'value' field".to_string())?;
                (variant_name.to_string(), variant_value.clone())
            } else {
                if enum_value.len() != 1 {
                    return Err("expected exactly one field (the variant name)".to_string());
                }
                let (variant_name, variant_value) =
                    enum_value.iter().next().ok_or_else(|| "empty object".to_string())?;
                (variant_name.clone(), variant_value.clone())
            };

            let (variant_index, expected_variant) = variants
                .iter()
                .enumerate()
                .find(|(_, v)| v.name == enum_variant_name)
                .ok_or_else(|| format!("unknown variant {}", enum_variant_name))?;

            let mut encoded = vec![variant_index as u8];

            match &expected_variant.fields {
                Some(ShankEnumFields::Named(fields)) => {
                    let user_values_map = enum_variant_value
                        .as_object()
                        .ok_or_else(|| format!("expected object for enum variant fields"))?;
                    for field in fields {
                        let user_value = user_values_map.get(&field.name).ok_or_else(|| {
                            format!("missing field '{}' in enum variant", field.name)
                        })?;
                        let field_encoded =
                            borsh_encode_value_to_shank_idl_type(user_value, &field.ty, idl_types)?;
                        encoded.extend(field_encoded);
                    }
                }
                Some(ShankEnumFields::Tuple(types)) => {
                    let values = enum_variant_value
                        .as_array()
                        .ok_or_else(|| format!("expected array for enum tuple variant"))?;
                    if values.len() != types.len() {
                        return Err(format!(
                            "expected {} tuple fields, found {}",
                            types.len(),
                            values.len()
                        ));
                    }
                    for (v, ty) in values.iter().zip(types.iter()) {
                        let field_encoded = borsh_encode_value_to_shank_idl_type(v, ty, idl_types)?;
                        encoded.extend(field_encoded);
                    }
                }
                None => {
                    // Unit variant, no additional data
                }
            }

            Ok(encoded)
        }
    }
}

fn borsh_encode_bytes_to_shank_idl_type(
    bytes: &[u8],
    idl_type: &ShankIdlType,
    _idl_types: &[ShankIdlTypeDef],
) -> Result<Vec<u8>, String> {
    match idl_type {
        ShankIdlType::U8 => {
            if bytes.len() != 1 {
                return Err(format!("expected 1 byte for u8, found {}", bytes.len()));
            }
            Ok(bytes.to_vec())
        }
        ShankIdlType::U16 => {
            if bytes.len() != 2 {
                return Err(format!("expected 2 bytes for u16, found {}", bytes.len()));
            }
            Ok(bytes.to_vec())
        }
        ShankIdlType::U32 => {
            if bytes.len() != 4 {
                return Err(format!("expected 4 bytes for u32, found {}", bytes.len()));
            }
            Ok(bytes.to_vec())
        }
        ShankIdlType::U64 => {
            if bytes.len() != 8 {
                return Err(format!("expected 8 bytes for u64, found {}", bytes.len()));
            }
            Ok(bytes.to_vec())
        }
        ShankIdlType::U128 => {
            if bytes.len() != 16 {
                return Err(format!("expected 16 bytes for u128, found {}", bytes.len()));
            }
            Ok(bytes.to_vec())
        }
        ShankIdlType::I8 => {
            if bytes.len() != 1 {
                return Err(format!("expected 1 byte for i8, found {}", bytes.len()));
            }
            Ok(bytes.to_vec())
        }
        ShankIdlType::I16 => {
            if bytes.len() != 2 {
                return Err(format!("expected 2 bytes for i16, found {}", bytes.len()));
            }
            Ok(bytes.to_vec())
        }
        ShankIdlType::I32 => {
            if bytes.len() != 4 {
                return Err(format!("expected 4 bytes for i32, found {}", bytes.len()));
            }
            Ok(bytes.to_vec())
        }
        ShankIdlType::I64 => {
            if bytes.len() != 8 {
                return Err(format!("expected 8 bytes for i64, found {}", bytes.len()));
            }
            Ok(bytes.to_vec())
        }
        ShankIdlType::I128 => {
            if bytes.len() != 16 {
                return Err(format!("expected 16 bytes for i128, found {}", bytes.len()));
            }
            Ok(bytes.to_vec())
        }
        ShankIdlType::Bool => {
            if bytes.len() != 1 {
                return Err(format!("expected 1 byte for bool, found {}", bytes.len()));
            }
            Ok(bytes.to_vec())
        }
        ShankIdlType::PublicKey => {
            if bytes.len() != 32 {
                return Err(format!("expected 32 bytes for Pubkey, found {}", bytes.len()));
            }
            Ok(bytes.to_vec())
        }
        ShankIdlType::String => {
            let s = std::str::from_utf8(bytes)
                .map_err(|e| format!("invalid UTF-8 for string: {}", e))?;
            borsh::to_vec(&s).map_err(|e| format!("failed to encode string: {}", e))
        }
        ShankIdlType::Bytes => Ok(bytes.to_vec()),
        ShankIdlType::Vec(vec) => {
            match &*vec.vec {
                ShankIdlType::U8 => {
                    borsh::to_vec(bytes).map_err(|e| format!("failed to encode Vec<u8>: {}", e))
                }
                _ => Err(format!(
                    "cannot convert raw bytes to Vec<{:?}>; bytes can only be directly converted to Vec<u8>",
                    vec.vec
                ))
            }
        }
        ShankIdlType::Array(arr) => {
            match &*arr.array.0 {
                ShankIdlType::U8 => {
                    let expected_len = arr.array.1;
                    if bytes.len() != expected_len {
                        return Err(format!(
                            "expected {} bytes for array, found {}",
                            expected_len,
                            bytes.len()
                        ));
                    }
                    Ok(bytes.to_vec())
                }
                _ => Err(format!(
                    "cannot convert raw bytes to [{:?}; {}]",
                    arr.array.0, arr.array.1
                ))
            }
        }
        _ => Err(format!("cannot convert raw bytes to {:?}", idl_type)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_idl_with_discriminator() -> shank_idl::idl::Idl {
        let idl_json = r#"{
            "name": "test_program",
            "version": "0.1.0",
            "metadata": {
                "origin": "shank",
                "address": "TestProgram11111111111111111111111111111111"
            },
            "constants": [
                {
                    "name": "account_discriminator",
                    "type": "string",
                    "value": "\"AccountType\""
                }
            ],
            "types": [
                {
                    "name": "AccountType",
                    "type": {
                        "kind": "enum",
                        "variants": [
                            { "name": "Account1" },
                            { "name": "Account2" },
                            { "name": "Account3" }
                        ]
                    }
                },
                {
                    "name": "Account1",
                    "type": {
                        "kind": "struct",
                        "fields": [
                            { "name": "account_type", "type": { "defined": "AccountType" } },
                            { "name": "data", "type": "u64" }
                        ]
                    }
                },
                {
                    "name": "Account2",
                    "type": {
                        "kind": "struct",
                        "fields": [
                            { "name": "account_type", "type": { "defined": "AccountType" } },
                            { "name": "value", "type": "u32" }
                        ]
                    }
                }
            ],
            "accounts": [
                {
                    "name": "Account1",
                    "type": {
                        "kind": "struct",
                        "fields": [
                            { "name": "account_type", "type": { "defined": "AccountType" } },
                            { "name": "data", "type": "u64" }
                        ]
                    }
                },
                {
                    "name": "Account2",
                    "type": {
                        "kind": "struct",
                        "fields": [
                            { "name": "account_type", "type": { "defined": "AccountType" } },
                            { "name": "value", "type": "u32" }
                        ]
                    }
                }
            ],
            "instructions": []
        }"#;

        serde_json::from_str(idl_json).expect("Failed to parse test IDL")
    }

    fn create_test_idl_without_discriminator() -> shank_idl::idl::Idl {
        let idl_json = r#"{
            "name": "test_program",
            "version": "0.1.0",
            "metadata": {
                "origin": "shank",
                "address": "TestProgram11111111111111111111111111111111"
            },
            "constants": [],
            "types": [
                {
                    "name": "SimpleAccount",
                    "type": {
                        "kind": "struct",
                        "fields": [
                            { "name": "data", "type": "u64" }
                        ]
                    }
                }
            ],
            "accounts": [
                {
                    "name": "SimpleAccount",
                    "type": {
                        "kind": "struct",
                        "fields": [
                            { "name": "data", "type": "u64" }
                        ]
                    }
                }
            ],
            "instructions": []
        }"#;

        serde_json::from_str(idl_json).expect("Failed to parse test IDL")
    }

    #[test]
    fn test_find_account_discriminator_const() {
        let idl = create_test_idl_with_discriminator();
        let result = find_account_discriminator_const(&idl);
        assert_eq!(result, Some("AccountType".to_string()));
    }

    #[test]
    fn test_find_account_discriminator_const_not_found() {
        let idl = create_test_idl_without_discriminator();
        let result = find_account_discriminator_const(&idl);
        assert_eq!(result, None);
    }

    #[test]
    fn test_build_discriminator_mapping() {
        let idl = create_test_idl_with_discriminator();
        let mapping = build_discriminator_mapping(&idl, "AccountType");

        assert!(mapping.is_some());
        let mapping = mapping.unwrap();

        assert_eq!(mapping.enum_type_name, "AccountType");
        // Variant 0 = Account1, Variant 1 = Account2
        // Account3 is in the enum but has no matching account, so it's not in the mapping
        assert_eq!(mapping.variant_to_account.get(&0), Some(&"Account1".to_string()));
        assert_eq!(mapping.variant_to_account.get(&1), Some(&"Account2".to_string()));
        assert_eq!(mapping.variant_to_account.get(&2), None); // No Account3 account
    }

    #[test]
    fn test_find_account_by_discriminator() {
        let idl = create_test_idl_with_discriminator();

        // Data with discriminator 0 (Account1): [0, <8 bytes of u64>]
        let data_account1 = [0u8, 42, 0, 0, 0, 0, 0, 0, 0]; // discriminator 0, data = 42
        let result = find_account_by_discriminator(&idl, &data_account1);
        assert_eq!(result, Some("Account1".to_string()));

        // Data with discriminator 1 (Account2): [1, <4 bytes of u32>]
        let data_account2 = [1u8, 100, 0, 0, 0]; // discriminator 1, value = 100
        let result = find_account_by_discriminator(&idl, &data_account2);
        assert_eq!(result, Some("Account2".to_string()));

        // Data with discriminator 2 (Account3) - no matching account
        let data_account3 = [2u8, 0, 0, 0, 0];
        let result = find_account_by_discriminator(&idl, &data_account3);
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_account_by_discriminator_no_const() {
        let idl = create_test_idl_without_discriminator();

        let data = [0u8, 42, 0, 0, 0, 0, 0, 0, 0];
        let result = find_account_by_discriminator(&idl, &data);
        assert_eq!(result, None); // No discriminator constant, so returns None
    }

    #[test]
    fn test_find_account_by_discriminator_empty_data() {
        let idl = create_test_idl_with_discriminator();

        let result = find_account_by_discriminator(&idl, &[]);
        assert_eq!(result, None);
    }

    fn create_test_idl_with_discriminator_at_offset() -> shank_idl::idl::Idl {
        // IDL where discriminator is NOT the first field
        // Account structure: { authority: Pubkey (32 bytes), account_type: AccountType (1 byte), ... }
        let idl_json = r#"{
            "name": "test_program",
            "version": "0.1.0",
            "metadata": {
                "origin": "shank",
                "address": "TestProgram11111111111111111111111111111111"
            },
            "constants": [
                {
                    "name": "account_discriminator",
                    "type": "string",
                    "value": "\"AccountType\""
                }
            ],
            "types": [
                {
                    "name": "AccountType",
                    "type": {
                        "kind": "enum",
                        "variants": [
                            { "name": "Pool" },
                            { "name": "Position" }
                        ]
                    }
                },
                {
                    "name": "Pool",
                    "type": {
                        "kind": "struct",
                        "fields": [
                            { "name": "authority", "type": "publicKey" },
                            { "name": "account_type", "type": { "defined": "AccountType" } },
                            { "name": "liquidity", "type": "u64" }
                        ]
                    }
                },
                {
                    "name": "Position",
                    "type": {
                        "kind": "struct",
                        "fields": [
                            { "name": "owner", "type": "publicKey" },
                            { "name": "account_type", "type": { "defined": "AccountType" } },
                            { "name": "amount", "type": "u64" }
                        ]
                    }
                }
            ],
            "accounts": [
                {
                    "name": "Pool",
                    "type": {
                        "kind": "struct",
                        "fields": [
                            { "name": "authority", "type": "publicKey" },
                            { "name": "account_type", "type": { "defined": "AccountType" } },
                            { "name": "liquidity", "type": "u64" }
                        ]
                    }
                },
                {
                    "name": "Position",
                    "type": {
                        "kind": "struct",
                        "fields": [
                            { "name": "owner", "type": "publicKey" },
                            { "name": "account_type", "type": { "defined": "AccountType" } },
                            { "name": "amount", "type": "u64" }
                        ]
                    }
                }
            ],
            "instructions": []
        }"#;

        serde_json::from_str(idl_json).expect("Failed to parse test IDL")
    }

    #[test]
    fn test_find_account_by_discriminator_at_offset() {
        let idl = create_test_idl_with_discriminator_at_offset();

        // Pool account: 32 bytes pubkey + 1 byte discriminator (0) + 8 bytes u64
        // Discriminator is at byte offset 32
        let mut pool_data = vec![0u8; 41]; // 32 + 1 + 8
        pool_data[32] = 0; // AccountType::Pool variant index
        let result = find_account_by_discriminator(&idl, &pool_data);
        assert_eq!(result, Some("Pool".to_string()));

        // Position account: 32 bytes pubkey + 1 byte discriminator (1) + 8 bytes u64
        let mut position_data = vec![0u8; 41];
        position_data[32] = 1; // AccountType::Position variant index
        let result = find_account_by_discriminator(&idl, &position_data);
        assert_eq!(result, Some("Position".to_string()));
    }

    #[test]
    fn test_find_account_by_discriminator_at_offset_wrong_discriminator() {
        let idl = create_test_idl_with_discriminator_at_offset();

        // Data with discriminator at wrong position (first byte instead of byte 32)
        // This should not match because we read from the correct offset
        let mut data = vec![0u8; 41];
        data[0] = 0; // Wrong position - this is the pubkey, not the discriminator
        data[32] = 99; // Invalid discriminator value at correct position
        let result = find_account_by_discriminator(&idl, &data);
        assert_eq!(result, None);
    }
}
