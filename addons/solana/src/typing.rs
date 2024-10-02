use txtx_addon_kit::types::types::{Type, Value};

pub const SOLANA_ADDRESS: &str = "solana::address";
pub const SOLANA_BYTES: &str = "solana::bytes";
pub const SOLANA_BYTES32: &str = "solana::bytes32";
pub const SOLANA_TRANSACTION: &str = "solana::transaction";
pub const SOLANA_INSTRUCTION: &str = "solana::instruction";
pub const SOLANA_ACCOUNT: &str = "solana::account";
pub const SOLANA_MESSAGE: &str = "solana::message";
pub const SOLANA_TX_HASH: &str = "solana::tx_hash";
pub const SOLANA_INIT_CODE: &str = "solana::init_code";
pub const SOLANA_BINARY: &str = "solana::binary";
pub const SOLANA_IDL: &str = "solana::idl";
pub const SOLANA_KEYPAIR: &str = "solana::keypair";
pub const SOLANA_PUBKEY: &str = "solana::pubkey";

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

    pub fn binary(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SOLANA_BINARY)
    }

    pub fn idl(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SOLANA_IDL)
    }

    pub fn keypair(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SOLANA_KEYPAIR)
    }

    pub fn pubkey(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SOLANA_PUBKEY)
    }
}

lazy_static! {
    pub static ref ANCHOR_PROGRAM_ARTIFACTS: Type = define_object_type! {
        idl: {
            documentation: "The program idl.",
            // typing: Type::addon(SOLANA_IDL),
            typing: Type::string(),
            optional: false,
            tainting: true
        },
        binary: {
            documentation: "The program binary.",
            typing: Type::addon(SOLANA_BINARY),
            optional: false,
            tainting: false
        },
        keypair: {
            documentation: "The program keypair.",
            typing: Type::addon(SOLANA_KEYPAIR),
            optional: false,
            tainting: true
        },
        program_id: {
            documentation: "The program id.",
            typing: Type::addon(SOLANA_PUBKEY),
            optional: false,
            tainting: true
        }
    };
}
