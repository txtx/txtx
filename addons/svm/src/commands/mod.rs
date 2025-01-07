use crate::constants::{SIGNER, SIGNERS};
// use encode_instruction::ENCODE_INSTRUCTION;
use deploy_program::DEPLOY_PROGRAM;
use process_instructions::PROCESS_INSTRUCTIONS;
use send_sol::SEND_SOL;
use send_token::SEND_TOKEN;
use send_transaction::SEND_TRANSACTION;
use sign_transaction::SIGN_TRANSACTION;
use txtx_addon_kit::types::commands::PreCommandSpecification;
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::{diagnostics::Diagnostic, ConstructDid, Did};

pub mod deploy_program;
pub mod process_instructions;
pub mod send_sol;
pub mod send_token;
pub mod send_transaction;
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

lazy_static! {
    pub static ref ACTIONS: Vec<PreCommandSpecification> = vec![
        SIGN_TRANSACTION.clone(),
        // ENCODE_INSTRUCTION.clone(),
        SEND_TRANSACTION.clone(),
        PROCESS_INSTRUCTIONS.clone(),
        DEPLOY_PROGRAM.clone(),
        SEND_SOL.clone(),
        SEND_TOKEN.clone(),
    ];
}
