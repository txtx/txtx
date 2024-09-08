use crate::constants::SIGNER;
use call_program::SEND_PROGRAM_CALL;
use sign_transaction::SIGN_SOLANA_TRANSACTION;
use txtx_addon_kit::types::commands::PreCommandSpecification;
use txtx_addon_kit::types::{diagnostics::Diagnostic, ConstructDid, Did, ValueStore};

mod call_program;
pub mod sign_transaction;

fn get_signer_did(args: &ValueStore) -> Result<ConstructDid, Diagnostic> {
    let signer = args.get_expected_string(SIGNER)?;
    let signer_did = ConstructDid(Did::from_hex_string(signer));
    Ok(signer_did)
}

lazy_static! {
    pub static ref ACTIONS: Vec<PreCommandSpecification> =
        vec![SEND_PROGRAM_CALL.clone(), SIGN_SOLANA_TRANSACTION.clone()];
}
