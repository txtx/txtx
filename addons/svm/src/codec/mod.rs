pub mod anchor;
pub mod idl;
pub mod instruction;
pub mod send_transaction;

use bip39::Language;
use bip39::Mnemonic;
use bip39::MnemonicType;
use bip39::Seed;
use serde::Deserialize;
use serde::Serialize;
use solana_client::rpc_client::RpcClient;
use solana_sdk::account::AccountSharedData;
use solana_sdk::account_utils::StateMut;
use solana_sdk::bpf_loader_upgradeable::create_buffer;
use solana_sdk::bpf_loader_upgradeable::get_program_data_address;
use solana_sdk::bpf_loader_upgradeable::UpgradeableLoaderState;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::commitment_config::CommitmentLevel;
use solana_sdk::hash::Hash;
use solana_sdk::nonce_account::lamports_per_signature_of;
use solana_sdk::packet::PACKET_DATA_SIZE;
use solana_sdk::signature::keypair_from_seed;
use solana_sdk::signature::Keypair;
use solana_sdk::signature::Signature;
use solana_sdk::signer::Signer;
use solana_sdk::system_instruction;
use solana_sdk::system_instruction::MAX_PERMITTED_DATA_LENGTH;
// use solana_sdk::loader_v4::finalize;
use solana_sdk::{
    bpf_loader_upgradeable, instruction::Instruction, message::Message, pubkey::Pubkey,
    transaction::Transaction,
};
use std::str::FromStr;
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::ObjectType;
use txtx_addon_kit::types::types::Value;
use txtx_addon_kit::types::ConstructDid;

use crate::commands::get_custom_signer_did;
use crate::constants::AUTHORITY;
use crate::constants::PAYER;
use crate::constants::RPC_API_URL;
use crate::typing::SvmValue;
use crate::typing::SVM_AUTHORITY_SIGNED_TRANSACTION;
use crate::typing::SVM_PAYER_SIGNED_TRANSACTION;
use crate::typing::SVM_TEMP_AUTHORITY_SIGNED_TRANSACTION;

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

pub fn public_key_from_str(str: &str) -> Result<Pubkey, Diagnostic> {
    Pubkey::from_str(str).map_err(|e| diagnosed_error!("invalid public key: {e}"))
}

