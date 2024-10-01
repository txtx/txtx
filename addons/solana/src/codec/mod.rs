pub mod anchor;
pub mod idl;

use bip39::Language;
use bip39::Mnemonic;
use bip39::MnemonicType;
use bip39::Seed;
use solana_client::rpc_client::RpcClient;
use solana_sdk::account_utils::StateMut;
use solana_sdk::bpf_loader_upgradeable::create_buffer;
use solana_sdk::bpf_loader_upgradeable::UpgradeableLoaderState;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::commitment_config::CommitmentLevel;
use solana_sdk::hash::Hash;
use solana_sdk::packet::PACKET_DATA_SIZE;
use solana_sdk::signature::keypair_from_seed;
use solana_sdk::signature::Keypair;
use solana_sdk::signature::Signature;
use solana_sdk::signer::Signer;
// use solana_sdk::loader_v4::finalize;
use solana_sdk::{
    bpf_loader_upgradeable, instruction::Instruction, message::Message, pubkey::Pubkey,
    transaction::Transaction,
};
use std::str::FromStr;
use txtx_addon_kit::types::types::Value;

use crate::typing::SolanaValue;

pub fn encode_contract_call(instructions: &Vec<Instruction>) -> Result<Value, String> {
    let message = Message::new(instructions, None);
    let message_bytes = message.serialize();
    Ok(Value::buffer(message_bytes))
}

pub fn public_key_from_bytes(bytes: &Vec<u8>) -> Result<Pubkey, String> {
    let bytes: [u8; 32] =
        bytes.as_slice().try_into().map_err(|e| format!("invalid public key: {e}"))?;
    Ok(Pubkey::new_from_array(bytes))
}

pub fn public_key_from_str(str: &str) -> Result<Pubkey, String> {
    Pubkey::from_str(str).map_err(|e| format!("invalid public key: {e}"))
}

pub struct UpgradeableProgramDeployer {
    /// The public key of the program to deploy.
    pub program_pubkey: Pubkey,
    /// The keypair of the program to deploy.
    pub program_keypair: Keypair,
    /// The public key of the payer.
    pub payer_pubkey: Pubkey,
    /// The public key of the upgrade authority. (Can be the same as the payer)
    pub upgrade_authority_pubkey: Pubkey,
    /// The keypair of the upgrade authority, or the public key of txtx signer that will be used.
    pub upgrade_authority: KeypairOrTxSigner,
    /// The public key of the buffer account. The buffer account exists to be a temporary address where the program is deployed.
    /// If there are failures in the deployment, the same buffer account can be provided to retry the deployment.
    pub buffer_pubkey: Pubkey,
    /// The keypair of the buffer account.
    pub buffer_keypair: Keypair,
    /// The data of the buffer account from the previous deployment attempt.
    pub buffer_data: Vec<u8>,
    /// The binary of the program to deploy.
    pub binary: Vec<u8>,
    /// The RPC client to use for fetching the latest blockhash and minimum balance for rent exemption.
    pub rpc_client: RpcClient,
    /// The commitment level to use for the deployment.
    pub commitment: CommitmentConfig,
}

pub enum KeypairOrTxSigner {
    Keypair(Keypair),
    TxSigner(Pubkey),
}

