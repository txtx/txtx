use call_contract::SEND_CONTRACT_CALL;
use sign_transaction::SIGN_SOLANA_TRANSACTION;
use txtx_addon_kit::types::{
    commands::PreCommandSpecification, diagnostics::Diagnostic, ConstructDid, Did, ValueStore,
};
mod call_contract;
pub mod sign_transaction;
use crate::constants::SIGNER;

lazy_static! {
    pub static ref ACTIONS: Vec<PreCommandSpecification> =
        vec![SEND_CONTRACT_CALL.clone(), SIGN_SOLANA_TRANSACTION.clone()];
}

fn get_signer_did(args: &ValueStore) -> Result<ConstructDid, Diagnostic> {
    let signer = args.get_expected_string(SIGNER)?;
    let signer_did = ConstructDid(Did::from_hex_string(signer));
    Ok(signer_did)
}
