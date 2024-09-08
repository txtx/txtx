use txtx_addon_kit::types::types::{Type, Value};

pub const SOL_ADDRESS: &str = "solana::address";
pub const SOL_BYTES: &str = "solana::bytes";
pub const SOL_BYTES32: &str = "solana::bytes32";
pub const SOL_TRANSACTION: &str = "solana::transaction";
pub const SOL_INSTRUCTION: &str = "solana::instruction";
pub const SOL_MESSAGE: &str = "solana::message";
pub const SOL_TX_HASH: &str = "solana::tx_hash";
pub const SOL_INIT_CODE: &str = "solana::init_code";

pub struct SolanaValue {}

impl SolanaValue {
    pub fn address(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SOL_ADDRESS)
    }

    pub fn bytes(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SOL_BYTES)
    }

    pub fn bytes32(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SOL_BYTES32)
    }

    pub fn transaction(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SOL_TRANSACTION)
    }

    pub fn instruction(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SOL_INSTRUCTION)
    }

    pub fn message(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SOL_MESSAGE)
    }

    pub fn tx_hash(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SOL_TX_HASH)
    }

    pub fn init_code(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SOL_INIT_CODE)
    }
}