impl UpgradeableProgramDeployer {
    /// Creates a new instance with the provided parameters.
    ///
    /// # Parameters
    ///
    /// * `program_keypair` - The keypair for the program being deployed.
    /// * `upgrade_authority_keypair` - The keypair for the upgrade authority. (Can be the same as the payer)
    /// * `binary` - A reference to a vector of bytes representing the binary data.
    /// * `payer_pubkey` - The public key of the payer.
    /// * `rpc_client` - The RPC client to interact with the Solana network.
    /// * `commitment` - An optional commitment configuration. If `None`, the default commitment level is `Confirmed`.
    /// * `existing_program_buffer_opts` - An optional tuple containing:
    ///     * `Pubkey` - The public key of the existing program buffer.
    ///     * `Keypair` - The keypair associated with the existing program buffer.
    ///     * `Vec<u8>` - A vector of bytes representing the existing program buffer data. If `None`, a new program buffer will be created.
    pub fn new(
        program_keypair: Keypair,
        upgrade_authority: KeypairOrTxSigner,
        binary: &Vec<u8>,
        payer_pubkey: &Pubkey,
        rpc_client: RpcClient,
        commitment: Option<CommitmentConfig>,
        existing_program_buffer_opts: Option<(Pubkey, Keypair, Vec<u8>)>,
    ) -> Self {
        let (buffer_pubkey, buffer_keypair, buffer_data) = match existing_program_buffer_opts {
            Some((buffer_pubkey, buffer_keypair, buffer_data)) => {
                (buffer_pubkey, buffer_keypair, buffer_data)
            }
            None => {
                let (_buffer_words, _buffer_mnemonic, buffer_keypair) = create_ephemeral_keypair();
                (buffer_keypair.pubkey(), buffer_keypair, vec![0; binary.len()])
            }
        };

        let upgrade_authority_pubkey = match &upgrade_authority {
            KeypairOrTxSigner::Keypair(keypair) => keypair.pubkey(),
            KeypairOrTxSigner::TxSigner(pubkey) => pubkey.clone(),
        };

        Self {
            program_pubkey: program_keypair.pubkey(),
            program_keypair,
            upgrade_authority_pubkey,
            upgrade_authority,
            binary: binary.clone(),
            payer_pubkey: *payer_pubkey,
            rpc_client,
            commitment: commitment
                .unwrap_or(CommitmentConfig { commitment: CommitmentLevel::Confirmed }),
            buffer_keypair,
            buffer_pubkey,
            buffer_data,
        }
    }

    pub fn get_transactions(&self) -> Result<Vec<Value>, String> {
        let should_do_initial_deploy = should_do_initial_deploy(
            &self.program_pubkey,
            &self.upgrade_authority_pubkey,
            &self.commitment,
            &self.rpc_client,
        )?;

        let recent_blockhash = self
            .rpc_client
            .get_latest_blockhash()
            .map_err(|e| format!("failed to fetch latest blockhash: rpc error: {e}"))?;

        if should_do_initial_deploy {
            let create_account_transaction = get_create_account_transaction(
                &self.payer_pubkey,
                &self.buffer_pubkey,
                &self.buffer_keypair,
                &self.upgrade_authority_pubkey,
                &self.upgrade_authority,
                &self.binary,
                &self.rpc_client,
                &recent_blockhash,
            )?;

            let mut write_transactions = get_write_transactions(
                &self.buffer_pubkey,
                &self.upgrade_authority_pubkey,
                &self.upgrade_authority,
                &self.binary,
                &self.payer_pubkey,
                &recent_blockhash,
                &self.buffer_data,
            )?;
            let finalize_transaction = get_final_transaction(
                &self.payer_pubkey,
                &self.program_pubkey,
                &self.program_keypair,
                &self.buffer_pubkey,
                &self.upgrade_authority_pubkey,
                &self.upgrade_authority,
                &recent_blockhash,
                &self.rpc_client,
                &self.binary,
            )?;
            let mut transactions = vec![create_account_transaction];
            transactions.append(&mut write_transactions);
            transactions.push(finalize_transaction);
            return Ok(transactions);
        } else {
        }
        Ok(vec![])
    }
}

fn create_ephemeral_keypair() -> (usize, Mnemonic, Keypair) {
    const WORDS: usize = 12;
    let mnemonic = Mnemonic::new(MnemonicType::for_word_count(WORDS).unwrap(), Language::English);
    let seed = Seed::new(&mnemonic, "");
    let new_keypair = keypair_from_seed(seed.as_bytes()).unwrap();

    (WORDS, mnemonic, new_keypair)
}

