use std::str::FromStr;

use solana_sdk::{instruction::Instruction, message::Message, pubkey::Pubkey};
use txtx_addon_kit::types::types::Value;

pub fn encode_contract_call(instructions: &Vec<Instruction>) -> Result<Value, String> {
    let message = Message::new(instructions, None);
    let message_bytes = message.serialize();
    Ok(Value::buffer(message_bytes))
}

pub fn public_key_from_bytes(bytes: &Vec<u8>) -> Result<Pubkey, String> {
    let bytes: [u8; 32] =
        bytes.as_slice().try_into().map_err(|e| format!("invalid public key: {e}"))?;
    Ok(Pubkey::new_from_array(bytes))
}

pub fn public_key_from_str(str: &str) -> Result<Pubkey, String> {
    Pubkey::from_str(str).map_err(|e| format!("invalid public key: {e}"))
}
