pub mod convert_idl;

use std::str::FromStr;

use crate::typing::anchor as anchor_lang_idl;
use crate::typing::shank as shank_idl;
use anchor_lang_idl::types::{Idl as AnchorIdl, IdlInstruction};
use convert_idl::classic_idl_to_anchor_idl;
use log::debug;
use shank_idl::idl::Idl as ShankIdl;
use solana_pubkey::Pubkey;
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::{helpers::fs::FileLocation, indexmap::IndexMap, types::types::Value};
use txtx_addon_network_svm_types::subgraph::idl::anchor::borsh_encode_value_to_idl_type;
use txtx_addon_network_svm_types::subgraph::idl::shank::{
    borsh_encode_value_to_shank_idl_type, extract_shank_instruction_arg_type, extract_shank_types,
};
use txtx_addon_network_svm_types::subgraph::idl::IdlKind;

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

    pub fn idl(&self) -> &IdlKind {
        &self.idl
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
                let instruction =
                    idl.instructions.iter().find(|i| i.name == instruction_name).ok_or_else(
                        || diagnosed_error!("instruction '{instruction_name}' not found in IDL"),
                    )?;
                Ok(instruction.discriminator.clone())
            }
            IdlKind::Shank(idl) => {
                let instruction =
                    idl.instructions.iter().find(|i| i.name == instruction_name).ok_or_else(
                        || diagnosed_error!("instruction '{instruction_name}' not found in IDL"),
                    )?;
                // Shank uses a single u8 discriminant
                Ok(vec![instruction.discriminant.value])
            }
        }
    }

    pub fn get_instruction(&self, instruction_name: &str) -> Result<&IdlInstruction, Diagnostic> {
        match &self.idl {
            IdlKind::Anchor(idl) => {
                idl.instructions.iter().find(|i| i.name == instruction_name).ok_or_else(|| {
                    diagnosed_error!("instruction '{instruction_name}' not found in IDL")
                })
            }
            IdlKind::Shank(_) => {
                Err(diagnosed_error!("get_instruction is not supported for Shank IDL"))
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
                let instruction =
                    idl.instructions.iter().find(|i| i.name == instruction_name).ok_or_else(
                        || diagnosed_error!("instruction '{instruction_name}' not found in IDL"),
                    )?;
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
                        borsh_encode_value_to_idl_type(arg, &idl_arg.ty, &idl_types, None)
                            .map_err(|e| {
                                diagnosed_error!(
                                    "error in argument at position {}: {}",
                                    user_arg_idx + 1,
                                    e
                                )
                            })?;
                    encoded_args.insert(idl_arg.name.clone(), encoded_arg);
                }
                Ok(encoded_args)
            }
            IdlKind::Shank(idl) => {
                let instruction =
                    idl.instructions.iter().find(|i| i.name == instruction_name).ok_or_else(
                        || diagnosed_error!("instruction '{instruction_name}' not found in IDL"),
                    )?;
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
                    let arg_type =
                        extract_shank_instruction_arg_type(idl, instruction_name, user_arg_idx)
                            .map_err(|e| diagnosed_error!("failed to extract arg type: {}", e))?;
                    let encoded_arg = borsh_encode_value_to_shank_idl_type(
                        arg, &arg_type, &idl_types,
                    )
                    .map_err(|e| {
                        diagnosed_error!(
                            "error in argument at position {}: {}",
                            user_arg_idx + 1,
                            e
                        )
                    })?;
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
                let instruction =
                    idl.instructions.iter().find(|i| i.name == instruction_name).ok_or_else(
                        || diagnosed_error!("instruction '{instruction_name}' not found in IDL"),
                    )?;
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
                        borsh_encode_value_to_idl_type(arg, &idl_arg.ty, &idl_types, None)
                            .map_err(|e| {
                                diagnosed_error!(
                                    "error in argument at position {}: {}",
                                    user_arg_idx + 1,
                                    e
                                )
                            })?;
                    encoded_args.append(&mut encoded_arg);
                }
                Ok(encoded_args)
            }
            IdlKind::Shank(idl) => {
                let instruction =
                    idl.instructions.iter().find(|i| i.name == instruction_name).ok_or_else(
                        || diagnosed_error!("instruction '{instruction_name}' not found in IDL"),
                    )?;
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
                    let arg_type =
                        extract_shank_instruction_arg_type(idl, instruction_name, user_arg_idx)
                            .map_err(|e| diagnosed_error!("failed to extract arg type: {}", e))?;
                    let mut encoded_arg = borsh_encode_value_to_shank_idl_type(
                        arg, &arg_type, &idl_types,
                    )
                    .map_err(|e| {
                        diagnosed_error!(
                            "error in argument at position {}: {}",
                            user_arg_idx + 1,
                            e
                        )
                    })?;
                    encoded_args.append(&mut encoded_arg);
                }
                Ok(encoded_args)
            }
        }
    }
}

fn parse_idl_string(idl_str: &str) -> Result<IdlKind, Diagnostic> {
    // Try parsing as Anchor IDL first (modern format)
    match serde_json::from_str::<AnchorIdl>(idl_str) {
        Ok(anchor_idl) => return Ok(IdlKind::Anchor(anchor_idl)),
        Err(e) => {
            debug!("Failed to parse as Anchor IDL: {}", e);
        }
    }

    // Try parsing as Shank IDL
    match serde_json::from_str::<ShankIdl>(idl_str) {
        Ok(shank_idl) => return Ok(IdlKind::Shank(shank_idl)),
        Err(e) => {
            debug!("Failed to parse as Shank IDL: {}", e);
        }
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
