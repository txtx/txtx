use crate::constants::{SIGNER, SIGNERS};
use deploy_program::DEPLOY_PROGRAM;
use deploy_subraph::DEPLOY_SUBGRAPH;
use process_instructions::PROCESS_INSTRUCTIONS;
use send_sol::SEND_SOL;
use send_token::SEND_TOKEN;
use setup_surfnet::SETUP_SURFNET;
use txtx_addon_kit::types::commands::PreCommandSpecification;
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::{diagnostics::Diagnostic, ConstructDid, Did};

pub mod deploy_program;
pub mod deploy_subraph;
pub mod process_instructions;
pub mod send_sol;
pub mod send_token;
mod setup_surfnet;
pub mod sign_transaction;

fn get_signers_did(args: &ValueStore) -> Result<Vec<ConstructDid>, Diagnostic> {
    let signers = args.get_expected_array(SIGNERS)?;
    let mut res = vec![];
    for signer in signers.iter() {
        res.push(ConstructDid(Did::from_hex_string(signer.expect_string())));
    }
    Ok(res)
}
fn get_signer_did(args: &ValueStore) -> Result<ConstructDid, Diagnostic> {
    let signer = args.get_expected_string(SIGNER)?;
    Ok(ConstructDid(Did::from_hex_string(signer)))
}

pub fn get_custom_signer_did(
    args: &ValueStore,
    signer_key: &str,
) -> Result<ConstructDid, Diagnostic> {
    let signer = args.get_expected_string(signer_key)?;
    Ok(ConstructDid(Did::from_hex_string(signer)))
}

lazy_static! {
    pub static ref ACTIONS: Vec<PreCommandSpecification> = vec![
        PROCESS_INSTRUCTIONS.clone(),
        DEPLOY_PROGRAM.clone(),
        SEND_SOL.clone(),
        SEND_TOKEN.clone(),
        DEPLOY_SUBGRAPH.clone(),
        SETUP_SURFNET.clone()
    ];
}
