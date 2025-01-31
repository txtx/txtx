pub mod anchor;
pub mod idl;
pub mod instruction;
pub mod send_transaction;
pub mod squads;

use bip39::Language;
use bip39::Mnemonic;
use bip39::MnemonicType;
use bip39::Seed;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
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
use solana_sdk::system_instruction;
use solana_sdk::system_instruction::MAX_PERMITTED_DATA_LENGTH;
// use solana_sdk::loader_v4::finalize;
use solana_sdk::{
    bpf_loader_upgradeable, instruction::Instruction, message::Message, pubkey::Pubkey,
    transaction::Transaction,
};
use std::collections::HashMap;
use std::str::FromStr;
use txtx_addon_kit::hex;
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::types::frontend::ProgressBarStatus;
use txtx_addon_kit::types::frontend::ProgressBarStatusColor;
use txtx_addon_kit::types::frontend::StatusUpdater;
use txtx_addon_kit::types::signers::SignerInstance;
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::Value;
use txtx_addon_kit::types::ConstructDid;

use crate::commands::get_custom_signer_did;
use crate::constants::AUTHORITY;
use crate::constants::PAYER;
use crate::typing::SvmValue;
use crate::typing::SVM_CLOSE_TEMP_AUTHORITY_TRANSACTION_PARTS;
use crate::typing::SVM_DEPLOYMENT_TRANSACTION;

const LAMPORTS_PER_SIGNATURE: u64 = 5000;

