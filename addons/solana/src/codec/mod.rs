use solana_sdk::{instruction::Instruction, message::Message, transaction::Transaction};
use txtx_addon_kit::types::types::Value;

pub fn encode_contract_call(instruction_bytes: &Vec<Vec<u8>>) -> Result<Value, String> {
    let instructions = instruction_bytes
        .iter()
        .map(|i| {
            serde_json::from_slice(&i)
                .map_err(|e| format!("failed to deserialize instruction: {e}"))
        })
        .collect::<Result<Vec<Instruction>, String>>()?;

    let message = Message::new(&instructions, None);
    let message_bytes = message.serialize();
    Ok(Value::buffer(message_bytes))
}
