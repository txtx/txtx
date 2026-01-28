pub mod convert_idl;

use std::str::FromStr;

use crate::typing::anchor as anchor_lang_idl;
use crate::typing::shank as shank_idl;
use crate::typing::SvmValue;
use anchor_lang_idl::types::{
    Idl as AnchorIdl, IdlArrayLen, IdlDefinedFields, IdlGenericArg, IdlInstruction, IdlType,
    IdlTypeDef, IdlTypeDefGeneric, IdlTypeDefTy,
};
use convert_idl::classic_idl_to_anchor_idl;
use shank_idl::idl::Idl as ShankIdl;
use solana_pubkey::Pubkey;
use std::fmt::Display;
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::{helpers::fs::FileLocation, indexmap::IndexMap, types::types::Value};
use txtx_addon_network_svm_types::subgraph::shank::{
    borsh_encode_value_to_shank_idl_type, extract_shank_instruction_arg_type, extract_shank_types,
};
use txtx_addon_network_svm_types::I256;
use txtx_addon_network_svm_types::U256;

/// Represents the kind of IDL format being used.
#[derive(Debug, Clone)]
pub enum IdlKind {
    /// Anchor IDL format (v0.30+)
    Anchor(AnchorIdl),
    /// Shank IDL format
    Shank(ShankIdl),
}

#[derive(Debug, Clone)]
pub struct IdlRef {
    pub idl: IdlKind,
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

    pub fn from_anchor_idl(idl: AnchorIdl) -> Self {
        Self { idl: IdlKind::Anchor(idl), location: None }
    }

