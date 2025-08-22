use borsh::BorshDeserialize;
use solana_sdk::pubkey::Pubkey;
use txtx_addon_kit::types::diagnostics::Diagnostic;

use crate::codec::idl::convert_idl::compute_discriminator;

#[derive(BorshDeserialize, Eq, PartialEq, Clone, Debug)]
pub struct Multisig {
    /// Key that is used to seed the multisig PDA.
    pub create_key: Pubkey,
    /// The authority that can change the multisig config.
    /// This is a very important parameter as this authority can change the members and threshold.
    ///
    /// The convention is to set this to `Pubkey::default()`.
    /// In this case, the multisig becomes autonomous, so every config change goes through
    /// the normal process of voting by the members.
    ///
    /// However, if this parameter is set to any other key, all the config changes for this multisig
    /// will need to be signed by the `config_authority`. We call such a multisig a "controlled multisig".
    pub config_authority: Pubkey,
    /// Threshold for signatures.
    pub threshold: u16,
    /// How many seconds must pass between transaction voting settlement and execution.
    pub time_lock: u32,
    /// Last transaction index. 0 means no transactions have been created.
    pub transaction_index: u64,
    /// Last stale transaction index. All transactions up until this index are stale.
    /// This index is updated when multisig config (members/threshold/time_lock) changes.
    pub stale_transaction_index: u64,
    /// The address where the rent for the accounts related to executed, rejected, or cancelled
    /// transactions can be reclaimed. If set to `None`, the rent reclamation feature is turned off.
    pub rent_collector: Option<Pubkey>,
    /// Bump for the multisig PDA seed.
    pub bump: u8,
    /// Members of the multisig.
    pub members: Vec<Member>,
}

impl Multisig {
    pub fn checked_deserialize(data: &[u8]) -> Result<Self, Diagnostic> {
        if data.len() < 8 {
            return Err(diagnosed_error!(
                "invalid Multisig account: too short, expected at least 8 bytes"
            ));
        }
        let discriminator = &data[0..8];
        if discriminator != compute_discriminator("account", "Multisig") {
            return Err(diagnosed_error!(
                "invalid Multisig account: incorrect account discriminator"
            ));
        }
        let mut account_data = &data[8..];
        Multisig::deserialize(&mut account_data)
            .map_err(|e| diagnosed_error!("invalid Multisig account data: {e}"))
    }
}

#[derive(BorshDeserialize, Eq, PartialEq, Clone, Debug)]
pub struct Member {
    pub key: Pubkey,
    pub permissions: Permissions,
}

#[derive(Clone, Copy)]
pub enum Permission {
    Initiate = 1 << 0,
    Vote = 1 << 1,
    Execute = 1 << 2,
}

#[derive(BorshDeserialize, Eq, PartialEq, Clone, Copy, Default, Debug)]
pub struct Permissions {
    pub mask: u8,
}

impl Permissions {
    /// Currently unused.
    pub fn from_vec(permissions: &[Permission]) -> Self {
        let mut mask = 0;
        for permission in permissions {
            mask |= *permission as u8;
        }
        Self { mask }
    }

    pub fn has(&self, permission: Permission) -> bool {
        self.mask & (permission as u8) != 0
    }
}
