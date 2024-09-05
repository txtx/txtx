use txtx_addon_kit::types::{diagnostics::Diagnostic, ConstructDid, Did, ValueStore};

use crate::constants::SIGNER;

pub mod actions;
fn get_signer_did(args: &ValueStore) -> Result<ConstructDid, Diagnostic> {
    let signer = args.get_expected_string(SIGNER)?;
    let signer_did = ConstructDid(Did::from_hex_string(signer));
    Ok(signer_did)
}