pub fn get_create_account_transaction(
    payer_pubkey: &Pubkey,
    buffer_pubkey: &Pubkey,
    buffer_keypair: &Keypair,
    buffer_authority_pubkey: &Pubkey,
    buffer_authority_keypair: &KeypairOrTxSigner,
    binary: &Vec<u8>,
    rpc_client: &RpcClient,
    blockhash: &Hash,
) -> Result<Value, String> {
    let program_data_length = binary.len();

    let rent_lamports = rpc_client
        .get_minimum_balance_for_rent_exemption(UpgradeableLoaderState::size_of_programdata(
            program_data_length,
        ))
        .unwrap();

    let create_program_account_instruction = create_buffer(
        payer_pubkey,
        buffer_pubkey,
        buffer_authority_pubkey,
        rent_lamports,
        program_data_length,
    )
    .map_err(|e| format!("failed to create buffer: {e}"))?;

    let message = Message::new_with_blockhash(
        &create_program_account_instruction,
        Some(buffer_authority_pubkey),
        &blockhash,
    );

    let transaction = Transaction::new_unsigned(message);
    let transaction_bytes = serde_json::to_vec(&transaction)
        .map_err(|e| format!("failed to serialize transaction: {e}"))?;

    let (deferred_signer_pos, initial_signers) = match buffer_authority_keypair {
        KeypairOrTxSigner::Keypair(keypair) => (
            Some(vec![(1, payer_pubkey.clone())]),
            vec![(keypair.to_bytes().to_vec(), 0), (buffer_keypair.to_bytes().to_vec(), 2)],
        ),
        KeypairOrTxSigner::TxSigner(pubkey) => (
            Some(vec![(0, pubkey.clone()), (1, payer_pubkey.clone())]),
            vec![(buffer_keypair.to_bytes().to_vec(), 2)],
        ),
    };

    let transaction_with_partial_signer = SolanaValue::transaction_with_partial_signer(
        transaction_bytes,
        deferred_signer_pos,
        initial_signers,
    )
    .map_err(|e| e.message)?;
    Ok(transaction_with_partial_signer)
}

/// Logic mostly copied from solana cli: https://github.com/txtx/solana/blob/8116c10021f09c806159852f65d37ffe6d5a118e/cli/src/program.rs#L1248-L1249
fn should_do_initial_deploy(
    program_pubkey: &Pubkey,
    upgrade_authority_pubkey: &Pubkey,
    commitment: &CommitmentConfig,
    rpc_client: &RpcClient,
) -> Result<bool, String> {
    if let Some(account) = rpc_client
        .get_account_with_commitment(&program_pubkey, commitment.clone())
        .map_err(|e| format!("failed to get program account: {e}"))?
        .value
    {
        if account.owner != bpf_loader_upgradeable::id() {
            return Err(format!(
                "Account {program_pubkey} is not an upgradeable program or already in use"
            )
            .into());
        }
        if !account.executable {
            return Ok(true);
        } else if let Ok(UpgradeableLoaderState::Program { programdata_address }) = account.state()
        {
            if let Some(account) = rpc_client
                .get_account_with_commitment(&programdata_address, commitment.clone())
                .map_err(|e| format!("failed to get program data account: {e}"))?
                .value
            {
                if let Ok(UpgradeableLoaderState::ProgramData {
                    slot: _,
                    upgrade_authority_address: program_authority_pubkey,
                }) = account.state()
                {
                    if program_authority_pubkey.is_none() {
                        return Err(
                            format!("Program {program_pubkey} is no longer upgradeable").into()
                        );
                    }
                    if program_authority_pubkey.as_ref() != Some(upgrade_authority_pubkey) {
                        return Err(format!(
                            "Program's authority {:?} does not match authority provided {:?}",
                            program_authority_pubkey, upgrade_authority_pubkey,
                        )
                        .into());
                    }
                    // Do upgrade
                    return Ok(false);
                } else {
                    return Err(format!(
                        "Program {program_pubkey} has been closed, use a new Program Id"
                    )
                    .into());
                }
            } else {
                return Err(format!(
                    "Program {program_pubkey} has been closed, use a new Program Id"
                )
                .into());
            }
        } else {
            return Err(format!("{program_pubkey} is not an upgradeable program").into());
        }
    } else {
        return Ok(true);
    }
}

