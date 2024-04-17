pub mod broadcast_transaction;
mod decode_contract_call;
mod deploy_contract;
pub mod encode_contract_call;
pub mod sign_transaction;

use broadcast_transaction::BROADCAST_STACKS_TRANSACTION;
use decode_contract_call::DECODE_STACKS_CONTRACT_CALL;
use deploy_contract::DEPLOY_STACKS_CONTRACT;
use encode_contract_call::ENCODE_STACKS_CONTRACT_CALL;
use sign_transaction::SIGN_STACKS_TRANSACTION;
use txtx_addon_kit::types::commands::PreCommandSpecification;

lazy_static! {
    pub static ref STACKS_ACTIONS: Vec<PreCommandSpecification> = vec![
        SIGN_STACKS_TRANSACTION.clone(),
        DECODE_STACKS_CONTRACT_CALL.clone(),
        ENCODE_STACKS_CONTRACT_CALL.clone(),
        DEPLOY_STACKS_CONTRACT.clone(),
        BROADCAST_STACKS_TRANSACTION.clone()
    ];
}
