use std::str::FromStr;

use solana_sdk::{pubkey::Pubkey, signature::Keypair, transaction::Transaction};
use txtx_addon_kit::{
    hex,
    types::{
        diagnostics::Diagnostic,
        types::{ObjectProperty, Type, Value},
    },
};

use crate::codec::DeploymentTransaction;

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
pub const SVM_DEPLOYMENT_TRANSACTION: &str = "svm::deployment_transaction";
pub const SVM_CLOSE_TEMP_AUTHORITY_TRANSACTION_PARTS: &str =
    "svm::close_temp_authority_transaction_parts";
pub const SVM_PAYER_SIGNED_TRANSACTION: &str = "svm::payer_signed_transaction";
pub const SVM_AUTHORITY_SIGNED_TRANSACTION: &str = "svm::authority_signed_transaction";
pub const SVM_TEMP_AUTHORITY_SIGNED_TRANSACTION: &str = "svm::temp_authority_signed_transaction";

pub struct SvmValue {}

fn is_hex(str: &str) -> bool {
    decode_hex(str).map(|_| true).unwrap_or(false)
}

fn decode_hex(str: &str) -> Result<Vec<u8>, Diagnostic> {
    let stripped = if str.starts_with("0x") { &str[2..] } else { &str[..] };
    hex::decode(stripped)
        .map_err(|e| diagnosed_error!("string '{}' could not be decoded to hex bytes: {}", str, e))
}

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

    pub fn transaction_from_bytes(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SVM_TRANSACTION)
    }

    pub fn transaction(transaction: &Transaction) -> Result<Value, Diagnostic> {
        let bytes = serde_json::to_vec(&transaction)
            .map_err(|e| diagnosed_error!("failed to deserialize transaction: {e}"))?;
        Ok(Value::addon(bytes, SVM_TRANSACTION))
    }

    pub fn to_transaction(value: &Value) -> Result<Transaction, Diagnostic> {
        match value {
            Value::String(s) => {
                if is_hex(s) {
                    let hex = decode_hex(s)?;
                    return serde_json::from_slice(&hex)
                        .map_err(|e| diagnosed_error!("could not deserialize transaction: {e}"));
                }
                return serde_json::from_str(s)
                    .map_err(|e| diagnosed_error!("could not deserialize transaction: {e}"));
            }
            Value::Addon(addon_data) => {
                if addon_data.id == SVM_TRANSACTION {
                    return serde_json::from_slice(&addon_data.bytes)
                        .map_err(|e| diagnosed_error!("could not deserialize transaction: {e}"));
                } else if addon_data.id == SVM_DEPLOYMENT_TRANSACTION
                    || addon_data.id == SVM_CLOSE_TEMP_AUTHORITY_TRANSACTION_PARTS
                {
                    let deployment_transaction = DeploymentTransaction::from_value(value)
                        .map_err(|e| diagnosed_error!("could not deserialize transaction: {e}"))?;
                    return Ok(deployment_transaction.transaction);
                } else {
                    return Err(diagnosed_error!(
                        "could not deserialize addon type '{}' into transaction",
                        addon_data.id
                    ));
                }
            }
            _ => {
                return Err(diagnosed_error!(
                    "could not deserialize transaction: expected string or addon"
                ))
            }
        };
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

    pub fn to_keypair(value: &Value) -> Result<Keypair, Diagnostic> {
        let bytes = value.to_bytes();
        Keypair::from_bytes(&bytes)
            .map_err(|e| diagnosed_error!("could not convert value to keypair: {e}"))
    }

    pub fn pubkey(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SVM_PUBKEY)
    }

    pub fn to_pubkey(value: &Value) -> Result<Pubkey, String> {
        match value.as_string() {
            Some(s) => {
                return Pubkey::from_str(s)
                    .map_err(|e| format!("could not convert value to pubkey: {e}"));
            }
            None => {}
        };
        let bytes = value.to_bytes();
        let bytes: [u8; 32] = bytes[0..32]
            .try_into()
            .map_err(|e| format!("could not convert value to pubkey: {e}"))?;
        Ok(Pubkey::new_from_array(bytes))
    }

    pub fn deployment_transaction(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SVM_DEPLOYMENT_TRANSACTION)
    }

    pub fn close_temp_authority_transaction_parts(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SVM_CLOSE_TEMP_AUTHORITY_TRANSACTION_PARTS)
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

    pub static ref INSTRUCTION_TYPE: Type = Type::map(vec![
        ObjectProperty {
            name: "description".into(),
            documentation: "A description of the instruction.".into(),
            typing: Type::string(),
            optional: true,
            tainting: false,
            internal: false,
        },
        ObjectProperty {
            name: "program_id".into(),
            documentation: "The SVM address of the program being invoked.".into(),
            typing: Type::string(),
            optional: false,
            tainting: true,
            internal: false
        },
        ObjectProperty {
            name: "account".into(),
            documentation: "A map of accounts (including other programs) that are read from or written to by the instruction.".into(),
            typing: ACCOUNT_META_TYPE.clone(),
            optional: false,
            tainting: true,
            internal: false
        },
        ObjectProperty {
            name: "data".into(),
            documentation: "A byte array that specifies which instruction handler on the program to invoke, plus any additional data required by the instruction handler, such as function arguments.".into(),
            typing: Type::buffer(),
            optional: true,
            tainting: true,
            internal: false
        }
    ]);

    pub static ref ACCOUNT_META_TYPE: Type = Type::map(vec![
        ObjectProperty {
            name: "public_key".into(),
            documentation: "The public key (SVM address) of the account.".into(),
            typing: Type::string(),
            optional: false,
            tainting: true,
            internal: false,
        },
        ObjectProperty {
            name: "is_signer".into(),
            documentation: "Specifies if the account is a signer on the instruction. The default is 'false'.".into(),
            typing: Type::bool(),
            optional: true,
            tainting: true,
            internal: false
        },
        ObjectProperty {
            name: "is_writable".into(),
            documentation: "Specifies if the account is written to by the instruction. The default is 'false'.".into(),
            typing: Type::bool(),
            optional: true,
            tainting: true,
            internal: false
        }
    ]);
}
