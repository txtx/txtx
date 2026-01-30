use std::path::PathBuf;

use crate::{codec::validate_program_so, typing::anchor::types as anchor_types};

use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use txtx_addon_kit::{
    indexmap::IndexMap,
    types::{
        diagnostics::Diagnostic,
        types::{ObjectType, Value},
    },
};

use crate::typing::SvmValue;

use super::idl::IdlRef;

pub struct AnchorProgramArtifacts {
    /// The IDL of the anchor program, stored for an anchor project at `target/idl/<program_name>.json`.
    pub idl: anchor_types::Idl,
    /// The binary of the anchor program, stored for an anchor project at `target/deploy/<program_name>.so`.
    pub bin: Vec<u8>,
    /// The keypair of the anchor program, stored for an anchor project at `target/deploy/<program_name>-keypair.json`.
    /// Only needed for initial deployments, so optional.
    pub keypair: Option<Keypair>,
    /// The program pubkey of the anchor program.
    pub program_id: Pubkey,
}

impl AnchorProgramArtifacts {
    pub fn new(
        keypair_path: PathBuf,
        idl_path: PathBuf,
        bin_path: PathBuf,
    ) -> Result<Self, String> {
        let idl_bytes: Vec<u8> = std::fs::read(&idl_path).map_err(|e| {
            format!("invalid anchor idl location {}: {}", &idl_path.to_str().unwrap_or(""), e)
        })?;

        let idl_ref: IdlRef = IdlRef::from_bytes(&idl_bytes).map_err(|e| {
            format!("invalid anchor idl at location {}: {}", &idl_path.to_str().unwrap_or(""), e)
        })?;

        let bin: Vec<u8> = std::fs::read(&bin_path).map_err(|e| {
            format!(
                "invalid anchor program binary location {}: {}",
                &bin_path.to_str().unwrap_or(""),
                e
            )
        })?;

        validate_program_so(&bin)?;

        let keypair = if std::fs::exists(&keypair_path).map_err(|e| {
            format!(
                "invalid location for anchor program keypair {}: {}",
                &keypair_path.to_str().unwrap_or(""),
                e
            )
        })? {
            let keypair_file: Vec<u8> = std::fs::read(&keypair_path).map_err(|e| {
                format!(
                    "invalid anchor program keypair location {}: {}",
                    &keypair_path.to_str().unwrap_or(""),
                    e
                )
            })?;

            let keypair_bytes: Vec<u8> = serde_json::from_slice(&keypair_file).map_err(|e| {
                format!(
                    "invalid anchor program keypair at location {}: {}",
                    &keypair_path.to_str().unwrap_or(""),
                    e
                )
            })?;

            let keypair: Keypair = Keypair::try_from(keypair_bytes.as_ref()).map_err(|e| {
                format!(
                    "invalid anchor program keypair at location {}: {}",
                    &keypair_path.to_str().unwrap_or(""),
                    e
                )
            })?;
            Some(keypair)
        } else {
            None
        };

        let program_id = idl_ref.get_program_pubkey().map_err(|e| {
            format!(
                "invalid anchor program idl at location {}: {}",
                &idl_path.to_str().unwrap_or(""),
                e
            )
        })?;

        let idl = idl_ref.as_anchor().ok_or_else(|| {
            format!(
                "expected Anchor IDL at location {}, but found a different IDL format",
                &idl_path.to_str().unwrap_or("")
            )
        })?.clone();

        Ok(Self { idl, bin, keypair, program_id })
    }

    pub fn to_value(&self) -> Result<Value, String> {
        // let idl_bytes =
        //     serde_json::to_vec(&self.idl).map_err(|e| format!("invalid anchor idl: {e}"))?;

        let idl_str = serde_json::to_string_pretty(&self.idl)
            .map_err(|e| format!("invalid anchor idl: {e}"))?;

        let mut obj = ObjectType::from(vec![
            ("binary", SvmValue::binary(self.bin.clone())),
            ("idl", Value::string(idl_str)),
            ("program_id", SvmValue::pubkey(self.program_id.to_bytes().to_vec())),
            ("framework", Value::string("anchor".to_string())),
        ]);
        if let Some(keypair) = &self.keypair {
            obj.insert("keypair", SvmValue::keypair(keypair.to_bytes().to_vec()));
        }
        Ok(obj.to_value())
    }

    pub fn from_map(map: &IndexMap<String, Value>) -> Result<Self, Diagnostic> {
        let bin = match map.get("binary") {
            Some(Value::Addon(addon_data)) => addon_data.bytes.clone(),
            _ => return Err(diagnosed_error!("anchor artifacts missing binary")),
        };
        // let idl_bytes = match map.get("idl") {
        //     Some(Value::Addon(addon_data)) => addon_data.bytes.clone(),
        //     _ => return Err("anchor artifacts missing idl".to_string()),
        // };
        let idl_str =
            map.get("idl").ok_or(diagnosed_error!("anchor artifacts missing idl"))?.to_string();
        // let idl: Idl =
        //     serde_json::from_slice(&idl_bytes).map_err(|e| format!("invalid anchor idl: {e}"))?;

        let idl: anchor_types::Idl = serde_json::from_str(&idl_str)
            .map_err(|e| diagnosed_error!("invalid anchor idl: {e}"))?;

        let keypair = match map.get("keypair") {
            Some(keypair) => Some(SvmValue::to_keypair(keypair)?),
            _ => None,
        };
        let program_id = SvmValue::to_pubkey(map.get("program_id").ok_or(diagnosed_error!(
            "native program artifacts value is missing program_id data"
        ))?)
        .map_err(|e| diagnosed_error!("{e}"))?;
        Ok(Self { idl, bin, keypair, program_id })
    }
}
