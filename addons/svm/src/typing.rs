use std::str::FromStr;

use solana_sdk::{pubkey::Pubkey, signature::Keypair, transaction::Transaction};
use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    types::{ObjectProperty, Type, Value},
};

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
pub const SVM_PAYER_SIGNED_TRANSACTION: &str = "svm::payer_signed_transaction";
pub const SVM_AUTHORITY_SIGNED_TRANSACTION: &str = "svm::authority_signed_transaction";
pub const SVM_TEMP_AUTHORITY_SIGNED_TRANSACTION: &str = "svm::temp_authority_signed_transaction";

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
                return serde_json::from_str(s)
                    .map_err(|e| diagnosed_error!("could not deserialize transaction: {e}"))
            }
            Value::Addon(addon_data) => {
                if addon_data.id != SVM_TRANSACTION {
                    return Err(diagnosed_error!(
                        "could not deserialize transaction: expected addon id '{SVM_TRANSACTION}' but got '{}'",addon_data.id
                    ));
                }
                return serde_json::from_slice(&addon_data.bytes)
                    .map_err(|e| diagnosed_error!("could not deserialize transaction: {e}"));
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

    pub fn parse_transaction_with_keypairs(
        value: &Value,
    ) -> Result<(Vec<u8>, Vec<Vec<u8>>), Diagnostic> {
        let addon_data = value.as_addon_data().ok_or(diagnosed_error!("expected addon"))?;
        match addon_data.id.as_str() {
            SVM_AUTHORITY_SIGNED_TRANSACTION
            | SVM_PAYER_SIGNED_TRANSACTION
            | SVM_TEMP_AUTHORITY_SIGNED_TRANSACTION => {}
            _ => {
                return Err(diagnosed_error!(
                    "expected addon id '{SVM_AUTHORITY_SIGNED_TRANSACTION}' or '{SVM_PAYER_SIGNED_TRANSACTION}' or '{SVM_TEMP_AUTHORITY_SIGNED_TRANSACTION}' but got '{}'",
                    addon_data.id
                ));
            }
        }
        let (transaction_bytes, available_keypair_bytes): (Vec<u8>, Vec<Vec<u8>>) =
            serde_json::from_slice(&addon_data.bytes).map_err(|e| {
                diagnosed_error!("failed to deserialize transaction with keypairs for signing: {e}")
            })?;
        Ok((transaction_bytes, available_keypair_bytes))
    }

    /// Creates a [Value] containing a transaction and a set of non-txtx-signer keypairs
    /// that need to sign the transaction.
    /// The txtx-signer that is expected to sign the transaction is the `payer`
    /// of a program deployment.
    pub fn payer_signed_transaction(
        transaction: &Transaction,
        keypairs: Vec<&Keypair>,
    ) -> Result<Value, Diagnostic> {
        let transaction_bytes = serde_json::to_vec(&transaction)
            .map_err(|e| diagnosed_error!("failed to serialize transaction: {e}"))?;

        let keypairs_bytes: Vec<Vec<u8>> =
            keypairs.iter().map(|keypair| keypair.to_bytes().to_vec()).collect();
        let transaction_with_keypairs = (&transaction_bytes, &keypairs_bytes);
        let bytes = serde_json::to_vec(&transaction_with_keypairs).unwrap();

        Ok(Value::addon(bytes, SVM_PAYER_SIGNED_TRANSACTION))
    }

    /// Creates a [Value] containing a transaction and a set of non-txtx-signer keypairs
    /// that need to sign the transaction.
    /// The txtx-signer that is expected to sign the transaction is the `authority`
    /// of a program deployment.
    pub fn authority_signed_transaction(
        transaction: &Transaction,
        keypairs: Vec<&Keypair>,
    ) -> Result<Value, Diagnostic> {
        let transaction_bytes = serde_json::to_vec(&transaction)
            .map_err(|e| diagnosed_error!("failed to serialize transaction: {e}"))?;

        let keypairs_bytes: Vec<Vec<u8>> =
            keypairs.iter().map(|keypair| keypair.to_bytes().to_vec()).collect();
        let transaction_with_keypairs = (&transaction_bytes, &keypairs_bytes);
        let bytes = serde_json::to_vec(&transaction_with_keypairs).unwrap();

        Ok(Value::addon(bytes, SVM_AUTHORITY_SIGNED_TRANSACTION))
    }

    /// Creates a [Value] containing a transaction and a set of non-txtx-signer keypairs
    /// that need to sign the transaction.
    /// The txtx-signer that is expected to sign the transaction is the `temp_authority`
    /// of a program deployment, which is already one of the provided keypairs.
    pub fn temp_authority_signed_transaction(
        transaction: &Transaction,
        keypairs: Vec<&Keypair>,
    ) -> Result<Value, Diagnostic> {
        let transaction_bytes = serde_json::to_vec(&transaction)
            .map_err(|e| diagnosed_error!("failed to serialize transaction: {e}"))?;

        let keypairs_bytes: Vec<Vec<u8>> =
            keypairs.iter().map(|keypair| keypair.to_bytes().to_vec()).collect();
        let transaction_with_keypairs = (&transaction_bytes, &keypairs_bytes);
        let bytes = serde_json::to_vec(&transaction_with_keypairs).unwrap();

        Ok(Value::addon(bytes, SVM_TEMP_AUTHORITY_SIGNED_TRANSACTION))
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
