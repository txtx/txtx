use std::str::FromStr;

use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use txtx_addon_kit::types::types::{Type, Value};

pub const SVM_ADDRESS: &str = "svm::address";
pub const SVM_BYTES: &str = "svm::bytes";
pub const SVM_BYTES32: &str = "svm::bytes32";
pub const SVM_TRANSACTION: &str = "svm::transaction";
pub const SVM_INSTRUCTION: &str = "svm::instruction";
pub const SVM_ACCOUNT: &str = "svm::account";
pub const SVM_MESSAGE: &str = "svm::message";
pub const SVM_TX_HASH: &str = "svm::tx_hash";
pub const SVM_INIT_CODE: &str = "svm::init_code";
pub const SVM_BINARY: &str = "svm::binary";
pub const SVM_IDL: &str = "svm::idl";
pub const SVM_KEYPAIR: &str = "svm::keypair";
pub const SVM_PUBKEY: &str = "svm::pubkey";
pub const SVM_TRANSACTION_WITH_KEYPAIRS: &str = "svm::transaction_with_keypairs";

pub struct SvmValue {}

impl SvmValue {
    pub fn address(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SVM_ADDRESS)
    }

    pub fn bytes(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SVM_BYTES)
    }

    pub fn bytes32(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SVM_BYTES32)
    }

    pub fn transaction(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SVM_TRANSACTION)
    }

    pub fn instruction(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SVM_INSTRUCTION)
    }

    pub fn account(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SVM_ACCOUNT)
    }

    pub fn message(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SVM_MESSAGE)
    }

    pub fn tx_hash(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SVM_TX_HASH)
    }

    pub fn init_code(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SVM_INIT_CODE)
    }

    pub fn binary(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SVM_BINARY)
    }

    pub fn idl(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SVM_IDL)
    }

    pub fn keypair(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SVM_KEYPAIR)
    }

    pub fn pubkey(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SVM_PUBKEY)
    }

    pub fn to_pubkey(value: &Value) -> Result<Pubkey, String> {
        match value.as_string() {
            Some(s) => {
                return Pubkey::from_str(s)
                    .map_err(|e| format!("could not convert value to pubkey: {e}"))
            }
            None => {}
        };
        let bytes = value.to_bytes();
        let bytes: [u8; 32] = bytes[0..32]
            .try_into()
            .map_err(|e| format!("could not convert value to pubkey: {e}"))?;
        Ok(Pubkey::new_from_array(bytes))
    }

    pub fn transaction_with_keypairs(transaction_bytes: Vec<u8>, keypairs: Vec<&Keypair>) -> Value {
        let keypairs_bytes: Vec<Vec<u8>> =
            keypairs.iter().map(|keypair| keypair.to_bytes().to_vec()).collect();
        let transaction_with_keypairs = (&transaction_bytes, &keypairs_bytes);
        let bytes = serde_json::to_vec(&transaction_with_keypairs).unwrap();
        Value::addon(bytes, SVM_TRANSACTION_WITH_KEYPAIRS)
    }
}

lazy_static! {
    pub static ref ANCHOR_PROGRAM_ARTIFACTS: Type = define_object_type! {
        idl: {
            documentation: "The program idl.",
            // typing: Type::addon(SVM_IDL),
            typing: Type::string(),
            optional: false,
            tainting: true
        },
        binary: {
            documentation: "The program binary.",
            typing: Type::addon(SVM_BINARY),
            optional: false,
            tainting: false
        },
        keypair: {
            documentation: "The program keypair.",
            typing: Type::addon(SVM_KEYPAIR),
            optional: false,
            tainting: true
        },
        program_id: {
            documentation: "The program id.",
            typing: Type::addon(SVM_PUBKEY),
            optional: false,
            tainting: true
        }
    };
    pub static ref PDA_RESULT: Type = define_object_type! {
        pda: {
            documentation: "The program derived address.",
            typing: Type::addon(SVM_PUBKEY),
            optional: false,
            tainting: true
        },
        bump_seed: {
            documentation: "The bump seed.",
            typing: Type::integer(),
            optional: false,
            tainting: true
        }
    };
}
