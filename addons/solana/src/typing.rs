use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    types::{Type, Value},
};

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
pub const SOLANA_TRANSACTION_PARTIAL_SIGNERS: &str = "solana::transaction_partial_signers";
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

    pub fn transaction_with_partial_signer(
        transaction_bytes: Vec<u8>,
        deferred_signer_pos: Option<Vec<(usize, Pubkey)>>,
        initial_signers: Vec<(Vec<u8>, usize)>,
    ) -> Result<Value, Diagnostic> {
        let partial_signer =
            PartialSigner::new(deferred_signer_pos, initial_signers, transaction_bytes);
        let bytes = serde_json::to_vec(&partial_signer)
            .map_err(|e| diagnosed_error!("failed to serialize partial signer: {}", e))?;

        Ok(Value::addon(bytes, SOLANA_TRANSACTION_PARTIAL_SIGNERS))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialSigner {
    pub deferred_signer_pos: Option<Vec<(usize, Pubkey)>>,
    pub signers: Vec<Option<Vec<u8>>>,
    pub transaction_bytes: Vec<u8>,
}
impl PartialSigner {
    pub fn new(
        deferred_signer_pos: Option<Vec<(usize, Pubkey)>>,
        initial_signers: Vec<(Vec<u8>, usize)>,
        transaction_bytes: Vec<u8>,
    ) -> Self {
        let mut signers = vec![
            None;
            deferred_signer_pos.as_ref().and_then(|v| Some(v.len())).unwrap_or(0)
                + initial_signers.len()
        ];

        for (keypair_bytes, pos) in initial_signers {
            signers[pos] = Some(keypair_bytes);
        }

        if let Some(deferred_signer_pos) = deferred_signer_pos.as_ref() {
            for (pos, _) in deferred_signer_pos.iter() {
                if signers.get(*pos).unwrap().is_some() {
                    panic!("Signer at position {} is already set", pos);
                }
                signers[*pos] = None;
            }
        }
        Self { deferred_signer_pos, signers, transaction_bytes }
    }

    pub fn fill_signer(&mut self, pubkey: Pubkey, keypair_bytes: &Vec<u8>) {
        let empty_vec = vec![];
        let positions = self
            .deferred_signer_pos
            .as_ref()
            .unwrap_or(&empty_vec)
            .iter()
            .filter_map(|(pos, p)| if p == &pubkey { Some(*pos) } else { None })
            .collect::<Vec<usize>>();
        for pos in positions.iter() {
            self.signers[*pos] = Some(keypair_bytes.clone());
        }
    }

    pub fn expect_signers(self) -> Vec<Keypair> {
        self.signers
            .iter()
            .map(|bytes| Keypair::from_bytes(&bytes.as_ref().unwrap()).unwrap())
            .collect::<Vec<_>>()
    }
}

lazy_static! {
    pub static ref ANCHOR_PROGRAM_ARTIFACTS: Type = define_object_type! {
        idl: {
            documentation: "The program idl.",
            typing: Type::addon(SOLANA_IDL),
            optional: false,
            tainting: true
        },
        binary: {
            documentation: "The program binary.",
            typing: Type::addon(SOLANA_BINARY),
            optional: false,
            tainting: true
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
