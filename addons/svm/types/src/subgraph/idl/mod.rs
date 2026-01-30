use anchor_lang_idl::types::Idl as AnchorIdl;
use serde::{Deserialize, Serialize};
use shank_idl::idl::Idl as ShankIdl;
use txtx_addon_kit::types::diagnostics::Diagnostic;

pub mod anchor;
pub mod shank;

/// Represents the kind of IDL format being used.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IdlKind {
    /// Anchor IDL format (v0.30+)
    Anchor(AnchorIdl),
    /// Shank IDL format
    Shank(ShankIdl),
}
impl IdlKind {
    pub fn to_json_value(&self) -> Result<serde_json::Value, Diagnostic> {
        match self {
            IdlKind::Anchor(idl) => serde_json::to_value(idl)
                .map_err(|e| diagnosed_error!("failed to serialize Anchor IDL: {}", e)),
            IdlKind::Shank(idl) => serde_json::to_value(idl)
                .map_err(|e| diagnosed_error!("failed to serialize Shank IDL: {}", e)),
        }
    }
}
