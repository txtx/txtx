mod send_contract_call;

use send_contract_call::SEND_CONTRACT_CALL;
use txtx_addon_kit::types::commands::CommandSpecification;

lazy_static! {
    pub static ref STACKS_PROMPTS: Vec<CommandSpecification> = vec![SEND_CONTRACT_CALL.clone(),];
}
