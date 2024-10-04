pub mod anchor;
pub mod idl;

use bip39::Language;
use bip39::Mnemonic;
use bip39::MnemonicType;
use bip39::Seed;
use solana_client::rpc_client::RpcClient;
use solana_sdk::account_utils::StateMut;
use solana_sdk::bpf_loader_upgradeable::create_buffer;
use solana_sdk::bpf_loader_upgradeable::get_program_data_address;
use solana_sdk::bpf_loader_upgradeable::UpgradeableLoaderState;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::commitment_config::CommitmentLevel;
use solana_sdk::hash::Hash;
use solana_sdk::packet::PACKET_DATA_SIZE;
use solana_sdk::signature::keypair_from_seed;
use solana_sdk::signature::Keypair;
use solana_sdk::signature::Signature;
use solana_sdk::signer::Signer;
use solana_sdk::system_instruction::MAX_PERMITTED_DATA_LENGTH;
// use solana_sdk::loader_v4::finalize;
use solana_sdk::{
    bpf_loader_upgradeable, instruction::Instruction, message::Message, pubkey::Pubkey,
    transaction::Transaction,
};
use std::str::FromStr;
use txtx_addon_kit::types::types::Value;

use crate::typing::SvmValue;

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
    /// Whether to auto extend the program data account if it is too small to accommodate the new program.
    pub auto_extend: bool,
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
        auto_extend: Option<bool>,
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
            auto_extend: auto_extend.unwrap_or(true),
        }
    }

    pub fn get_transactions(&self) -> Result<Vec<Value>, String> {
        let recent_blockhash = self
            .rpc_client
            .get_latest_blockhash()
            .map_err(|e| format!("failed to fetch latest blockhash: rpc error: {e}"))?;

        let transactions = if self.should_do_initial_deploy()? {
            let create_account_transaction =
                self.get_create_buffer_transaction(&recent_blockhash)?;

            let mut write_transactions =
                self.get_write_to_buffer_transactions(&recent_blockhash)?;

            let finalize_transaction =
                self.get_deploy_with_max_program_len_transaction(&recent_blockhash)?;

            let mut transactions = vec![create_account_transaction];
            transactions.append(&mut write_transactions);
            transactions.push(finalize_transaction);
            transactions
        } else {
            // upgrading an existing program
            let prepare_program_upgrade_transaction =
                self.get_prepare_program_upgrade_transaction(&recent_blockhash)?;

            let mut write_transactions =
                self.get_write_to_buffer_transactions(&recent_blockhash)?;

            let upgrade_transaction = self.get_upgrade_transaction(&recent_blockhash)?;

            let mut transactions = vec![prepare_program_upgrade_transaction];
            transactions.append(&mut write_transactions);
            transactions.push(upgrade_transaction);
            transactions
        };
        Ok(transactions)
    }

    fn get_create_buffer_instruction(&self) -> Result<Vec<Instruction>, String> {
        let program_data_length = self.binary.len();

        let rent_lamports = self
            .rpc_client
            .get_minimum_balance_for_rent_exemption(UpgradeableLoaderState::size_of_programdata(
                program_data_length,
            ))
            .unwrap();

        create_buffer(
            &self.payer_pubkey,
            &self.buffer_pubkey,
            &self.upgrade_authority_pubkey,
            rent_lamports,
            program_data_length,
        )
        .map_err(|e| format!("failed to create buffer: {e}"))
    }

    fn get_create_buffer_transaction(&self, blockhash: &Hash) -> Result<Value, String> {
        let create_buffer_instruction = self.get_create_buffer_instruction()?;

        let message = Message::new_with_blockhash(
            &create_buffer_instruction,
            Some(&self.upgrade_authority_pubkey),
            &blockhash,
        );

        let mut transaction = Transaction::new_unsigned(message);

        let available_keypairs = match &self.upgrade_authority {
            KeypairOrTxSigner::Keypair(keypair) => vec![keypair, &self.buffer_keypair],
            KeypairOrTxSigner::TxSigner(_) => vec![&self.buffer_keypair],
        };
        transaction.try_partial_sign(&available_keypairs, blockhash.clone()).map_err(|e| {
            format!("failed to sign transaction to create program buffer account: {e}")
        })?;
        let transaction_bytes = serde_json::to_vec(&transaction)
            .map_err(|e| format!("failed to serialize transaction: {e}"))?;

        Ok(SvmValue::transaction(transaction_bytes))
    }

    // Mostly copied from solana cli: https://github.com/txtx/solana/blob/8116c10021f09c806159852f65d37ffe6d5a118e/cli/src/program.rs#L2807
    fn get_extend_program_instruction(&self) -> Result<Option<Instruction>, String> {
        let program_data_address = get_program_data_address(&self.program_pubkey);

        let Some(program_data_account) = self
            .rpc_client
            .get_account_with_commitment(&program_data_address, self.commitment)
            .map_err(|e| format!("failed to get program data account: {e}",))?
            .value
        else {
            // Program data has not been allocated yet.
            return Ok(None);
        };

        let program_len = self.binary.len();
        let required_len = UpgradeableLoaderState::size_of_programdata(program_len);
        let max_permitted_data_length = usize::try_from(MAX_PERMITTED_DATA_LENGTH).unwrap();
        if required_len > max_permitted_data_length {
            let max_program_len = max_permitted_data_length
                .saturating_sub(UpgradeableLoaderState::size_of_programdata(0));
            return Err(format!(
                "New program ({}) data account is too big: {}.\n\
             Maximum program size: {}.",
                &self.program_pubkey, required_len, max_program_len
            )
            .into());
        }

        let current_len = program_data_account.data.len();
        let additional_bytes = required_len.saturating_sub(current_len);
        if additional_bytes == 0 {
            // Current allocation is sufficient.
            return Ok(None);
        }

        let additional_bytes =
            u32::try_from(additional_bytes).expect("`u32` is big enough to hold an account size");
        let instruction = bpf_loader_upgradeable::extend_program(
            &self.program_pubkey,
            Some(&self.upgrade_authority_pubkey),
            additional_bytes,
        );

        Ok(Some(instruction))
    }

    fn get_prepare_program_upgrade_transaction(&self, blockhash: &Hash) -> Result<Value, String> {
        let mut instructions = self.get_create_buffer_instruction()?;
        if self.auto_extend {
            if let Some(extend_program_instruction) = self.get_extend_program_instruction()? {
                println!("Extending program data account to accommodate new program...");
                instructions.push(extend_program_instruction);
            } else {
                println!("Program data account is already large enough for new program.");
            };
        }

        let message = Message::new_with_blockhash(
            &instructions,
            Some(&self.upgrade_authority_pubkey),
            &blockhash,
        );

        let mut transaction = Transaction::new_unsigned(message);

        let available_keypairs = match &self.upgrade_authority {
            KeypairOrTxSigner::Keypair(keypair) => vec![keypair, &self.buffer_keypair],
            KeypairOrTxSigner::TxSigner(_) => vec![&self.buffer_keypair],
        };
        transaction
            .try_partial_sign(&available_keypairs, blockhash.clone())
            .map_err(|e| format!("failed to sign transaction to prepare program upgrade: {e}"))?;
        let transaction_bytes = serde_json::to_vec(&transaction)
            .map_err(|e| format!("failed to serialize transaction: {e}"))?;

        Ok(SvmValue::transaction(transaction_bytes))
    }

    // Mostly copied from solana cli: https://github.com/txtx/solana/blob/8116c10021f09c806159852f65d37ffe6d5a118e/cli/src/program.rs#L2455
    fn get_write_to_buffer_transactions(&self, blockhash: &Hash) -> Result<Vec<Value>, String> {
        let create_msg = |offset: u32, bytes: Vec<u8>| {
            let instruction = bpf_loader_upgradeable::write(
                &self.buffer_pubkey,
                &self.upgrade_authority_pubkey,
                offset,
                bytes,
            );

            let instructions = vec![instruction];
            Message::new_with_blockhash(&instructions, Some(&self.payer_pubkey), &blockhash)
        };

        let mut write_transactions = vec![];
        let chunk_size = calculate_max_chunk_size(&create_msg);
        for (chunk, i) in self.binary.chunks(chunk_size).zip(0usize..) {
            let offset = i.saturating_mul(chunk_size);
            // Only write the chunk if it differs from our initial buffer data
            if chunk != &self.buffer_data[offset..offset.saturating_add(chunk.len())] {
                let mut transaction =
                    Transaction::new_unsigned(create_msg(offset as u32, chunk.to_vec()));

                let available_keypairs = match &self.upgrade_authority {
                    KeypairOrTxSigner::Keypair(keypair) => vec![keypair],
                    KeypairOrTxSigner::TxSigner(_) => vec![],
                };

                transaction
                    .try_partial_sign(&available_keypairs, blockhash.clone())
                    .map_err(|e| format!("failed to sign transaction to write program data to buffer account: {e}"))?;
                let transaction_bytes = serde_json::to_vec(&transaction)
                    .map_err(|e| format!("failed to serialize transaction: {e}"))?;

                write_transactions.push(SvmValue::transaction(transaction_bytes));
            }
        }
        Ok(write_transactions)
    }

    fn get_deploy_with_max_program_len_transaction(
        &self,
        blockhash: &Hash,
    ) -> Result<Value, String> {
        let instructions = bpf_loader_upgradeable::deploy_with_max_program_len(
            &self.payer_pubkey,
            &self.program_pubkey,
            &self.buffer_pubkey,
            &self.upgrade_authority_pubkey,
            self.rpc_client
                .get_minimum_balance_for_rent_exemption(UpgradeableLoaderState::size_of_program())
                .map_err(|e| format!("failed to get minimum balance for rent exemption: {e}"))?,
            self.binary.len(),
        )
        .map_err(|e| format!("failed to create deploy with max program len instruction: {e}"))?;

        let message =
            Message::new_with_blockhash(&instructions, Some(&self.payer_pubkey), &blockhash);
        let mut transaction = Transaction::new_unsigned(message);

        let available_keypairs = match &self.upgrade_authority {
            KeypairOrTxSigner::Keypair(keypair) => vec![&self.program_keypair, keypair],
            KeypairOrTxSigner::TxSigner(_) => vec![&self.program_keypair],
        };

        transaction
            .try_partial_sign(&available_keypairs, blockhash.clone())
            .map_err(|e| format!("failed to sign transaction to deploy program: {e}"))?;

        let transaction_bytes = serde_json::to_vec(&transaction)
            .map_err(|e| format!("failed to serialize transaction: {e}"))?;

        Ok(SvmValue::transaction(transaction_bytes))
    }

    fn get_upgrade_transaction(&self, blockhash: &Hash) -> Result<Value, String> {
        let upgrade_instruction = bpf_loader_upgradeable::upgrade(
            &self.program_pubkey,
            &self.buffer_pubkey,
            &self.upgrade_authority_pubkey,
            &self.payer_pubkey,
        );

        let message = Message::new_with_blockhash(
            &[upgrade_instruction],
            Some(&self.payer_pubkey),
            &blockhash,
        );
        let mut transaction = Transaction::new_unsigned(message);

        let available_keypairs = match &self.upgrade_authority {
            KeypairOrTxSigner::Keypair(keypair) => vec![keypair],
            KeypairOrTxSigner::TxSigner(_) => vec![],
        };

        transaction
            .try_partial_sign(&available_keypairs, blockhash.clone())
            .map_err(|e| format!("failed to sign transaction to upgrade program: {e}"))?;

        let transaction_bytes = serde_json::to_vec(&transaction)
            .map_err(|e| format!("failed to serialize transaction: {e}"))?;

        Ok(SvmValue::transaction(transaction_bytes))
    }
    /// Logic mostly copied from solana cli: https://github.com/txtx/solana/blob/8116c10021f09c806159852f65d37ffe6d5a118e/cli/src/program.rs#L1248-L1249
    fn should_do_initial_deploy(&self) -> Result<bool, String> {
        if let Some(account) = self
            .rpc_client
            .get_account_with_commitment(&self.program_pubkey, self.commitment.clone())
            .map_err(|e| format!("failed to get program account: {e}"))?
            .value
        {
            if account.owner != bpf_loader_upgradeable::id() {
                return Err(format!(
                    "Account {} is not an upgradeable program or already in use",
                    self.program_pubkey
                )
                .into());
            }
            if !account.executable {
                return Ok(true);
            } else if let Ok(UpgradeableLoaderState::Program { programdata_address }) =
                account.state()
            {
                if let Some(account) = self
                    .rpc_client
                    .get_account_with_commitment(&programdata_address, self.commitment.clone())
                    .map_err(|e| format!("failed to get program data account: {e}"))?
                    .value
                {
                    if let Ok(UpgradeableLoaderState::ProgramData {
                        slot: _,
                        upgrade_authority_address: program_authority_pubkey,
                    }) = account.state()
                    {
                        if let Some(program_authority_pubkey) = program_authority_pubkey {
                            if program_authority_pubkey != self.upgrade_authority_pubkey {
                                return Err(format!(
                                    "Program's authority {:?} does not match authority provided {:?}",
                                    program_authority_pubkey, self.upgrade_authority_pubkey,
                                )
                                .into());
                            }
                        }
                        // Do upgrade
                        return Ok(false);
                    } else {
                        return Err(format!(
                            "Program {} has been closed, use a new Program Id",
                            self.program_pubkey
                        )
                        .into());
                    }
                } else {
                    return Err(format!(
                        "Program {} has been closed, use a new Program Id",
                        self.program_pubkey
                    )
                    .into());
                }
            } else {
                return Err(format!("{} is not an upgradeable program", self.program_pubkey).into());
            }
        } else {
            return Ok(true);
        }
    }
}

fn create_ephemeral_keypair() -> (usize, Mnemonic, Keypair) {
    const WORDS: usize = 12;
    let mnemonic = Mnemonic::new(MnemonicType::for_word_count(WORDS).unwrap(), Language::English);
    let seed = Seed::new(&mnemonic, "");
    let new_keypair = keypair_from_seed(seed.as_bytes()).unwrap();

    (WORDS, mnemonic, new_keypair)
}

/// Copied from solana cli: https://github.com/txtx/solana/blob/8116c10021f09c806159852f65d37ffe6d5a118e/cli/src/program.rs#L2386
fn calculate_max_chunk_size<F>(create_msg: &F) -> usize
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