/// A struct to help deploy and upgrade an upgradeable program.
///
/// ### Actors
///  - Final Authority: this is who we want to own the program and program data account at the end of each deployment/upgrade.
///  - Temp Authority: this is a temporary keypair that we use to write to the buffer account.
///    - This prevents the user from having to sign many transactions to deploy/upgrade.
///    - Once this account is created/funded, we need to be sure to display the keys to the user. If the deployment fails, we don't want them to lose this account
///  - Buffer account: this is the account that is created to temporarily write data to before deploying to the final program address
///  - Payer: this is who pays to fund the temp authority
///
/// ## First Deployment
///  
/// ### Transaction 1: Seed Temp Authority
///  1. This is signed by the payer
///
/// ### Transaction 2: Create Buffer
///  1. First instruction creates the Buffer account
///     1. Signed by the Temp Authority
///  2. Second instruction initializes the Buffer, with the Temp Authority as the authority
///
/// ### Transaction 3 - X: Write to Buffer
///  1. Temp Authority writes to Buffer
///
/// ### Transaction X + 1: Transfer Buffer authority from Temp Authority to Final Authority
///  1. Temp Authority signs
///
/// ### Transaction X + 2: Deploy Program              
///  1. Create final program account
///     1. Final Authority signs
///  2. Transfer buffer to final program
///     1. Final Authority signs (Buffer authority **must match** program authority)
///     2. After this, the Final Authority owns the final program
///
/// ### Transaction X + 3: Transfer leftover Temp Authority funds to the Payer
///  1. Temp Authority signs
/// ---
///
/// ## Upgrades
///
/// ### Transaction 1: Seed Temp Authority
///  1. This is signed by the payer
///
/// ### Transaction 2: Create Buffer
///  1. First instruction creates the Buffer account
///     1. Signed by the Temp Authority
///  2. Second instruction initializes the Buffer, with the Temp Authority as the authority
///  3. Third instruction extends the program data account if necessary
///     1. Payed for by the Temp Authority
///
/// ### Transaction 3 - X: Write to Buffer
///  1. Temp Authority writes to Buffer
///
/// ### Transaction X + 1: Transfer Buffer authority from Temp Authority to Final Authority
///  1. Temp Authority signs
///
/// ### Transaction X + 2: Upgrade Program              
///  1. Final Authority signs
///
/// ### Transaction X + 3: Transfer leftover Temp Authority funds to the Payer
///  1. Temp Authority signs
///
pub struct UpgradeableProgramDeployer {
    /// The public key of the program to deploy.
    pub program_pubkey: Pubkey,
    /// The keypair of the program to deploy.
    pub program_keypair: Keypair,
    /// The public key of the payer.
    pub payer_pubkey: Pubkey,
    /// The public key of the final upgrade authority. (Can be the same as the payer)
    pub final_upgrade_authority_pubkey: Pubkey,
    /// The pubkey of the temporary upgrade authority.
    pub temp_upgrade_authority_pubkey: Pubkey,
    /// The keypair of the temporary upgrade authority.
    pub temp_upgrade_authority: Keypair,
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
    /// Whether the program is being upgraded (true), or deployed for the first time (false).
    pub is_program_upgrade: bool,
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
        final_upgrade_authority_pubkey: &Pubkey,
        temp_authority_keypair: Keypair,
        binary: &Vec<u8>,
        payer_pubkey: &Pubkey,
        rpc_client: RpcClient,
        commitment: Option<CommitmentConfig>,
        existing_program_buffer_opts: Option<(Pubkey, Keypair, Vec<u8>)>,
        auto_extend: Option<bool>,
    ) -> Result<Self, Diagnostic> {
        let (buffer_pubkey, buffer_keypair, buffer_data) = match existing_program_buffer_opts {
            Some((buffer_pubkey, buffer_keypair, buffer_data)) => {
                (buffer_pubkey, buffer_keypair, buffer_data)
            }
            None => {
                let (_buffer_words, _buffer_mnemonic, buffer_keypair) = create_ephemeral_keypair();
                (buffer_keypair.pubkey(), buffer_keypair, vec![0; binary.len()])
            }
        };
        let commitment =
            commitment.unwrap_or(CommitmentConfig { commitment: CommitmentLevel::Confirmed });

        let is_program_upgrade = !UpgradeableProgramDeployer::should_do_initial_deploy(
            &rpc_client,
            &program_keypair.pubkey(),
            &final_upgrade_authority_pubkey,
            &commitment,
        )?;

        Ok(Self {
            program_pubkey: program_keypair.pubkey(),
            program_keypair,
            final_upgrade_authority_pubkey: *final_upgrade_authority_pubkey,
            temp_upgrade_authority_pubkey: temp_authority_keypair.pubkey(),
            temp_upgrade_authority: temp_authority_keypair,
            binary: binary.clone(),
            payer_pubkey: *payer_pubkey,
            rpc_client,
            commitment,
            buffer_keypair,
            buffer_pubkey,
            buffer_data,
            auto_extend: auto_extend.unwrap_or(true),
            is_program_upgrade,
        })
    }

    pub fn get_transactions(&mut self) -> Result<Vec<Value>, Diagnostic> {
        let recent_blockhash = self
            .rpc_client
            .get_latest_blockhash()
            .map_err(|e| diagnosed_error!("failed to fetch latest blockhash: rpc error: {e}"))?;

        let mut core_transactions =
            // transactions for first deployment of a program
            if !self.is_program_upgrade {
                println!("not program upgrade");
                // create the buffer account
                let create_account_transaction =
                    self.get_create_buffer_transaction(&recent_blockhash)?;

                // write transaction data to the buffer account
                let mut write_transactions =
                    self.get_write_to_buffer_transactions(&recent_blockhash)?;

                // transfer the buffer authority from the temp authority to the final authority
                let transfer_authority = self.get_set_buffer_authority_to_final_authority_transaction(&recent_blockhash)?;

                // deploy the program, with the final authority as program authority
                let finalize_transaction =
                    self.get_deploy_with_max_program_len_transaction(&recent_blockhash)?;

                let mut transactions = vec![create_account_transaction];
                transactions.append(&mut write_transactions);
                transactions.push(transfer_authority);
                transactions.push(finalize_transaction);
                transactions
            }
            // transactions for upgrading an existing program
            else {
                println!("program upgrade");
                // extend the program length and create the buffer account
                let prepare_program_upgrade_transaction =
                    self.get_prepare_program_upgrade_transaction(&recent_blockhash)?;

                // write transaction data to the buffer account
                let mut write_transactions =
                    self.get_write_to_buffer_transactions(&recent_blockhash)?;

                // transfer the buffer authority from the temp authority to the final authority
                let transfer_authority = self.get_set_buffer_authority_to_final_authority_transaction(&recent_blockhash)?;

                // upgrade the program, with the final authority as program authority
                let upgrade_transaction = self.get_upgrade_transaction(&recent_blockhash)?;

                let mut transactions = vec![prepare_program_upgrade_transaction];
                transactions.append(&mut write_transactions);
                transactions.push(transfer_authority);
                transactions.push(upgrade_transaction);
                transactions
            };

        let mut transactions = vec![];
        // the first transaction needs to create the temp account
        transactions.push(
            self.get_create_temp_account_transaction(&recent_blockhash, core_transactions.len())?,
        );

        transactions.append(&mut core_transactions);

        // close out our temp authority account and transfer any leftover funds back to the payer
        transactions.push(self.get_close_temp_authority_transaction_parts()?);
        Ok(transactions)
    }

    pub fn is_program_upgrade() {}

    fn get_create_temp_account_transaction(
        &self,
        blockhash: &Hash,
        transaction_count: usize,
    ) -> Result<Value, Diagnostic> {
        const LAMPORTS_PER_SIGNATURE: u64 = 5000;
        let mut lamports = 0;

        // calculate transaction fees for all deployment transactions
        {
            let buffer_create_tx_count = 1;
            let set_buffer_authority_tx_count = 1;
            let finalize_tx_count = 1;
            let write_tx_count = transaction_count
                - buffer_create_tx_count
                - set_buffer_authority_tx_count
                - finalize_tx_count;
            let return_funds_tx_count = 1;

            // let temp_account = self
            //     .rpc_client
            //     .get_account(&self.temp_upgrade_authority_pubkey)
            //     .map_err(|e| format!("failed to get account: {e}"))?;
            // lamports += lamports_per_signature_of(&AccountSharedData::from(temp_account))
            //     .unwrap_or(LAMPORTS_PER_SIGNATURE)
            //     * (buffer_create_tx_count
            //         + write_tx_count
            //         + set_buffer_authority_tx_count
            //         + return_funds_tx_count) as u64;
            lamports += LAMPORTS_PER_SIGNATURE
                * (buffer_create_tx_count
                    + write_tx_count
                    + set_buffer_authority_tx_count
                    + return_funds_tx_count) as u64;

            // let final_authority_account = self
            //     .rpc_client
            //     .get_account(&self.final_upgrade_authority_pubkey)
            //     .map_err(|e| format!("failed to get account: {e}"))?;
            // lamports +=
            //     lamports_per_signature_of(&AccountSharedData::from(final_authority_account))
            //         .unwrap_or(LAMPORTS_PER_SIGNATURE)
            //         * (finalize_tx_count) as u64;
            lamports += LAMPORTS_PER_SIGNATURE * (finalize_tx_count) as u64;
        }

        // calculate rent for all program data written
        {
            let program_data_length = self.binary.len();
            // size for the program data
            lamports += self
                .rpc_client
                .get_minimum_balance_for_rent_exemption(
                    UpgradeableLoaderState::size_of_programdata(program_data_length),
                )
                .unwrap();
        }

        // add 20% buffer
        let lamports = ((lamports as f64) * 1.2).round() as u64;
        println!("funding temp account with {} lamports", lamports);
        let instruction = system_instruction::create_account(
            &self.payer_pubkey,
            &self.temp_upgrade_authority_pubkey,
            lamports,
            0,
            &solana_sdk::system_program::id(),
        );
        let message =
            Message::new_with_blockhash(&[instruction], Some(&self.payer_pubkey), &blockhash);

        let transaction = Transaction::new_unsigned(message);

        Ok(SvmValue::payer_signed_transaction(&transaction, vec![&self.temp_upgrade_authority])?)
    }

    fn get_create_buffer_instruction(&self) -> Result<Vec<Instruction>, Diagnostic> {
        let program_data_length = self.binary.len();

        let rent_lamports = self
            .rpc_client
            .get_minimum_balance_for_rent_exemption(UpgradeableLoaderState::size_of_programdata(
                program_data_length,
            ))
            .unwrap();

        create_buffer(
            &self.temp_upgrade_authority_pubkey,
            &self.buffer_pubkey,
            &self.temp_upgrade_authority_pubkey,
            rent_lamports,
            program_data_length,
        )
        .map_err(|e| diagnosed_error!("failed to create buffer: {e}"))
    }

    fn get_create_buffer_transaction(&self, blockhash: &Hash) -> Result<Value, Diagnostic> {
        let create_buffer_instruction = self.get_create_buffer_instruction()?;

        let message = Message::new_with_blockhash(
            &create_buffer_instruction,
            Some(&self.temp_upgrade_authority_pubkey), // todo: can this be none? isn't the payer already set in the instruction
            &blockhash,
        );

        let transaction = Transaction::new_unsigned(message);

        Ok(SvmValue::temp_authority_signed_transaction(
            &transaction,
            vec![&self.temp_upgrade_authority, &self.buffer_keypair],
        )?)
    }

    // Mostly copied from solana cli: https://github.com/txtx/solana/blob/8116c10021f09c806159852f65d37ffe6d5a118e/cli/src/program.rs#L2807
    fn get_extend_program_instruction(&self) -> Result<Option<Instruction>, Diagnostic> {
        let program_data_address = get_program_data_address(&self.program_pubkey);

        let Some(program_data_account) = self
            .rpc_client
            .get_account_with_commitment(&program_data_address, self.commitment)
            .map_err(|e| diagnosed_error!("failed to get program data account: {e}",))?
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
            return Err(diagnosed_error!(
                "New program ({}) data account is too big: {}.\n\
             Maximum program size: {}.",
                &self.program_pubkey,
                required_len,
                max_program_len
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
            Some(&self.temp_upgrade_authority_pubkey),
            additional_bytes,
        );

        Ok(Some(instruction))
    }

    fn get_prepare_program_upgrade_transaction(
        &self,
        blockhash: &Hash,
    ) -> Result<Value, Diagnostic> {
        let mut instructions = self.get_create_buffer_instruction()?;
        if self.auto_extend {
            if let Some(extend_program_instruction) = self.get_extend_program_instruction()? {
                instructions.push(extend_program_instruction);
            };
        }

        let message = Message::new_with_blockhash(
            &instructions,
            Some(&self.temp_upgrade_authority_pubkey), // todo: can this be none? isn't the payer already set in the instruction
            &blockhash,
        );

        let transaction = Transaction::new_unsigned(message);

        Ok(SvmValue::temp_authority_signed_transaction(
            &transaction,
            vec![&self.temp_upgrade_authority, &self.buffer_keypair],
        )?)
    }

    // Mostly copied from solana cli: https://github.com/txtx/solana/blob/8116c10021f09c806159852f65d37ffe6d5a118e/cli/src/program.rs#L2455
    fn get_write_to_buffer_transactions(&self, blockhash: &Hash) -> Result<Vec<Value>, Diagnostic> {
        let create_msg = |offset: u32, bytes: Vec<u8>| {
            let instruction = bpf_loader_upgradeable::write(
                &self.buffer_pubkey,
                &self.temp_upgrade_authority_pubkey,
                offset,
                bytes,
            );

            let instructions = vec![instruction];
            Message::new_with_blockhash(
                &instructions,
                Some(&self.temp_upgrade_authority_pubkey), // todo: can this be none? isn't the payer already set in the instruction
                &blockhash,
            )
        };

        let mut write_transactions = vec![];
        let chunk_size = calculate_max_chunk_size(&create_msg);
        for (chunk, i) in self.binary.chunks(chunk_size).zip(0usize..) {
            let offset = i.saturating_mul(chunk_size);
            // Only write the chunk if it differs from our initial buffer data
            if chunk != &self.buffer_data[offset..offset.saturating_add(chunk.len())] {
                let transaction =
                    Transaction::new_unsigned(create_msg(offset as u32, chunk.to_vec()));

                write_transactions.push(SvmValue::temp_authority_signed_transaction(
                    &transaction,
                    vec![&self.temp_upgrade_authority],
                )?);
            }
        }
        Ok(write_transactions)
    }

    fn get_set_buffer_authority_to_final_authority_transaction(
        &self,
        blockhash: &Hash,
    ) -> Result<Value, Diagnostic> {
        let instruction = bpf_loader_upgradeable::set_buffer_authority(
            &self.buffer_pubkey,
            &self.temp_upgrade_authority_pubkey,
            &self.final_upgrade_authority_pubkey,
        );

        let message = Message::new_with_blockhash(
            &[instruction],
            Some(&self.temp_upgrade_authority_pubkey), // todo: can this be none? isn't the payer already set in the instruction
            &blockhash,
        );

        let transaction = Transaction::new_unsigned(message);

        Ok(SvmValue::temp_authority_signed_transaction(
            &transaction,
            vec![&self.temp_upgrade_authority],
        )?)
    }

    fn get_close_temp_authority_transaction_parts(&self) -> Result<Value, Diagnostic> {
        let temp_authority_keypair_bytes =
            SvmValue::keypair(self.temp_upgrade_authority.to_bytes().to_vec());

        let rpc_api_url = Value::string(self.rpc_client.url());

        let payer_pubkey = SvmValue::pubkey(self.payer_pubkey.to_bytes().to_vec());

        Ok(ObjectType::from(vec![
            ("temp_authority_keypair", temp_authority_keypair_bytes),
            (RPC_API_URL, rpc_api_url),
            (PAYER, payer_pubkey),
        ])
        .to_value())
    }

    pub fn get_close_temp_authority_transaction(value: &Value) -> Result<Value, Diagnostic> {
        let object_map = value.expect_object();
        let temp_upgrade_authority_keypair_bytes = object_map
            .get("temp_authority_keypair")
            .expect("temp_authority_keypair")
            .expect_buffer_bytes_result()
            .unwrap();
        let temp_upgrade_authority_keypair =
            Keypair::from_bytes(&temp_upgrade_authority_keypair_bytes).unwrap();
        let temp_upgrade_authority_pubkey = temp_upgrade_authority_keypair.pubkey();

        let rpc_api_url = object_map.get(RPC_API_URL).expect(RPC_API_URL).expect_string();
        let rpc_client = RpcClient::new(rpc_api_url);
        let blockhash = rpc_client
            .get_latest_blockhash()
            .map_err(|e| diagnosed_error!("failed to fetch latest blockhash: rpc error: {e}"))?;

        let payer_pubkey = SvmValue::to_pubkey(object_map.get(PAYER).expect(PAYER)).unwrap();

        let mut instructions = vec![];
        let err_prefix = format!(
            "failed to close temp upgrade authority account ({}) and send funds back to the payer",
            temp_upgrade_authority_pubkey
        );
        // fetch data length to know how much memory to clear
        let account_info = rpc_client
            .get_account_with_commitment(
                &temp_upgrade_authority_pubkey,
                CommitmentConfig::processed(),
            )
            .map_err(|e| {
                diagnosed_error!(
                    "{err_prefix}: failed to get temp upgrade authority account data: {e}"
                )
            })?
            .value
            .ok_or(diagnosed_error!(
                "{err_prefix}: temp upgrade authority account does not exist"
            ))?;
        let data_length = account_info.data.len();
        println!("temp authority account has data length of {}", data_length);

        if data_length > 0 {
            // Instruction to zero out the data (write zeros)
            let zero_data = vec![0u8; data_length];
            let clear_data_instruction = Instruction {
                program_id: solana_sdk::system_program::ID, // Or the relevant program
                accounts: vec![solana_sdk::instruction::AccountMeta::new(
                    temp_upgrade_authority_pubkey.clone(),
                    true,
                )],
                data: zero_data,
            };
            instructions.push(clear_data_instruction);
        }

        // Instruction to assign the account to the system program (deallocate it)
        println!("temp authority account has owner of {}", account_info.owner);
        if account_info.owner != solana_sdk::system_program::ID {
            let assign_instruction = system_instruction::assign(
                &temp_upgrade_authority_pubkey,
                &solana_sdk::system_program::ID,
            );
            instructions.push(assign_instruction);
        }

        // fetch balance to know how much to transfer back
        let temp_authority_balance = rpc_client
            .get_balance(&temp_upgrade_authority_pubkey)
            .map_err(|e| diagnosed_error!("{err_prefix}: failed to get leftover balance: {e}"))?;

        println!("temp authority has nonzero balance of {} lamports", temp_authority_balance);
        if temp_authority_balance > 0 {
            let transfer_instruction = system_instruction::transfer(
                &temp_upgrade_authority_pubkey,
                &payer_pubkey,
                temp_authority_balance,
            );
            let mut fee_instructions = instructions.clone();
            fee_instructions.push(transfer_instruction);

            let fee_message =
                Message::new_with_blockhash(&fee_instructions, Some(&payer_pubkey), &blockhash);
            let fee = rpc_client.get_fee_for_message(&fee_message).map_err(|e| {
                diagnosed_error!("{err_prefix}: failed to get fee for transfer: {e}")
            })?;
            // the temp authority has enough to pay the fee
            if temp_authority_balance >= fee {
                // the temp authority has leftovers after the fee, so transfer that back to the payer
                if temp_authority_balance > fee {
                    let transfer_instruction = system_instruction::transfer(
                        &temp_upgrade_authority_pubkey,
                        &payer_pubkey,
                        temp_authority_balance - fee,
                    );
                    instructions.push(transfer_instruction);
                }
                println!("temp authority has balance of {} lamports, which will be returned to the payer", temp_authority_balance);
                let message = Message::new_with_blockhash(
                    &instructions,
                    Some(&temp_upgrade_authority_pubkey),
                    &blockhash,
                );

                let transaction = Transaction::new_unsigned(message);

                return Ok(SvmValue::temp_authority_signed_transaction(
                    &transaction,
                    vec![&temp_upgrade_authority_keypair],
                )?);
            }
        }

        println!("Temp authority account has no funds to transfer back to the payer.");
        println!("The payer will have to sign a transaction to close the account.");

        let message = Message::new_with_blockhash(&instructions, Some(&payer_pubkey), &blockhash);

        let transaction = Transaction::new_unsigned(message);

        Ok(SvmValue::payer_signed_transaction(&transaction, vec![])?)
    }

    fn get_deploy_with_max_program_len_transaction(
        &self,
        blockhash: &Hash,
    ) -> Result<Value, Diagnostic> {
        let instructions = bpf_loader_upgradeable::deploy_with_max_program_len(
            &self.temp_upgrade_authority_pubkey,
            &self.program_pubkey,
            &self.buffer_pubkey,
            &self.final_upgrade_authority_pubkey,
            self.rpc_client
                .get_minimum_balance_for_rent_exemption(UpgradeableLoaderState::size_of_program())
                .map_err(|e| {
                    diagnosed_error!("failed to get minimum balance for rent exemption: {e}")
                })?,
            self.binary.len(),
        )
        .map_err(|e| {
            diagnosed_error!("failed to create deploy with max program len instruction: {e}")
        })?;

        let message = Message::new_with_blockhash(
            &instructions,
            Some(&self.temp_upgrade_authority_pubkey),
            &blockhash,
        );
        let transaction = Transaction::new_unsigned(message);

        Ok(SvmValue::authority_signed_transaction(
            &transaction,
            vec![&self.temp_upgrade_authority, &self.program_keypair],
        )?)
    }

    fn get_upgrade_transaction(&self, blockhash: &Hash) -> Result<Value, Diagnostic> {
        let upgrade_instruction = bpf_loader_upgradeable::upgrade(
            &self.program_pubkey,
            &self.buffer_pubkey,
            &self.final_upgrade_authority_pubkey,
            &self.payer_pubkey,
        );

        let message = Message::new_with_blockhash(
            &[upgrade_instruction],
            Some(&self.final_upgrade_authority_pubkey),
            &blockhash,
        );
        let transaction = Transaction::new_unsigned(message);

        Ok(SvmValue::authority_signed_transaction(&transaction, vec![])?)
    }

    /// Logic mostly copied from solana cli: https://github.com/txtx/solana/blob/8116c10021f09c806159852f65d37ffe6d5a118e/cli/src/program.rs#L1248-L1249
    fn should_do_initial_deploy(
        rpc_client: &RpcClient,
        program_pubkey: &Pubkey,
        final_upgrade_authority_pubkey: &Pubkey,
        commitment: &CommitmentConfig,
    ) -> Result<bool, Diagnostic> {
        if let Some(account) = rpc_client
            .get_account_with_commitment(&program_pubkey, commitment.clone())
            .map_err(|e| diagnosed_error!("failed to get program account: {e}"))?
            .value
        {
            if account.owner != bpf_loader_upgradeable::id() {
                return Err(diagnosed_error!(
                    "Account {} is not an upgradeable program or already in use",
                    program_pubkey
                )
                .into());
            }
            if !account.executable {
                return Ok(true);
            } else if let Ok(UpgradeableLoaderState::Program { programdata_address }) =
                account.state()
            {
                if let Some(account) = rpc_client
                    .get_account_with_commitment(&programdata_address, commitment.clone())
                    .map_err(|e| diagnosed_error!("failed to get program data account: {e}"))?
                    .value
                {
                    if let Ok(UpgradeableLoaderState::ProgramData {
                        slot: _,
                        upgrade_authority_address: program_authority_pubkey,
                    }) = account.state()
                    {
                        if let Some(program_authority_pubkey) = program_authority_pubkey {
                            if program_authority_pubkey != *final_upgrade_authority_pubkey {
                                return Err(diagnosed_error!(
                                    "Program's authority {:?} does not match authority provided {:?}",
                                    program_authority_pubkey, final_upgrade_authority_pubkey,
                                )
                                .into());
                            }
                        }
                        // Do upgrade
                        return Ok(false);
                    } else {
                        return Err(diagnosed_error!(
                            "Program {} has been closed, use a new Program Id",
                            program_pubkey
                        )
                        .into());
                    }
                } else {
                    return Err(diagnosed_error!(
                        "Program {} has been closed, use a new Program Id",
                        program_pubkey
                    )
                    .into());
                }
            } else {
                return Err(
                    diagnosed_error!("{} is not an upgradeable program", program_pubkey).into()
                );
            }
        } else {
            return Ok(true);
        };
    }

    pub fn get_signer_did_from_transaction_value(
        value: &Value,
        values: &ValueStore,
    ) -> Result<(Vec<u8>, Option<ConstructDid>), Diagnostic> {
        let addon_data = value.as_addon_data().expect("expected addon data");
        let (transaction_bytes, _) = SvmValue::parse_transaction_with_keypairs(&value).unwrap();
        let signer_key = match addon_data.id.as_str() {
            SVM_PAYER_SIGNED_TRANSACTION => Some(PAYER),
            SVM_AUTHORITY_SIGNED_TRANSACTION => Some(AUTHORITY),
            SVM_TEMP_AUTHORITY_SIGNED_TRANSACTION => None,
            _ => unreachable!("invalid transaction type"),
        };
        let Some(signer_key) = signer_key else {
            return Ok((transaction_bytes, None));
        };

        let signer_did = get_custom_signer_did(values, signer_key)
            .map_err(|e| diagnosed_error!("failed to get signer DID: {e}"))?;
        Ok((transaction_bytes, Some(signer_did)))
    }

    pub fn create_temp_authority() -> Keypair {
        let (_buffer_words, _buffer_mnemonic, temp_authority_keypair) = create_ephemeral_keypair();
        let temp_authority_pubkey = temp_authority_keypair.pubkey();

        println!("A temporary account will be created and funded to write to the buffer account.");
        println!("Please save the following information in case the deployment fails and the account needs to be recovered:");
        println!("Temporary Authority Public Key: {:?}", temp_authority_pubkey);
        println!("Temporary Authority Keypair: {:?}", temp_authority_keypair);
        temp_authority_keypair
    }
}

// todo: need to make this function secure, and to verify the account doesn't already exist
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
