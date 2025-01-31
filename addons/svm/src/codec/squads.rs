use std::str::FromStr;

use borsh::{io, BorshDeserialize, BorshSerialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{account_info::AccountInfo, instruction::Instruction, pubkey::Pubkey};
use txtx_addon_kit::types::diagnostics::Diagnostic;

lazy_static::lazy_static! {
    static ref SQUADS_PROGRAM_ID: Pubkey =
        Pubkey::from_str("SQDS4ep65T869zMMBKyuUq6aD6EgTu8psMjkvj52pCf").unwrap();
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct ProposalCreateArgs {
    /// Index of the multisig transaction this proposal is associated with.
    pub transaction_index: u64,
    /// Whether the proposal should be initialized with status `Draft`.
    pub draft: bool,
}

const SEED_PREFIX: &str = "multisig";
const SEED_MULTISIG: &str = "multisig";
pub struct SquadsMultisig {
    pub program_id: Pubkey,
    pub create_key: Option<String>,
    pub bump_seed: Option<u8>,
    pub rpc_client: RpcClient,
}
impl SquadsMultisig {
    pub fn from_program_id(program_id: Pubkey, rpc_client: RpcClient) -> Self {
        Self { program_id, create_key: None, bump_seed: None, rpc_client }
    }

    pub fn from_create_key(create_key: String, rpc_client: RpcClient) -> Result<Self, Diagnostic> {
        let seed_refs = [SEED_PREFIX.as_bytes(), SEED_MULTISIG.as_bytes(), create_key.as_bytes()];
        let Some((program_id, bump_seed)) =
            Pubkey::try_find_program_address(&seed_refs, &SQUADS_PROGRAM_ID)
        else {
            return Err(diagnosed_error!(
                "invalid create key for squads signer: failed to find multisig pda"
            ));
        };
        Ok(Self {
            program_id,
            create_key: Some(create_key),
            bump_seed: Some(bump_seed),
            rpc_client,
        })
    }

    pub fn get_multisig_account_info(&self) -> Result<AccountInfo, Diagnostic> {
        todo!()
    }

    pub fn get_next_proposal_account(&self) -> Result<Pubkey, Diagnostic> {
        todo!()
    }

    pub fn get_next_proposal_account_info(&self) -> Result<AccountInfo, Diagnostic> {
        todo!()
    }

    pub fn get_next_vault_transaction_account(&self) -> Result<Pubkey, Diagnostic> {
        todo!()
    }

    pub fn get_next_vault_transaction_account_info(&self) -> Result<AccountInfo, Diagnostic> {
        todo!()
    }
}
