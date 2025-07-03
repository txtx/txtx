#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate txtx_addon_kit;

pub mod subgraph;

use std::str::FromStr;

use serde::{Deserialize, Serialize};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_transaction::Transaction;

use txtx_addon_kit::{
    hex,
    types::{
        diagnostics::Diagnostic,
        types::{Type, Value},
    },
};

pub use anchor_lang_idl as anchor;

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
pub const SVM_U128: &str = "svm::u128";
pub const SVM_U256: &str = "svm::u256";
pub const SVM_I256: &str = "svm::i256";

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

    pub fn u128(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SVM_U128)
    }

    pub fn u256(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SVM_U256)
    }

    pub fn i256(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SVM_I256)
    }

    pub fn transaction_from_bytes(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SVM_TRANSACTION)
    }

    pub fn transaction(transaction: &Transaction) -> Result<Value, Diagnostic> {
        let bytes = serde_json::to_vec(&transaction)
            .map_err(|e| diagnosed_error!("failed to deserialize transaction: {e}"))?;
        Ok(Value::addon(bytes, SVM_TRANSACTION))
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
                if is_hex(s) {
                    let hex = decode_hex(s).map_err(|e| e.message)?;
                    let bytes: [u8; 32] = hex[0..32]
                        .try_into()
                        .map_err(|e| format!("could not convert value to pubkey: {e}"))?;
                    return Ok(Pubkey::new_from_array(bytes));
                }
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
    pub static ref ANCHOR_PROGRAM_ARTIFACTS: Type = define_strict_object_type! {
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

    pub static ref CLASSIC_RUST_PROGRAM_ARTIFACTS: Type = define_strict_object_type! {
        idl: {
            documentation: "The program idl.",
            typing: Type::string(),
            optional: true,
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

    pub static ref PDA_RESULT: Type = define_strict_object_type! {
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

    pub static ref INSTRUCTION_TYPE: Type = define_documented_arbitrary_map_type! {
        raw_bytes: {
            documentation: "The serialized instruction bytes. Use this field in place of the other instructions if direct instruction bytes would like to be used.",
            typing: Type::addon(SVM_INSTRUCTION),
            optional: true,
            tainting: true
        },
        program_id: {
            documentation: "The SVM address of the program being invoked. If omitted, the program_id will be pulled from the idl.",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        program_idl: {
            documentation: "The IDL of the program being invoked.",
            typing: Type::string(),
            optional: false,
            tainting: true
        },
        instruction_name: {
            documentation: "The name of the instruction being invoked.",
            typing: Type::string(),
            optional: false,
            tainting: true
        },
        instruction_args: {
            documentation: "The arguments to the instruction being invoked.",
            typing: Type::array(Type::string()),
            optional: false,
            tainting: true
        }
    };

    pub static ref DEPLOYMENT_TRANSACTION_SIGNATURES: Type = define_strict_object_type! {
        create_temp_authority: {
            documentation: "The signature of the create temp authority transaction.",
            typing: Type::array(Type::string()),
            optional: false,
            tainting: true
        },
        create_buffer: {
            documentation: "The signature of the create buffer transaction.",
            typing: Type::array(Type::string()),
            optional: false,
            tainting: true
        },
        write_to_buffer: {
            documentation: "The signature of the write to buffer transaction.",
            typing: Type::array(Type::string()),
            optional: false,
            tainting: true
        },
        transfer_buffer_authority: {
            documentation: "The signature of the transfer buffer authority transaction.",
            typing: Type::array(Type::string()),
            optional: true,
            tainting: true
        },
        deploy_program: {
            documentation: "The signature of the deploy program transaction.",
            typing: Type::array(Type::string()),
            optional: true,
            tainting: true
        },
        upgrade_program: {
            documentation: "The signature of the upgrade program transaction.",
            typing: Type::array(Type::string()),
            optional: true,
            tainting: true
        },
        close_temp_authority: {
            documentation: "The signature of the close temp authority transaction.",
            typing: Type::array(Type::string()),
            optional: false,
            tainting: true
        }
    };

    pub static ref SUBGRAPH_EVENT: Type = define_strict_map_type! {
        name: {
            documentation: "The name of the event, as indexed by the IDL, whose occurrences should be added to the subgraph.",
            typing: Type::string(),
            optional: false,
            tainting: true
        },
        field: {
            documentation: "A map of fields to index.",
            typing: SUBGRAPH_EVENT_FIELD.clone(),
            optional: false,
            tainting: true
        }
    };

    pub static ref SUBGRAPH_EVENT_FIELD: Type = define_strict_map_type! {
        name: {
            documentation: "The name of the field as it should appear in the subgraph.",
            typing: Type::string(),
            optional: false,
            tainting: true
        },
        description: {
            documentation: "A description of the field as it should appear in the subgraph schema.",
            typing: Type::string(),
            optional: true,
            tainting: false
        },
        idl_key: {
            documentation: "A key from the event's type in the IDL, indicating which argument from the IDL type to map to this field. By default, the field name is used.",
            typing: Type::string(),
            optional: true,
            tainting: true
        }
    };

    pub static ref SET_ACCOUNT_MAP: Type = define_strict_map_type! {
        public_key: {
            documentation: "The public key of the account to set.",
            typing: Type::addon(SVM_PUBKEY),
            optional: false,
            tainting: true
        },
        lamports: {
            documentation: "The amount of lamports the account should be set to have.",
            typing: Type::integer(),
            optional: true,
            tainting: true
        },
        data: {
            documentation: "The data to set in the account.",
            typing: Type::addon(SVM_BYTES),
            optional: true,
            tainting: true
        },
        owner: {
            documentation: "The owner to set for the account.",
            typing: Type::addon(SVM_PUBKEY),
            optional: true,
            tainting: true
        },
        executable: {
            documentation: "The executability state to set for the account.",
            typing: Type::bool(),
            optional: true,
            tainting: false
        },
        rent_epoch: {
            documentation: "The epoch at which the account will be rent-exempt.",
            typing: Type::integer(),
            optional: true,
            tainting: false
        }
    };

    pub static ref SET_TOKEN_ACCOUNT_MAP: Type = define_strict_map_type! {
        public_key: {
            documentation: "The public key of the token owner account to update.",
            typing: Type::addon(SVM_PUBKEY),
            optional: false,
            tainting: true
        },
        token: {
            documentation: "The token symbol or public key for the token.",
            typing: Type::addon(SVM_PUBKEY),
            optional: false,
            tainting: true
        },
        token_program: {
            documentation: "The token program id. Valid values are `token2020`, `token2022`, or a public key.",
            typing: Type::addon(SVM_PUBKEY),
            optional: true,
            tainting: true
        },
        amount: {
            documentation: "The amount of tokens to set.",
            typing: Type::integer(),
            optional: true,
            tainting: true
        },
        delegate: {
            documentation: "The public key of the delegate to set.",
            typing: Type::addon(SVM_PUBKEY),
            optional: true,
            tainting: true
        },
        delegated_amount: {
            documentation: "The amount of tokens to delegate.",
            typing: Type::integer(),
            optional: true,
            tainting: true
        },
        close_authority: {
            documentation: "The public key of the close authority to set.",
            typing: Type::addon(SVM_PUBKEY),
            optional: true,
            tainting: true
        },
        state: {
            documentation: "The state of the token account. Valid values are `initialized`, `frozen`, or `uninitialized`.",
            typing: Type::string(),
            optional: true,
            tainting: true
        }
    };

    pub static ref CLONE_PROGRAM_ACCOUNT: Type = define_strict_map_type! {
        source_program_id: {
            documentation: "The public key of the program to clone.",
            typing: Type::addon(SVM_PUBKEY),
            optional: false,
            tainting: true
        },
        destination_program_id: {
            documentation: "The destination public key of the program.",
            typing: Type::addon(SVM_PUBKEY),
            optional: false,
            tainting: true
        }
    };

    pub static ref SET_PROGRAM_AUTHORITY: Type = define_strict_map_type! {
        program_id: {
            documentation: "The public key of the program to set the authority for.",
            typing: Type::addon(SVM_PUBKEY),
            optional: false,
            tainting: true
        },
        authority: {
            documentation: "The new authority for the program. If not provided, program's authority will be set to None.",
            typing: Type::addon(SVM_PUBKEY),
            optional: true,
            tainting: true
        }
    };
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DeploymentTransactionType {
    CreateTempAuthority(Vec<u8>),
    CreateBuffer,
    WriteToBuffer,
    TransferBufferAuthority,
    TransferProgramAuthority,
    DeployProgram,
    UpgradeProgram,
    CloseTempAuthority,
    SkipCloseTempAuthority,
}

impl DeploymentTransactionType {
    pub fn to_string(&self) -> String {
        match self {
            DeploymentTransactionType::CreateTempAuthority(_) => "create_temp_authority",
            DeploymentTransactionType::CreateBuffer => "create_buffer",
            DeploymentTransactionType::WriteToBuffer => "write_to_buffer",
            DeploymentTransactionType::TransferBufferAuthority => "transfer_buffer_authority",
            DeploymentTransactionType::TransferProgramAuthority => "transfer_program_authority",
            DeploymentTransactionType::DeployProgram => "deploy_program",
            DeploymentTransactionType::UpgradeProgram => "upgrade_program",
            DeploymentTransactionType::CloseTempAuthority => "close_temp_authority",
            DeploymentTransactionType::SkipCloseTempAuthority => "skip_close_temp_authority",
        }
        .into()
    }
    pub fn from_string(s: &str) -> Self {
        match s {
            "create_temp_authority" => DeploymentTransactionType::CreateTempAuthority(vec![]),
            "create_buffer" => DeploymentTransactionType::CreateBuffer,
            "write_to_buffer" => DeploymentTransactionType::WriteToBuffer,
            "transfer_buffer_authority" => DeploymentTransactionType::TransferBufferAuthority,
            "deploy_program" => DeploymentTransactionType::DeployProgram,
            "upgrade_program" => DeploymentTransactionType::UpgradeProgram,
            "close_temp_authority" => DeploymentTransactionType::CloseTempAuthority,
            "skip_close_temp_authority" => DeploymentTransactionType::SkipCloseTempAuthority,
            "transfer_program_authority" => DeploymentTransactionType::TransferProgramAuthority,
            _ => unreachable!(),
        }
    }
}