// Mostly copied from solana cli: https://github.com/txtx/solana/blob/8116c10021f09c806159852f65d37ffe6d5a118e/cli/src/program.rs#L2455
fn get_write_transactions(
    buffer_pubkey: &Pubkey,
    buffer_authority_pubkey: &Pubkey,
    upgrade_authority_keypair: &KeypairOrTxSigner,
    program_data: &Vec<u8>,
    payer_pubkey: &Pubkey,
    blockhash: &Hash,
    buffer_program_data: &[u8],
) -> Result<Vec<Value>, String> {
    let create_msg = |offset: u32, bytes: Vec<u8>| {
        let instruction =
            bpf_loader_upgradeable::write(buffer_pubkey, &buffer_authority_pubkey, offset, bytes);

        let instructions = vec![instruction];
        Message::new_with_blockhash(&instructions, Some(&payer_pubkey), &blockhash)
    };

    let mut write_transactions = vec![];
    let chunk_size = calculate_max_chunk_size(&create_msg);
    for (chunk, i) in program_data.chunks(chunk_size).zip(0usize..) {
        let offset = i.saturating_mul(chunk_size);
        if chunk != &buffer_program_data[offset..offset.saturating_add(chunk.len())] {
            let transaction = Transaction::new_unsigned(create_msg(offset as u32, chunk.to_vec()));
            let transaction_bytes = serde_json::to_vec(&transaction)
                .map_err(|e| format!("failed to serialize transaction: {e}"))
                .unwrap();

            let (deferred_signer_pos, initial_signers) = match upgrade_authority_keypair {
                KeypairOrTxSigner::Keypair(keypair) => {
                    (Some(vec![(0, payer_pubkey.clone())]), vec![(keypair.to_bytes().to_vec(), 1)])
                }
                KeypairOrTxSigner::TxSigner(pubkey) => {
                    (Some(vec![(0, payer_pubkey.clone()), (1, pubkey.clone())]), vec![])
                }
            };

            let transaction_with_partial_signer = SolanaValue::transaction_with_partial_signer(
                transaction_bytes,
                deferred_signer_pos,
                initial_signers,
            )
            .map_err(|e| e.message)?;
            write_transactions.push(transaction_with_partial_signer);
        }
    }
    Ok(write_transactions)
}

/// Copied from solana cli: https://github.com/txtx/solana/blob/8116c10021f09c806159852f65d37ffe6d5a118e/cli/src/program.rs#L2386
pub fn calculate_max_chunk_size<F>(create_msg: &F) -> usize
where
    F: Fn(u32, Vec<u8>) -> Message,
{
    let baseline_msg = create_msg(0, Vec::new());
    let tx_size = bincode::serialized_size(&Transaction {
        signatures: vec![
            Signature::default();
            baseline_msg.header.num_required_signatures as usize
        ],
        message: baseline_msg,
    })
    .unwrap() as usize;
    // add 1 byte buffer to account for shortvec encoding
    PACKET_DATA_SIZE.saturating_sub(tx_size).saturating_sub(1)
}

pub fn get_final_transaction(
    payer_pubkey: &Pubkey,
    program_pubkey: &Pubkey,
    program_keypair: &Keypair,
    buffer_pubkey: &Pubkey,
    upgrade_authority_pubkey: &Pubkey,
    upgrade_authority_keypair_or_signer: &KeypairOrTxSigner,
    blockhash: &Hash,
    rpc_client: &RpcClient,
    binary: &Vec<u8>,
) -> Result<Value, String> {
    let instructions = bpf_loader_upgradeable::deploy_with_max_program_len(
        &payer_pubkey,
        &program_pubkey,
        buffer_pubkey,
        &upgrade_authority_pubkey,
        rpc_client
            .get_minimum_balance_for_rent_exemption(UpgradeableLoaderState::size_of_program())
            .map_err(|e| format!("failed to get minimum balance for rent exemption: {e}"))?,
        binary.len(),
    )
    .map_err(|e| format!("failed to create deploy with max program len instruction: {e}"))?;

    let message = Message::new_with_blockhash(&instructions, Some(&payer_pubkey), &blockhash);
    let transaction = Transaction::new_unsigned(message);

    let transaction_bytes = serde_json::to_vec(&transaction)
        .map_err(|e| format!("failed to serialize transaction: {e}"))?;

    let (deferred_signer_pos, initial_signers) = match upgrade_authority_keypair_or_signer {
        KeypairOrTxSigner::Keypair(upgrade_authority_keypair) => (
            Some(vec![(0, payer_pubkey.clone())]),
            vec![
                (program_keypair.to_bytes().to_vec(), 1),
                (upgrade_authority_keypair.to_bytes().to_vec(), 2),
            ],
        ),
        KeypairOrTxSigner::TxSigner(upgrade_authority_pubkey) => (
            Some(vec![(0, payer_pubkey.clone()), (2, upgrade_authority_pubkey.clone())]),
            vec![(program_keypair.to_bytes().to_vec(), 1)],
        ),
    };

    let transaction_with_partial_signer = SolanaValue::transaction_with_partial_signer(
        transaction_bytes,
        deferred_signer_pos,
        initial_signers,
    )
    .map_err(|e| e.message)?;

    Ok(transaction_with_partial_signer)
}
