use crate::typing::anchor::types as anchor_types;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
use txtx_addon_kit::{
    helpers::fs::FileLocation,
    types::{
        diagnostics::Diagnostic,
        types::{ObjectType, Value},
    },
};

use crate::typing::SvmValue;

use super::idl::IdlRef;

pub struct ClassicRustProgramArtifacts {
    /// The binary of the rust program, stored for a rust project at `target/deploy/<program_name>.so`.
    pub bin: Vec<u8>,
    /// The keypair of the rust program, stored for a rust project at `target/deploy/<program_name>-keypair.json`.
    pub keypair: Keypair,
    /// The program pubkey of the rust program.
    pub program_id: Pubkey,
    /// The IDL of the program, if provided. IDLs are converted to anchor-style IDLs.
    pub idl: Option<anchor_types::Idl>,
}

impl ClassicRustProgramArtifacts {
    pub fn new(
        keypair_path: FileLocation,
        idl_path: FileLocation,
        bin_path: FileLocation,
    ) -> Result<Self, Diagnostic> {
        let some_idl = if idl_path.exists() {
            let idl_str = idl_path.read_content_as_utf8().map_err(|e| {
                diagnosed_error!("invalid idl location {}: {}", &idl_path.to_string(), e)
            })?;

            let idl = IdlRef::from_str(&idl_str).map_err(|e| {
                diagnosed_error!("invalid idl at location {}: {}", &idl_path.to_string(), e)
            })?;
            Some(idl.idl)
        } else {
            None
        };

        let bin = bin_path.read_content().map_err(|e| {
            diagnosed_error!(
                "invalid rust program binary location {}: {}",
                &bin_path.to_string(),
                e
            )
        })?;

        let keypair_file = keypair_path.read_content().map_err(|e| {
            diagnosed_error!(
                "invalid rust program keypair location {}: {}",
                &keypair_path.to_string(),
                e
            )
        })?;

        let keypair_bytes: Vec<u8> = serde_json::from_slice(&keypair_file).map_err(|e| {
            diagnosed_error!(
                "invalid rust program keypair at location {}: {}",
                &keypair_path.to_string(),
                e
            )
        })?;

        let keypair = Keypair::from_bytes(&keypair_bytes).map_err(|e| {
            diagnosed_error!(
                "invalid rust program keypair at location {}: {}",
                &keypair_path.to_string(),
                e
            )
        })?;

        let program_id = Pubkey::from(keypair.pubkey());

        Ok(ClassicRustProgramArtifacts { bin, keypair, program_id, idl: some_idl })
    }

    pub fn to_value(&self) -> Result<Value, Diagnostic> {
        let mut obj = ObjectType::from(vec![
            ("binary", SvmValue::binary(self.bin.clone())),
            ("keypair", SvmValue::keypair(self.keypair.to_bytes().to_vec())),
            ("program_id", SvmValue::pubkey(self.program_id.to_bytes().to_vec())),
            ("framework", Value::string("native".to_string())),
        ]);
        if let Some(idl) = &self.idl {
            let idl_str = serde_json::to_string_pretty(&idl)
                .map_err(|e| diagnosed_error!("invalid idl: {e}"))?;

            obj.insert("idl", Value::string(idl_str));
        };
        Ok(obj.to_value())
    }

    pub fn from_value(value: &Value) -> Result<Self, Diagnostic> {
        let map = value
            .as_object()
            .ok_or(diagnosed_error!("rust program artifacts must be an object"))?;

        let bin = map
            .get("binary")
            .ok_or(diagnosed_error!("rust program artifacts value is missing binary data"))?
            .get_buffer_bytes_result()
            .map_err(|e| diagnosed_error!("{e}"))?;

        let keypair = SvmValue::to_keypair(
            map.get("keypair")
                .ok_or(diagnosed_error!("rust program artifacts value is missing keypair data"))?,
        )?;

        let program_id =
            SvmValue::to_pubkey(map.get("program_id").ok_or(diagnosed_error!(
                "rust program artifacts value is missing program_id data"
            ))?)
            .map_err(|e| diagnosed_error!("{e}"))?;

        let idl = if let Some(idl_value) = map.get("idl") {
            let idl_str = idl_value.as_string().ok_or(diagnosed_error!(
                "rust program artifacts value had invalid idl data: expected string"
            ))?;
            let idl: anchor_types::Idl =
                serde_json::from_str(idl_str).map_err(|e| diagnosed_error!("{e}"))?;
            Some(idl)
        } else {
            None
        };

        Ok(ClassicRustProgramArtifacts { bin, keypair, program_id, idl })
    }
}
