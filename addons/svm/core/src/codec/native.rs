use crate::codec::validate_program_so;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use txtx_addon_kit::{
    helpers::fs::FileLocation,
    types::{
        diagnostics::Diagnostic,
        types::{ObjectType, Value},
    },
};

use crate::typing::SvmValue;

use super::idl::{IdlKind, IdlRef};

pub struct NativeProgramArtifacts {
    /// The binary of the native program, stored for a native project at `target/deploy/<program_name>.so`.
    pub bin: Vec<u8>,
    /// The keypair of the native program, stored for a native project at `target/deploy/<program_name>-keypair.json`.
    /// Only needed for initial deployments, so optional.
    pub keypair: Option<Keypair>,
    /// The program pubkey of the native program.
    pub program_id: Pubkey,
    /// The IDL of the program, if provided. Can be either Anchor or Shank format.
    pub idl: Option<IdlKind>,
}

impl NativeProgramArtifacts {
    pub fn new(
        keypair_path: FileLocation,
        idl_path: FileLocation,
        bin_path: FileLocation,
    ) -> Result<Self, Diagnostic> {
        let some_idl_ref = if idl_path.exists() {
            let idl_str = idl_path.read_content_as_utf8().map_err(|e| {
                diagnosed_error!("invalid idl location {}: {}", &idl_path.to_string(), e)
            })?;

            let idl_ref = IdlRef::from_str(&idl_str).map_err(|e| {
                diagnosed_error!("invalid idl at location {}: {}", &idl_path.to_string(), e)
            })?;
            Some(idl_ref)
        } else {
            None
        };

        let bin = bin_path.read_content().map_err(|e| {
            diagnosed_error!(
                "invalid native program binary location {}: {}",
                &bin_path.to_string(),
                e
            )
        })?;

        validate_program_so(&bin)?;

        let keypair = if keypair_path.exists() {
            let keypair_file = keypair_path.read_content().map_err(|e| {
                diagnosed_error!(
                    "invalid native program keypair location {}: {}",
                    &keypair_path.to_string(),
                    e
                )
            })?;

            let keypair_bytes: Vec<u8> = serde_json::from_slice(&keypair_file).map_err(|e| {
                diagnosed_error!(
                    "invalid native program keypair at location {}: {}",
                    &keypair_path.to_string(),
                    e
                )
            })?;

            let keypair = Keypair::try_from(keypair_bytes.as_ref()).map_err(|e| {
                diagnosed_error!(
                    "invalid native program keypair at location {}: {}",
                    &keypair_path.to_string(),
                    e
                )
            })?;
            Some(keypair)
        } else {
            None
        };

        let program_id = match (keypair.as_ref(), some_idl_ref.as_ref()) {
            (_, Some(idl_ref)) => idl_ref.get_program_pubkey().map_err(|e| {
                diagnosed_error!(
                    "invalid program id in idl at location {}: {}",
                    &idl_path.to_string(),
                    e
                )
            })?,
            (Some(keypair), None) => keypair.pubkey(),
            (None, None) => {
                return Err(diagnosed_error!(
                    "native program artifacts must have either a keypair or an idl to derive the program id"
                ));
            }
        };

        let some_idl = some_idl_ref.map(|idl_ref| idl_ref.idl);
        Ok(NativeProgramArtifacts { bin, keypair, program_id, idl: some_idl })
    }

    pub fn to_value(&self) -> Result<Value, Diagnostic> {
        let mut obj = ObjectType::from(vec![
            ("binary", SvmValue::binary(self.bin.clone())),
            ("program_id", SvmValue::pubkey(self.program_id.to_bytes().to_vec())),
            ("framework", Value::string("native".to_string())),
        ]);
        if let Some(idl) = &self.idl {
            let idl_str = match idl {
                IdlKind::Anchor(anchor_idl) => serde_json::to_string_pretty(anchor_idl),
                IdlKind::Shank(shank_idl) => serde_json::to_string_pretty(shank_idl),
            }
            .map_err(|e| diagnosed_error!("invalid idl: {e}"))?;

            obj.insert("idl", Value::string(idl_str));
        };
        if let Some(keypair) = &self.keypair {
            obj.insert("keypair", SvmValue::keypair(keypair.to_bytes().to_vec()));
        }

        Ok(obj.to_value())
    }

    pub fn from_value(value: &Value) -> Result<Self, Diagnostic> {
        let map = value
            .as_object()
            .ok_or(diagnosed_error!("native program artifacts must be an object"))?;

        let bin = map
            .get("binary")
            .ok_or(diagnosed_error!("native program artifacts value is missing binary data"))?
            .get_buffer_bytes_result()
            .map_err(|e| diagnosed_error!("{e}"))?;

        let keypair = match map.get("keypair") {
            Some(keypair) => Some(SvmValue::to_keypair(keypair)?),
            _ => None,
        };

        let program_id = SvmValue::to_pubkey(map.get("program_id").ok_or(diagnosed_error!(
            "native program artifacts value is missing program_id data"
        ))?)
        .map_err(|e| diagnosed_error!("{e}"))?;

        let idl = if let Some(idl_value) = map.get("idl") {
            let idl_str = idl_value.as_string().ok_or(diagnosed_error!(
                "native program artifacts value had invalid idl data: expected string"
            ))?;
            let idl_ref =
                IdlRef::from_str(idl_str).map_err(|e| diagnosed_error!("{e}"))?;
            Some(idl_ref.idl)
        } else {
            None
        };

        Ok(NativeProgramArtifacts { bin, keypair, program_id, idl })
    }
}