pub fn public_key_from_str(str: &str) -> Result<Pubkey, Diagnostic> {
    Pubkey::from_str(str).map_err(|e| diagnosed_error!("invalid public key: {e}"))
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

#[derive(Debug, Serialize, Deserialize)]
pub enum TxtxDeploymentSigner {
    Payer,
    FinalAuthority,
}
impl TxtxDeploymentSigner {
    pub fn to_signer_key(&self) -> String {
        match self {
            Self::Payer => PAYER.to_string(),
            Self::FinalAuthority => AUTHORITY.to_string(),
        }
    }
}

/// `transaction_with_keypairs` - The transaction to sign, with the keypairs we have on hand to sign them
/// `signers` - The txtx signers (if any) that need to sign the transaction
#[derive(Debug, Serialize, Deserialize)]
pub struct DeploymentTransaction {
    pub signers: Option<Vec<TxtxDeploymentSigner>>,
    pub transaction: Transaction,
    pub keypairs_bytes: Vec<Vec<u8>>,
    pub transaction_type: DeploymentTransactionType,
    pub commitment_level: CommitmentLevel,
    pub do_await_confirmation: bool,
}

impl DeploymentTransaction {
    pub fn new(
        transaction: &Transaction,
        keypairs: Vec<&Keypair>,
        signers: Option<Vec<TxtxDeploymentSigner>>,
        transaction_type: DeploymentTransactionType,
        commitment_level: CommitmentLevel,
        do_await_confirmation: bool,
    ) -> Self {
        let keypairs_bytes = keypairs.iter().map(|k| k.to_bytes().to_vec()).collect();
        Self {
            signers,
            transaction: transaction.clone(),
            keypairs_bytes,
            transaction_type,
            commitment_level,
            do_await_confirmation,
        }
    }

    pub fn create_temp_account(
        transaction: &Transaction,
        keypairs: Vec<&Keypair>,
        temp_authority_keypair: &Keypair,
    ) -> Self {
        Self::new(
            transaction,
            keypairs,
            Some(vec![TxtxDeploymentSigner::Payer]),
            DeploymentTransactionType::CreateTempAuthority(
                temp_authority_keypair.to_bytes().to_vec(),
            ),
            CommitmentLevel::Confirmed,
            true,
        )
    }

    pub fn create_buffer(transaction: &Transaction, keypairs: Vec<&Keypair>) -> Self {
        Self::new(
            transaction,
            keypairs,
            None,
            DeploymentTransactionType::CreateBuffer,
            CommitmentLevel::Confirmed,
            true,
        )
    }

    pub fn write_to_buffer(
        transaction: &Transaction,
        keypairs: Vec<&Keypair>,
        commitment_level: CommitmentLevel,
        do_await_confirmation: bool,
    ) -> Self {
        Self::new(
            transaction,
            keypairs,
            None,
            DeploymentTransactionType::WriteToBuffer,
            commitment_level,
            do_await_confirmation,
        )
    }

    pub fn transfer_buffer_authority(transaction: &Transaction, keypairs: Vec<&Keypair>) -> Self {
        Self::new(
            transaction,
            keypairs,
            None,
            DeploymentTransactionType::TransferBufferAuthority,
            CommitmentLevel::Confirmed,
            true,
        )
    }

    pub fn transfer_program_authority(transaction: &Transaction, keypairs: Vec<&Keypair>) -> Self {
        Self::new(
            transaction,
            keypairs,
            None,
            DeploymentTransactionType::TransferProgramAuthority,
            CommitmentLevel::Confirmed,
            true,
        )
    }

    pub fn deploy_program(transaction: &Transaction, keypairs: Vec<&Keypair>) -> Self {
        Self::new(
            transaction,
            keypairs,
            None,
            DeploymentTransactionType::DeployProgram,
            CommitmentLevel::Confirmed,
            true,
        )
    }

    pub fn upgrade_program(transaction: &Transaction, keypairs: Vec<&Keypair>) -> Self {
        Self::new(
            transaction,
            keypairs,
            Some(vec![TxtxDeploymentSigner::FinalAuthority]),
            DeploymentTransactionType::UpgradeProgram,
            CommitmentLevel::Confirmed,
            true,
        )
    }

    pub fn payer_close_temp_authority(transaction: &Transaction, keypairs: Vec<&Keypair>) -> Self {
        Self::new(
            transaction,
            keypairs,
            Some(vec![TxtxDeploymentSigner::Payer]),
            DeploymentTransactionType::CloseTempAuthority,
            CommitmentLevel::Confirmed,
            true,
        )
    }

    pub fn temp_authority_close_temp_authority(
        transaction: &Transaction,
        keypairs: Vec<&Keypair>,
    ) -> Self {
        Self::new(
            transaction,
            keypairs,
            None,
            DeploymentTransactionType::CloseTempAuthority,
            CommitmentLevel::Confirmed,
            true,
        )
    }

    pub fn skip_temp_authority_close() -> Self {
        Self {
            signers: None,
            transaction: Transaction::new_unsigned(Message::new(&[], None)),
            keypairs_bytes: vec![],
            transaction_type: DeploymentTransactionType::SkipCloseTempAuthority,
            commitment_level: CommitmentLevel::Confirmed,
            do_await_confirmation: false,
        }
    }

    pub fn to_value(&self) -> Result<Value, Diagnostic> {
        let bytes = serde_json::to_vec(self)
            .map_err(|e| diagnosed_error!("failed to serialize transaction with keypairs: {e}"))?;
        Ok(SvmValue::deployment_transaction(bytes))
    }

    pub fn from_value(value: &Value) -> Result<Self, Diagnostic> {
        let addon_data = value.as_addon_data().ok_or(diagnosed_error!(
            "expected addon data for deployment transaction, found: {}",
            value.get_type().to_string()
        ))?;
        if addon_data.id == SVM_DEPLOYMENT_TRANSACTION {
            return serde_json::from_slice(&addon_data.bytes).map_err(|e| {
                diagnosed_error!("failed to deserialize deployment transaction: {e}")
            });
        } else if addon_data.id == SVM_CLOSE_TEMP_AUTHORITY_TRANSACTION_PARTS {
            let parts = CloseTempAuthorityTransactionParts::from_value(value)?;
            return match UpgradeableProgramDeployer::get_close_temp_authority_transaction(&parts)? {
                Some(transaction_value) => Self::from_value(&transaction_value),
                None => Ok(Self::skip_temp_authority_close()),
            };
        } else {
            return Err(diagnosed_error!(
                "failed to decode deployment transaction: invalid addon data type: {}",
                addon_data.id
            ));
        }
    }

    pub fn get_signers_dids(
        &self,
        authority_signer_did: ConstructDid,
        payer_signer_did: ConstructDid,
    ) -> Result<Option<Vec<ConstructDid>>, Diagnostic> {
        let signer_dids = match &self.signers {
            Some(signers) => Some(
                signers
                    .iter()
                    .filter_map(|s| {
                        if s.to_signer_key() == AUTHORITY {
                            Some(authority_signer_did.clone())
                        } else if s.to_signer_key() == PAYER {
                            Some(payer_signer_did.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<ConstructDid>>(),
            ),
            None => None,
        };

        Ok(signer_dids)
    }

    pub fn get_formatted_transaction(
        &self,
        signer_dids: Vec<ConstructDid>,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
    ) -> Result<Option<(String, String)>, Diagnostic> {
        let description = match &self.transaction_type {
            DeploymentTransactionType::CreateTempAuthority(_) => {
                "This transaction creates a temporary account that will execute the deployment."
            }
            DeploymentTransactionType::CreateBuffer => return Ok(None),
            DeploymentTransactionType::WriteToBuffer => return Ok(None),
            DeploymentTransactionType::TransferBufferAuthority => return Ok(None),
            DeploymentTransactionType::TransferProgramAuthority => return Ok(None),
            DeploymentTransactionType::DeployProgram => "This transaction will deploy the program.",
            DeploymentTransactionType::UpgradeProgram => {
                "This transaction will upgrade the program."
            }
            DeploymentTransactionType::CloseTempAuthority => return Ok(None),
            DeploymentTransactionType::SkipCloseTempAuthority => return Ok(None),
        };

        let mut signer_names = String::new();
        let signer_count = signer_dids.len();
        for (i, did) in signer_dids.iter().enumerate() {
            let signer_instance = signers_instances.get(did).unwrap();
            let name = format!("'{}'", signer_instance.name);
            if i == 0 {
                signer_names = name;
            } else {
                if signer_count > 2 {
                    if i == signer_count - 1 {
                        signer_names = format!("{} & {}", signer_names, name);
                    } else {
                        signer_names = format!("{}, {}", signer_names, name);
                    }
                } else {
                    signer_names = format!("{} & {}", signer_names, name);
                }
            }
        }

        let description = format!(
            "{} Signed by the {} signer{}.",
            description,
            signer_names,
            if signer_count > 1 { "s" } else { "" }
        );

        let mut instructions = vec![];
        let message_account_keys = self.transaction.message.account_keys.clone();
        for instruction in self.transaction.message.instructions.iter() {
            let Some(account) = message_account_keys.get(instruction.program_id_index as usize)
            else {
                continue;
            };
            let accounts = instruction
                .accounts
                .iter()
                .filter_map(|a| {
                    let Some(account) = message_account_keys.get(*a as usize) else {
                        return None;
                    };
                    Some(account.to_string())
                })
                .collect::<Vec<String>>();
            let account_name = account.to_string();

            instructions.push(json!({
                "program_id": account_name,
                "instruction_data": format!("0x{}", hex::encode(&instruction.data)),
                "accounts": accounts
            }));
        }
        let formatted_transaction = json!({
            "instructions": instructions,
            "num_required_signatures": self.transaction.message.header.num_required_signatures,
            "num_readonly_signed_accounts": self.transaction.message.header.num_readonly_signed_accounts,
            "num_readonly_unsigned_accounts": self.transaction.message.header.num_readonly_unsigned_accounts,
        });

        Ok(Some((serde_json::to_string_pretty(&formatted_transaction).unwrap(), description)))
    }

    pub fn get_keypairs(&self) -> Result<Vec<Keypair>, Diagnostic> {
        self.keypairs_bytes
            .iter()
            .map(|b| Keypair::from_bytes(&b))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| diagnosed_error!("failed to decode keypair: {e}"))
    }

    pub fn sign_transaction_with_keypairs(
        &self,
        rpc_api_url: &str,
    ) -> Result<Transaction, Diagnostic> {
        let rpc_client = RpcClient::new_with_commitment(
            rpc_api_url,
            CommitmentConfig { commitment: self.commitment_level },
        );

        let blockhash = rpc_client
            .get_latest_blockhash()
            .map_err(|e| diagnosed_error!("failed to get latest blockhash: {e}"))?;

        let mut transaction: Transaction = self.transaction.clone();

        transaction.message.recent_blockhash = blockhash;
        let keypairs =
            self.get_keypairs().map_err(|e| diagnosed_error!("failed to sign transaction: {e}"))?;

        transaction
            .try_partial_sign(&keypairs, transaction.message.recent_blockhash)
            .map_err(|e| diagnosed_error!("failed to sign transaction: {e}"))?;

        Ok(transaction)
    }

    pub fn pre_send_status_updates(
        &self,
        status_updater: &mut StatusUpdater,
        transaction_index: usize,
        transaction_count: usize,
    ) -> Result<(), Diagnostic> {
        match &self.transaction_type {
            DeploymentTransactionType::SkipCloseTempAuthority => {
                status_updater.propagate_info(&format!(
                    "Temp account has no leftover funds; skipping transaction to close the account",
                ));
                return Ok(());
            }
            DeploymentTransactionType::CreateTempAuthority(temp_authority_keypair_bytes) => {
                let temp_authority_keypair = Keypair::from_bytes(&temp_authority_keypair_bytes)
                    .map_err(|e| {
                        diagnosed_error!("failed to deserialize temp authority keypair: {}", e)
                    })?;
                status_updater
                    .propagate_info("A temporary account will be created and funded to write to the buffer account.");
                status_updater
                    .propagate_info("Please save the following information in case the deployment fails and the account needs to be recovered:");
                status_updater.propagate_info(&format!(
                    "Temporary Authority Public Key: {}",
                    temp_authority_keypair.pubkey()
                ));
                status_updater.propagate_info(&format!(
                    "Temporary Authority Keypair: {}",
                    temp_authority_keypair.to_base58_string()
                ));
            }
            _ => {}
        };

        // to prevent overloading the supervisor with a ton of status updates,
        // only send for every 10 of the buffer writes
        if match &self.transaction_type {
            DeploymentTransactionType::WriteToBuffer => (transaction_index + 1) % 10 == 0,
            _ => true,
        } {
            status_updater.propagate_pending_status(&format!(
                "Sending transaction {}/{}",
                transaction_index + 1,
                transaction_count
            ));
        }

        Ok(())
    }

    pub fn post_send_status_updates(&self, status_updater: &mut StatusUpdater, program_id: Pubkey) {
        match self.transaction_type {
            DeploymentTransactionType::CreateTempAuthority(_) => {
                status_updater.propagate_status(ProgressBarStatus::new_msg(
                    ProgressBarStatusColor::Green,
                    "Account Created",
                    "Temp account created to write to buffer",
                ));
            }
            DeploymentTransactionType::CreateBuffer => {
                status_updater.propagate_status(ProgressBarStatus::new_msg(
                    ProgressBarStatusColor::Green,
                    "Account Created",
                    "Program buffer creation complete",
                ));
            }
            DeploymentTransactionType::DeployProgram => {
                status_updater.propagate_status(ProgressBarStatus::new_msg(
                    ProgressBarStatusColor::Green,
                    "Program Created",
                    &format!("Program {} has been deployed", program_id,),
                ));
            }
            DeploymentTransactionType::UpgradeProgram => {
                status_updater.propagate_status(ProgressBarStatus::new_msg(
                    ProgressBarStatusColor::Green,
                    "Program Upgraded",
                    &format!("Program {} has been upgraded", program_id,),
                ));
            }
            DeploymentTransactionType::CloseTempAuthority => {
                status_updater.propagate_status(ProgressBarStatus::new_msg(
                    ProgressBarStatusColor::Green,
                    "Complete",
                    "Temp account closed and leftover funds returned to payer",
                ));
            }
            _ => {}
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CloseTempAuthorityTransactionParts {
    pub temp_authority_keypair_bytes: Vec<u8>,
    pub rpc_api_url: String,
    pub payer_pubkey: Pubkey,
}

impl CloseTempAuthorityTransactionParts {
    pub fn to_value(&self) -> Result<Value, Diagnostic> {
        let bytes = serde_json::to_vec(self)
            .map_err(|e| diagnosed_error!("failed to serialize close temp authority parts: {e}"))?;
        Ok(SvmValue::close_temp_authority_transaction_parts(bytes))
    }
    pub fn from_value(value: &Value) -> Result<Self, Diagnostic> {
        let addon_data = value
            .as_addon_data()
            .ok_or(diagnosed_error!("expected addon data for close temp authority parts"))?;

        if addon_data.id == SVM_CLOSE_TEMP_AUTHORITY_TRANSACTION_PARTS {
            return serde_json::from_slice(&addon_data.bytes).map_err(|e| {
                diagnosed_error!("failed to deserialize close temp authority parts: {e}")
            });
        } else {
            return Err(diagnosed_error!("expected close temp authority parts"));
        }
    }
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
/// ### Transaction X + 1: Deploy Program              
///  1. Create final program account
///     1. Temp Authority signs
///  2. Transfer buffer to final program
///     1. Temp Authority signs (Buffer authority **must match** program authority)
///     2. After this, the Temp Authority owns the final program
///
/// ### Transaction X + 2: Transfer Program authority from Temp Authority to Final Authority
///  1. Temp Authority signs
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

        let is_program_upgrade = !UpgradeableProgramDeployer::should_do_initial_deploy(
            &rpc_client,
            &program_keypair.pubkey(),
            &final_upgrade_authority_pubkey,
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

                // create the buffer account
                let create_account_transaction =
                    self.get_create_buffer_transaction(&recent_blockhash)?;

                // write transaction data to the buffer account
                let mut write_transactions =
                    self.get_write_to_buffer_transactions(&recent_blockhash)?;

                // deploy the program, with the final authority as program authority
                let finalize_transaction =
                    self.get_deploy_with_max_program_len_transaction(&recent_blockhash)?;

                // transfer the program authority from the temp authority to the final authority
                let transfer_authority = self.get_set_program_authority_to_final_authority_transaction(&recent_blockhash)?;

                let mut transactions = vec![create_account_transaction];
                transactions.append(&mut write_transactions);
                transactions.push(finalize_transaction);
                transactions.push(transfer_authority);
                transactions
            }
            // transactions for upgrading an existing program
            else {

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

        DeploymentTransaction::create_temp_account(
            &transaction,
            vec![&self.temp_upgrade_authority],
            &self.temp_upgrade_authority,
        )
        .to_value()
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

        DeploymentTransaction::create_buffer(
            &transaction,
            vec![&self.temp_upgrade_authority, &self.buffer_keypair],
        )
        .to_value()
    }

    // Mostly copied from solana cli: https://github.com/txtx/solana/blob/8116c10021f09c806159852f65d37ffe6d5a118e/cli/src/program.rs#L2807
    fn get_extend_program_instruction(&self) -> Result<Option<Instruction>, Diagnostic> {
        let program_data_address = get_program_data_address(&self.program_pubkey);

        let Some(program_data_account) = self
            .rpc_client
            .get_account_with_commitment(&program_data_address, CommitmentConfig::processed())
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

        DeploymentTransaction::create_buffer(
            &transaction,
            vec![&self.temp_upgrade_authority, &self.buffer_keypair],
        )
        .to_value()
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

        let chunks = self.binary.chunks(chunk_size).collect::<Vec<_>>();

        for (chunk, i) in chunks.iter().zip(0usize..) {
            let offset = i.saturating_mul(chunk_size);
            // Only write the chunk if it differs from our initial buffer data
            if *chunk != &self.buffer_data[offset..offset.saturating_add(chunk.len())] {
                let transaction =
                    Transaction::new_unsigned(create_msg(offset as u32, chunk.to_vec()));

                let (do_await_confirmation, commitment_level) = if i == chunks.len() - 1 {
                    (true, CommitmentLevel::Confirmed)
                } else {
                    (false, CommitmentLevel::Processed)
                };

                write_transactions.push(
                    DeploymentTransaction::write_to_buffer(
                        &transaction,
                        vec![&self.temp_upgrade_authority],
                        commitment_level,
                        do_await_confirmation,
                    )
                    .to_value()?,
                );
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

        DeploymentTransaction::transfer_buffer_authority(
            &transaction,
            vec![&self.temp_upgrade_authority],
        )
        .to_value()
    }

    fn get_set_program_authority_to_final_authority_transaction(
        &self,
        blockhash: &Hash,
    ) -> Result<Value, Diagnostic> {
        let instruction = bpf_loader_upgradeable::set_upgrade_authority(
            &self.program_pubkey,
            &self.temp_upgrade_authority_pubkey,
            Some(&self.final_upgrade_authority_pubkey),
        );

        let message = Message::new_with_blockhash(
            &[instruction],
            Some(&self.temp_upgrade_authority_pubkey), // todo: can this be none? isn't the payer already set in the instruction
            &blockhash,
        );

        let transaction = Transaction::new_unsigned(message);

        DeploymentTransaction::transfer_program_authority(
            &transaction,
            vec![&self.temp_upgrade_authority],
        )
        .to_value()
    }

    fn get_close_temp_authority_transaction_parts(&self) -> Result<Value, Diagnostic> {
        CloseTempAuthorityTransactionParts {
            temp_authority_keypair_bytes: self.temp_upgrade_authority.to_bytes().to_vec(),
            rpc_api_url: self.rpc_client.url(),
            payer_pubkey: self.payer_pubkey,
        }
        .to_value()
    }

    /// Closing an account requires clearing out the data and removing all lamports to not cover rent.
    /// The temp account we're opening does not have any data, so we don't need to clear it.
    /// So this function is only used to send any leftover lamports back to the payer.
    pub fn get_close_temp_authority_transaction(
        CloseTempAuthorityTransactionParts {
            temp_authority_keypair_bytes,
            rpc_api_url,
            payer_pubkey,
        }: &CloseTempAuthorityTransactionParts,
    ) -> Result<Option<Value>, Diagnostic> {
        let temp_upgrade_authority_keypair =
            Keypair::from_bytes(&temp_authority_keypair_bytes).unwrap();
        let temp_upgrade_authority_pubkey = temp_upgrade_authority_keypair.pubkey();

        // use processed commitment so we're sure to get the most recent balance
        let rpc_client = RpcClient::new_with_commitment(rpc_api_url, CommitmentConfig::confirmed());

        let err_prefix = format!(
            "failed to close temp upgrade authority account ({}) and send funds back to the payer",
            temp_upgrade_authority_pubkey
        );

        let blockhash = rpc_client.get_latest_blockhash().map_err(|e| {
            diagnosed_error!("{err_prefix}: failed to fetch latest blockhash: rpc error: {e}")
        })?;

        let err_prefix = format!(
            "failed to close temp upgrade authority account ({}) and send funds back to the payer",
            temp_upgrade_authority_pubkey
        );

        // fetch balance to know how much to transfer back
        let temp_authority_balance = rpc_client
            .get_balance(&temp_upgrade_authority_pubkey)
            .map_err(|e| diagnosed_error!("{err_prefix}: failed to get leftover balance: {e}"))?;

        if temp_authority_balance > (LAMPORTS_PER_SIGNATURE) {
            let instructions = vec![system_instruction::transfer(
                &temp_upgrade_authority_pubkey,
                &payer_pubkey,
                temp_authority_balance - (5000),
            )];

            let message = Message::new_with_blockhash(
                &instructions,
                Some(&temp_upgrade_authority_pubkey),
                &blockhash,
            );

            let transaction = Transaction::new_unsigned(message);

            return Some(
                DeploymentTransaction::temp_authority_close_temp_authority(
                    &transaction,
                    vec![&temp_upgrade_authority_keypair],
                )
                .to_value(),
            )
            .transpose();
        }

        return Ok(None);
    }

    fn get_deploy_with_max_program_len_transaction(
        &self,
        blockhash: &Hash,
    ) -> Result<Value, Diagnostic> {
        let instructions = bpf_loader_upgradeable::deploy_with_max_program_len(
            &self.temp_upgrade_authority_pubkey,
            &self.program_pubkey,
            &self.buffer_pubkey,
            &self.temp_upgrade_authority_pubkey,
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

        DeploymentTransaction::deploy_program(
            &transaction,
            vec![&self.temp_upgrade_authority, &self.program_keypair],
        )
        .to_value()
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

        DeploymentTransaction::upgrade_program(&transaction, vec![]).to_value()
    }

    /// Logic mostly copied from solana cli: https://github.com/txtx/solana/blob/8116c10021f09c806159852f65d37ffe6d5a118e/cli/src/program.rs#L1248-L1249
    fn should_do_initial_deploy(
        rpc_client: &RpcClient,
        program_pubkey: &Pubkey,
        final_upgrade_authority_pubkey: &Pubkey,
    ) -> Result<bool, Diagnostic> {
        if let Some(account) = rpc_client
            .get_account_with_commitment(&program_pubkey, CommitmentConfig::processed())
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
                    .get_account_with_commitment(
                        &programdata_address,
                        CommitmentConfig::processed(),
                    )
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

    // todo: need to make this function secure, and to verify the account doesn't already exist
    pub fn create_temp_authority() -> Keypair {
        let (_buffer_words, _buffer_mnemonic, temp_authority_keypair) = create_ephemeral_keypair();

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

pub fn transaction_is_fully_signed(transaction: &Transaction) -> bool {
    let expected_signature_count = transaction.message.header.num_required_signatures as usize;
    let actual_signature_count = transaction.signatures.len();
    expected_signature_count == actual_signature_count && transaction.is_signed()
}
