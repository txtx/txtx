use txtx_addon_kit::types::types::Value;

pub const SOLANA_ADDRESS: &str = "solana::address";
pub const SOLANA_BYTES: &str = "solana::bytes";
pub const SOLANA_BYTES32: &str = "solana::bytes32";
pub const SOLANA_TRANSACTION: &str = "solana::transaction";
pub const SOLANA_INSTRUCTION: &str = "solana::instruction";
pub const SOLANA_ACCOUNT: &str = "solana::account";
pub const SOLANA_MESSAGE: &str = "solana::message";
pub const SOLANA_TX_HASH: &str = "solana::tx_hash";
pub const SOLANA_INIT_CODE: &str = "solana::init_code";

pub struct SolanaValue {}

impl SolanaValue {
    pub fn address(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SOLANA_ADDRESS)
    }

    pub fn bytes(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SOLANA_BYTES)
    }

    pub fn bytes32(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SOLANA_BYTES32)
    }

    pub fn transaction(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SOLANA_TRANSACTION)
    }

    pub fn instruction(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SOLANA_INSTRUCTION)
    }

    pub fn account(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SOLANA_ACCOUNT)
    }

    pub fn message(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SOLANA_MESSAGE)
    }

    pub fn tx_hash(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SOLANA_TX_HASH)
    }

    pub fn init_code(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SOLANA_INIT_CODE)
    }
}
