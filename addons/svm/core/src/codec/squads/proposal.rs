use borsh::{BorshDeserialize, BorshSerialize};
use solana_sdk::pubkey::Pubkey;
use txtx_addon_kit::types::diagnostics::Diagnostic;

use crate::codec::idl::convert_idl::compute_discriminator;

pub const CREATE_PROPOSAL_FN_NAME: &str = "proposal_create";
pub const CREATE_PROPOSAL_DISCRIMINATOR: [u8; 8] = [220, 60, 73, 224, 30, 108, 79, 159];

#[derive(BorshSerialize, Eq, PartialEq, Clone)]
struct ProposalCreateArgs {
    /// Index of the multisig transaction this proposal is associated with.
    pub transaction_index: u64,
    /// Whether the proposal should be initialized with status `Draft`.
    pub draft: bool,
}

pub fn get_proposal_ix_data(transaction_index: u64, draft: bool) -> Vec<u8> {
    let args = ProposalCreateArgs { transaction_index, draft };
    let mut data = vec![];
    data.extend_from_slice(&CREATE_PROPOSAL_DISCRIMINATOR);
    args.serialize(&mut data).expect("failed to serialize proposal create args");
    data
}

#[derive(BorshDeserialize, Clone, PartialEq, Eq, Debug)]
pub struct Proposal {
    /// The multisig this belongs to.
    pub multisig: Pubkey,
    /// Index of the multisig transaction this proposal is associated with.
    pub transaction_index: u64,
    /// The status of the transaction.
    pub status: ProposalStatus,
    /// PDA bump.
    pub bump: u8,
    /// Keys that have approved/signed.
    pub approved: Vec<Pubkey>,
    /// Keys that have rejected.
    pub rejected: Vec<Pubkey>,
    /// Keys that have cancelled (Approved only).
    pub cancelled: Vec<Pubkey>,
}
impl Proposal {
    pub fn checked_deserialize(data: &[u8]) -> Result<Self, Diagnostic> {
        if data.len() < 8 {
            return Err(diagnosed_error!(
                "invalid Proposal account: too short, expected at least 8 bytes"
            ));
        }
        let discriminator = &data[0..8];
        if discriminator != compute_discriminator("account", "Proposal") {
            return Err(diagnosed_error!(
                "invalid Proposal account: incorrect account discriminator"
            ));
        }
        let mut account_data = &data[8..];
        Proposal::deserialize(&mut account_data)
            .map_err(|e| diagnosed_error!("invalid Proposal account data: {e}"))
    }
}

/// The status of a proposal.
/// Each variant wraps a timestamp of when the status was set.
#[allow(deprecated)]
#[derive(BorshDeserialize, Clone, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum ProposalStatus {
    /// Proposal is in the draft mode and can be voted on.
    Draft { timestamp: i64 },
    /// Proposal is live and ready for voting.
    Active { timestamp: i64 },
    /// Proposal has been rejected.
    Rejected { timestamp: i64 },
    /// Proposal has been approved and is pending execution.
    Approved { timestamp: i64 },
    /// Proposal is being executed. This is a transient state that always transitions to `Executed` in the span of a single transaction.
    #[deprecated(
        note = "This status used to be used to prevent reentrancy attacks. It is no longer needed."
    )]
    Executing,
    /// Proposal has been executed.
    Executed { timestamp: i64 },
    /// Proposal has been cancelled.
    Cancelled { timestamp: i64 },
}
