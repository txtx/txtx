use borsh::BorshSerialize;
use solana_sdk::{
    address_lookup_table,
    instruction::{AccountMeta, Instruction},
    message::{AccountKeys, AddressLookupTableAccount, Message},
    pubkey::Pubkey,
};
use txtx_addon_kit::types::diagnostics::Diagnostic;

use super::{compiled_keys::CompiledKeys, small_vec::SmallVec};

pub const CREATE_VAULT_TRANSACTION_DISCRIMINATOR: [u8; 8] = [48, 250, 78, 168, 208, 226, 218, 211];

#[derive(BorshSerialize, Eq, PartialEq, Clone)]
pub struct VaultTransactionCreateArgs {
    /// Index of the vault this transaction belongs to.
    pub vault_index: u8,
    /// Number of ephemeral signing PDAs required by the transaction.
    pub ephemeral_signers: u8,
    pub transaction_message: Vec<u8>,
    pub memo: Option<String>,
}

impl TransactionMessage {
    pub fn try_compile(
        vault_key: &Pubkey,
        instructions: &[Instruction],
        address_lookup_table_accounts: &[AddressLookupTableAccount],
    ) -> Result<Self, Diagnostic> {
        let mut compiled_keys = CompiledKeys::compile(&instructions, Some(*vault_key));

        let mut address_table_lookups = Vec::with_capacity(address_lookup_table_accounts.len());
        let mut loaded_addresses_list = Vec::with_capacity(address_lookup_table_accounts.len());
        for lookup_table_account in address_lookup_table_accounts {
            if let Some((lookup, loaded_addresses)) = compiled_keys
                .try_extract_table_lookup(lookup_table_account)
                .map_err(|e| diagnosed_error!("failed to extract address table lookup: {e}"))?
            {
                address_table_lookups.push(lookup);
                loaded_addresses_list.push(loaded_addresses);
            }
        }

        let (header, static_keys) = compiled_keys
            .try_into_message_components()
            .map_err(|e| diagnosed_error!("failed to compile transaction message: {e}"))?;
        let dynamic_keys = loaded_addresses_list.into_iter().collect();
        let account_keys = AccountKeys::new(&static_keys, Some(&dynamic_keys));
        let instructions = account_keys
            .try_compile_instructions(instructions)
            .map_err(|e| diagnosed_error!("failed to compile transaction instructions: {e}"))?;

        let num_static_keys: u8 = static_keys
            .len()
            .try_into()
            .map_err(|_| diagnosed_error!("failed to convert static keys length to u8"))?;

        Ok(TransactionMessage {
            num_signers: header.num_required_signatures,
            num_writable_signers: header.num_required_signatures
                - header.num_readonly_signed_accounts,
            num_writable_non_signers: num_static_keys
                - header.num_required_signatures
                - header.num_readonly_unsigned_accounts,
            account_keys: static_keys.into(),
            instructions: instructions
                .into_iter()
                .map(|ix| CompiledInstruction {
                    program_id_index: ix.program_id_index,
                    account_indexes: ix.accounts.into(),
                    data: ix.data.into(),
                })
                .collect::<Vec<CompiledInstruction>>()
                .into(),
            address_table_lookups: address_table_lookups
                .into_iter()
                .map(|lookup| MessageAddressTableLookup {
                    account_key: lookup.account_key,
                    writable_indexes: lookup.writable_indexes.into(),
                    readonly_indexes: lookup.readonly_indexes.into(),
                })
                .collect::<Vec<MessageAddressTableLookup>>()
                .into(),
        })
    }
}

/// Unvalidated instruction data, must be treated as untrusted.
#[derive(BorshSerialize, Clone, Debug)]
pub struct TransactionMessage {
    /// The number of signer pubkeys in the account_keys vec.
    pub num_signers: u8,
    /// The number of writable signer pubkeys in the account_keys vec.
    pub num_writable_signers: u8,
    /// The number of writable non-signer pubkeys in the account_keys vec.
    pub num_writable_non_signers: u8,
    /// The list of unique account public keys (including program IDs) that will be used in the provided instructions.
    pub account_keys: SmallVec<u8, Pubkey>,
    /// The list of instructions to execute.
    pub instructions: SmallVec<u8, CompiledInstruction>,
    /// List of address table lookups used to load additional accounts
    /// for this transaction.
    pub address_table_lookups: SmallVec<u8, MessageAddressTableLookup>,
}

// Concise serialization schema for instructions that make up transaction.
#[derive(BorshSerialize, Clone, Debug)]
pub struct CompiledInstruction {
    pub program_id_index: u8,
    /// Indices into the tx's `account_keys` list indicating which accounts to pass to the instruction.
    pub account_indexes: SmallVec<u8, u8>,
    /// Instruction data.
    pub data: SmallVec<u16, u8>,
}

/// Address table lookups describe an on-chain address lookup table to use
/// for loading more readonly and writable accounts in a single tx.
#[derive(BorshSerialize, Clone, Debug)]
pub struct MessageAddressTableLookup {
    /// Address lookup table account key
    pub account_key: Pubkey,
    /// List of indexes used to load writable account addresses
    pub writable_indexes: SmallVec<u8, u8>,
    /// List of indexes used to load readonly account addresses
    pub readonly_indexes: SmallVec<u8, u8>,
}

pub fn get_create_vault_transaction_ix_data(
    vault_key: &Pubkey,
    vault_index: u8,
    ephemeral_signers: u8,
    message: Message,
    memo: Option<String>,
) -> Result<Vec<u8>, Diagnostic> {
    let mut ixs = vec![];
    for instruction in message.instructions.iter() {
        ixs.push(Instruction {
            program_id: message.account_keys[instruction.program_id_index as usize],
            accounts: instruction
                .accounts
                .iter()
                .map(|index| {
                    let index = *index as usize;

                    AccountMeta {
                        pubkey: message.account_keys[index],
                        is_signer: message.is_signer(index),
                        is_writable: message.is_writable(index),
                    }
                })
                .collect(),
            data: instruction.data.to_vec(),
        });
    }

    let transaction_message = TransactionMessage::try_compile(vault_key, &ixs, &[])?;

    let mut message_bytes = vec![];
    transaction_message
        .serialize(&mut message_bytes)
        .expect("failed to serialize transaction message");
    let args = VaultTransactionCreateArgs {
        vault_index,
        ephemeral_signers,
        transaction_message: message_bytes,
        memo,
    };
    let mut data = vec![];
    data.extend_from_slice(&CREATE_VAULT_TRANSACTION_DISCRIMINATOR);
    args.serialize(&mut data).expect("failed to serialize proposal create args");
    Ok(data)
}
