mod send_contract_call;
mod sign_transaction;

use send_contract_call::SEND_CONTRACT_CALL;
use sign_transaction::SIGN_STACKS_TRANSACTION;
use txtx_addon_kit::types::commands::PreCommandSpecification;

lazy_static! {
    pub static ref PROMPTS: Vec<PreCommandSpecification> =
        vec![SEND_CONTRACT_CALL.clone(), SIGN_STACKS_TRANSACTION.clone()];
}