    pub fn from_shank_idl(idl: ShankIdl) -> Self {
        Self { idl: IdlKind::Shank(idl), location: None }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Diagnostic> {
        let idl = parse_idl_bytes(bytes)?;
        Ok(Self { idl, location: None })
    }

    pub fn from_str(idl_str: &str) -> Result<Self, Diagnostic> {
        let idl = parse_idl_string(idl_str)?;
        Ok(Self { idl, location: None })
    }

    /// Returns the IDL kind (Anchor or Shank)
    pub fn kind(&self) -> &IdlKind {
        &self.idl
    }

    /// Returns true if this is an Anchor IDL
    pub fn is_anchor(&self) -> bool {
        matches!(self.idl, IdlKind::Anchor(_))
    }

    /// Returns true if this is a Shank IDL
    pub fn is_shank(&self) -> bool {
        matches!(self.idl, IdlKind::Shank(_))
    }

    /// Returns a reference to the Anchor IDL if this is an Anchor IDL
    pub fn as_anchor(&self) -> Option<&AnchorIdl> {
        match &self.idl {
            IdlKind::Anchor(idl) => Some(idl),
            IdlKind::Shank(_) => None,
        }
    }

    /// Returns a reference to the Shank IDL if this is a Shank IDL
    pub fn as_shank(&self) -> Option<&ShankIdl> {
        match &self.idl {
            IdlKind::Anchor(_) => None,
            IdlKind::Shank(idl) => Some(idl),
        }
    }

    pub fn get_program_pubkey(&self) -> Result<Pubkey, Diagnostic> {
        let address = match &self.idl {
            IdlKind::Anchor(idl) => idl.address.clone(),
            IdlKind::Shank(idl) => idl
                .metadata
                .address
                .clone()
                .ok_or_else(|| diagnosed_error!("Shank IDL is missing program address"))?,
        };
        Pubkey::from_str(&address)
            .map_err(|e| diagnosed_error!("invalid pubkey in program IDL: {e}"))
    }

    pub fn get_discriminator(&self, instruction_name: &str) -> Result<Vec<u8>, Diagnostic> {
        match &self.idl {
            IdlKind::Anchor(idl) => {
                let instruction = idl
                    .instructions
                    .iter()
                    .find(|i| i.name == instruction_name)
                    .ok_or_else(|| {
                        diagnosed_error!("instruction '{instruction_name}' not found in IDL")
                    })?;
                Ok(instruction.discriminator.clone())
            }
            IdlKind::Shank(idl) => {
                let instruction = idl
                    .instructions
                    .iter()
                    .find(|i| i.name == instruction_name)
                    .ok_or_else(|| {
                        diagnosed_error!("instruction '{instruction_name}' not found in IDL")
                    })?;
                // Shank uses a single u8 discriminant
                Ok(vec![instruction.discriminant.value])
            }
        }
    }

    pub fn get_instruction(&self, instruction_name: &str) -> Result<&IdlInstruction, Diagnostic> {
        match &self.idl {
            IdlKind::Anchor(idl) => idl
                .instructions
                .iter()
                .find(|i| i.name == instruction_name)
                .ok_or_else(|| {
                    diagnosed_error!("instruction '{instruction_name}' not found in IDL")
                }),
            IdlKind::Shank(_) => Err(diagnosed_error!(
                "get_instruction is not supported for Shank IDL"
            )),
        }
    }

    pub fn get_types(&self) -> Vec<IdlTypeDef> {
        match &self.idl {
            IdlKind::Anchor(idl) => idl.types.clone(),
            IdlKind::Shank(_) => {
                // Shank types are not directly compatible with Anchor IdlTypeDef
                vec![]
            }
        }
    }

    /// Encodes the arguments for a given instruction into a map of argument names to byte arrays.
    /// Note: This method currently only supports Anchor IDLs.
    pub fn get_encoded_args_map(
        &self,
        instruction_name: &str,
        args: Vec<Value>,
    ) -> Result<IndexMap<String, Vec<u8>>, Diagnostic> {
        match &self.idl {
            IdlKind::Anchor(idl) => {
                let instruction = idl
                    .instructions
                    .iter()
                    .find(|i| i.name == instruction_name)
                    .ok_or_else(|| {
                        diagnosed_error!("instruction '{instruction_name}' not found in IDL")
                    })?;
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

                let idl_types = idl.types.clone();

                let mut encoded_args = IndexMap::new();
                for (user_arg_idx, arg) in args.iter().enumerate() {
                    let idl_arg = instruction.args.get(user_arg_idx).unwrap();
                    let encoded_arg =
                        borsh_encode_value_to_idl_type(arg, &idl_arg.ty, &idl_types, None).map_err(
                            |e| {
                                diagnosed_error!(
                                    "error in argument at position {}: {}",
                                    user_arg_idx + 1,
                                    e
                                )
                            },
                        )?;
                    encoded_args.insert(idl_arg.name.clone(), encoded_arg);
                }
                Ok(encoded_args)
            }
            IdlKind::Shank(idl) => {
                let instruction = idl
                    .instructions
                    .iter()
                    .find(|i| i.name == instruction_name)
                    .ok_or_else(|| {
                        diagnosed_error!("instruction '{instruction_name}' not found in IDL")
                    })?;
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

                // Extract types to our local format for encoding
                let idl_types = extract_shank_types(idl)
                    .map_err(|e| diagnosed_error!("failed to extract IDL types: {}", e))?;

                let mut encoded_args = IndexMap::new();
                for (user_arg_idx, arg) in args.iter().enumerate() {
                    let idl_arg = instruction.args.get(user_arg_idx).unwrap();
                    let arg_type = extract_shank_instruction_arg_type(idl, instruction_name, user_arg_idx)
                        .map_err(|e| diagnosed_error!("failed to extract arg type: {}", e))?;
                    let encoded_arg =
                        borsh_encode_value_to_shank_idl_type(arg, &arg_type, &idl_types).map_err(
                            |e| {
                                diagnosed_error!(
                                    "error in argument at position {}: {}",
                                    user_arg_idx + 1,
                                    e
                                )
                            },
                        )?;
                    encoded_args.insert(idl_arg.name.clone(), encoded_arg);
                }
                Ok(encoded_args)
            }
        }
    }

    /// Encodes the arguments for a given instruction into a flat byte array.
    pub fn get_encoded_args(
        &self,
        instruction_name: &str,
        args: Vec<Value>,
    ) -> Result<Vec<u8>, Diagnostic> {
        match &self.idl {
            IdlKind::Anchor(idl) => {
                let instruction = idl
                    .instructions
                    .iter()
                    .find(|i| i.name == instruction_name)
                    .ok_or_else(|| {
                        diagnosed_error!("instruction '{instruction_name}' not found in IDL")
                    })?;
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

                let idl_types = idl.types.clone();

                let mut encoded_args = vec![];
                for (user_arg_idx, arg) in args.iter().enumerate() {
                    let idl_arg = instruction.args.get(user_arg_idx).unwrap();
                    let mut encoded_arg =
                        borsh_encode_value_to_idl_type(arg, &idl_arg.ty, &idl_types, None).map_err(
                            |e| {
                                diagnosed_error!(
                                    "error in argument at position {}: {}",
                                    user_arg_idx + 1,
                                    e
                                )
                            },
                        )?;
                    encoded_args.append(&mut encoded_arg);
                }
                Ok(encoded_args)
            }
            IdlKind::Shank(idl) => {
                let instruction = idl
                    .instructions
                    .iter()
                    .find(|i| i.name == instruction_name)
                    .ok_or_else(|| {
                        diagnosed_error!("instruction '{instruction_name}' not found in IDL")
                    })?;
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

                // Extract types to our local format for encoding
                let idl_types = extract_shank_types(idl)
                    .map_err(|e| diagnosed_error!("failed to extract IDL types: {}", e))?;

                let mut encoded_args = vec![];
                for (user_arg_idx, arg) in args.iter().enumerate() {
                    let arg_type = extract_shank_instruction_arg_type(idl, instruction_name, user_arg_idx)
                        .map_err(|e| diagnosed_error!("failed to extract arg type: {}", e))?;
                    let mut encoded_arg =
                        borsh_encode_value_to_shank_idl_type(arg, &arg_type, &idl_types).map_err(
                            |e| {
                                diagnosed_error!(
                                    "error in argument at position {}: {}",
                                    user_arg_idx + 1,
                                    e
                                )
                            },
                        )?;
                    encoded_args.append(&mut encoded_arg);
                }
                Ok(encoded_args)
            }
        }
    }
}

fn parse_idl_string(idl_str: &str) -> Result<IdlKind, Diagnostic> {
    // Try parsing as Anchor IDL first (modern format)
    if let Ok(anchor_idl) = serde_json::from_str::<AnchorIdl>(idl_str) {
        return Ok(IdlKind::Anchor(anchor_idl));
    }

    // Try parsing as Shank IDL
    if let Ok(shank_idl) = serde_json::from_str::<ShankIdl>(idl_str) {
        return Ok(IdlKind::Shank(shank_idl));
    }

    // Try parsing as classic/legacy Anchor IDL and convert to modern format
    match serde_json::from_str(idl_str) {
        Ok(classic_idl) => {
            let anchor_idl = classic_idl_to_anchor_idl(classic_idl)?;
            Ok(IdlKind::Anchor(anchor_idl))
        }
        Err(e) => Err(diagnosed_error!("invalid idl: {e}")),
    }
}

fn parse_idl_bytes(idl_bytes: &[u8]) -> Result<IdlKind, Diagnostic> {
    // Try parsing as Anchor IDL first (modern format)
    if let Ok(anchor_idl) = serde_json::from_slice::<AnchorIdl>(idl_bytes) {
        return Ok(IdlKind::Anchor(anchor_idl));
    }

    // Try parsing as Shank IDL
    if let Ok(shank_idl) = serde_json::from_slice::<ShankIdl>(idl_bytes) {
        return Ok(IdlKind::Shank(shank_idl));
    }

    // Try parsing as classic/legacy Anchor IDL and convert to modern format
    match serde_json::from_slice(idl_bytes) {
        Ok(classic_idl) => {
            let anchor_idl = classic_idl_to_anchor_idl(classic_idl)?;
            Ok(IdlKind::Anchor(anchor_idl))
        }
        Err(e) => Err(diagnosed_error!("invalid idl: {e}")),
    }
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
                    let (enum_variant, enum_variant_value) = if let Some(variant_field) = enum_value.get("variant") {
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
                        let (variant_name, variant_value) = enum_value.iter().next().ok_or_else(|| {
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
        IdlType::Generic(name) => {
            Err(format!(
                "cannot convert raw bytes to generic type '{}'; type must be resolved first",
                name
            ))
        }
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
