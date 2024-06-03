pub mod broadcast_transaction;
pub mod call_readonly_fn;
mod decode_contract_call;
mod deploy_contract;
pub mod encode_contract_call;
mod encode_multisig;
mod send_contract_call;
pub mod set_default_network;
pub mod sign_transaction;

use broadcast_transaction::BROADCAST_STACKS_TRANSACTION;
use call_readonly_fn::CALL_READONLY_FN;
use decode_contract_call::DECODE_STACKS_CONTRACT_CALL;
use deploy_contract::DEPLOY_STACKS_CONTRACT;
use encode_contract_call::ENCODE_STACKS_CONTRACT_CALL;
use encode_multisig::ENCODE_MULTISIG_TRANSACTION;
use send_contract_call::SEND_CONTRACT_CALL;
use set_default_network::SET_DEFAULT_NETWORK;
use sign_transaction::SIGN_STACKS_TRANSACTION;
use txtx_addon_kit::types::commands::PreCommandSpecification;

lazy_static! {
    pub static ref ACTIONS: Vec<PreCommandSpecification> = vec![
        SIGN_STACKS_TRANSACTION.clone(),
        DECODE_STACKS_CONTRACT_CALL.clone(),
        ENCODE_STACKS_CONTRACT_CALL.clone(),
        DEPLOY_STACKS_CONTRACT.clone(),
        BROADCAST_STACKS_TRANSACTION.clone(),
        CALL_READONLY_FN.clone(),
        SET_DEFAULT_NETWORK.clone(),
        ENCODE_MULTISIG_TRANSACTION.clone(),
        SEND_CONTRACT_CALL.clone(),
    ];
}
