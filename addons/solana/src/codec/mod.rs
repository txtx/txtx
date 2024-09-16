use solana_sdk::{instruction::Instruction, message::Message};
use txtx_addon_kit::types::types::Value;

pub fn encode_contract_call(instructions: &Vec<Instruction>) -> Result<Value, String> {
    let message = Message::new(instructions, None);
    let message_bytes = message.serialize();
    Ok(Value::buffer(message_bytes))
}
