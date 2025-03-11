pub mod anchor;
pub mod idl;
pub mod instruction;
pub mod native;
pub mod send_transaction;
pub mod subgraph;

use anchor::AnchorProgramArtifacts;
use bip39::Language;
use bip39::Mnemonic;
use bip39::MnemonicType;
use bip39::Seed;
use native::ClassicRustProgramArtifacts;
use serde::Deserialize;
use serde::Serialize;
use solana_client::rpc_client::RpcClient;
use solana_sdk::bpf_loader_upgradeable::UpgradeableLoaderState;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::commitment_config::CommitmentLevel;
use solana_sdk::hash::Hash;
use solana_sdk::loader_v4;
use solana_sdk::loader_v4::LoaderV4State;
use solana_sdk::loader_v4::LoaderV4Status;
use solana_sdk::packet::PACKET_DATA_SIZE;
use solana_sdk::signature::keypair_from_seed;
use solana_sdk::signature::Keypair;
use solana_sdk::signature::Signature;
use solana_sdk::signer::Signer;
use solana_sdk::system_instruction;
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
use txtx_addon_kit::types::types::ObjectType;
use txtx_addon_kit::types::types::Value;
use txtx_addon_kit::types::ConstructDid;

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
            "deploy_program" => DeploymentTransactionType::DeployProgram,
            "upgrade_program" => DeploymentTransactionType::UpgradeProgram,
            "close_temp_authority" => DeploymentTransactionType::CloseTempAuthority,
            "skip_close_temp_authority" => DeploymentTransactionType::SkipCloseTempAuthority,
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

    pub fn deploy_program(transaction: &Transaction, keypairs: Vec<&Keypair>) -> Self {
        Self::new(
            transaction,
            keypairs,
            Some(vec![TxtxDeploymentSigner::FinalAuthority]),
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
    ) -> Result<Option<(Value, String)>, Diagnostic> {
        let description = match &self.transaction_type {
            DeploymentTransactionType::CreateTempAuthority(_) => {
                "This transaction creates an ephemeral account that will execute the deployment."
            }
            DeploymentTransactionType::CreateBuffer => return Ok(None),
            DeploymentTransactionType::WriteToBuffer => return Ok(None),
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
                    Some(Value::string(account.to_string()))
                })
                .collect::<Vec<Value>>();
            let account_name = account.to_string();

            instructions.push(
                ObjectType::from(vec![
                    ("program_id", Value::string(account_name)),
                    (
                        "instruction_data",
                        Value::string(format!("0x{}", hex::encode(&instruction.data))),
                    ),
                    ("accounts", Value::array(accounts)),
                ])
                .to_value(),
            );
        }
        let formatted_transaction = ObjectType::from(vec![
            ("instructions", Value::array(instructions)),
            (
                "num_required_signatures",
                Value::integer(self.transaction.message.header.num_required_signatures as i128),
            ),
            (
                "num_readonly_signed_accounts",
                Value::integer(
                    self.transaction.message.header.num_readonly_signed_accounts as i128,
                ),
            ),
            (
                "num_readonly_unsigned_accounts",
                Value::integer(
                    self.transaction.message.header.num_readonly_unsigned_accounts as i128,
                ),
            ),
        ]);

        Ok(Some((formatted_transaction.to_value(), description)))
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
                    "Ephemeral authority account has no leftover funds; skipping transaction to close the account",
                ));
                return Ok(());
            }
            DeploymentTransactionType::CreateTempAuthority(temp_authority_keypair_bytes) => {
                let temp_authority_keypair = Keypair::from_bytes(&temp_authority_keypair_bytes)
                    .map_err(|e| {
                        diagnosed_error!("failed to deserialize temp authority keypair: {}", e)
                    })?;
                status_updater
                    .propagate_info("An ephemeral authority account will be created and funded to write to the buffer account.");
                status_updater
                    .propagate_info("Please save the following information in case the deployment fails and the account needs to be recovered:");
                status_updater.propagate_info(&format!(
                    "Ephemeral authority public key: {}",
                    temp_authority_keypair.pubkey()
                ));
                status_updater.propagate_info(&format!(
                    "Ephemeral authority secret key: {}",
                    temp_authority_keypair.to_base58_string()
                ));
            }
            _ => {}
        };

        status_updater.propagate_pending_status(&format!(
            "Sending transaction {}/{}",
            transaction_index + 1,
            transaction_count
        ));

        Ok(())
    }

    pub fn post_send_status_updates(&self, status_updater: &mut StatusUpdater, program_id: Pubkey) {
        match self.transaction_type {
            DeploymentTransactionType::CreateTempAuthority(_) => {
                status_updater.propagate_status(ProgressBarStatus::new_msg(
                    ProgressBarStatusColor::Green,
                    "Account Created",
                    "Ephemeral authority account created to write to buffer",
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
                    "Ephemeral authority account closed and leftover funds returned to payer",
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
///  - Ephemeral authority: this is a temporary keypair that we use to write to the buffer account.
///    - This prevents the user from having to sign many transactions to deploy/upgrade.
///    - Once this account is created/funded, we need to be sure to display the keys to the user. If the deployment fails, we don't want them to lose this account
///  - Buffer account: this is the account that is created to temporarily write data to before deploying to the final program address
///  - Payer: this is who pays to fund the temp authority
///
/// ## First Deployment
///  
/// ### Transaction 1: Seed Ephemeral authority
///  1. This is signed by the payer
///
/// ### Transaction 2: Create Undeployed Program Account
///  1. First instruction creates the Undeployed Program account
///     1. Signed by the Ephemeral authority
///  2. Second instruction sets the program length
///
/// ### Transaction 3 - X: Write to Undeployed Program Account
///  1. Ephemeral authority writes to Undeployed Program Account
///
/// ### Transaction X + 1: Deploy Program          
///  1. Mark the Undeployed Program Account as executable
///     1. Signed by the Ephemeral authority
///  2. Transfer program authority from Ephemeral authority to Final Authority
///     1. Both current authority (Ephemeral authority) and new authority (Final Authority) sign
///
/// ### Transaction X + 2: Transfer leftover Ephemeral authority funds to the Payer
///  1. Ephemeral authority signs
/// ---
///
/// ## Upgrades
///
/// ### Transaction 1: Seed Ephemeral authority
///  1. This is signed by the payer
///
/// ### Transaction 2: Create Buffer
///  1. First instruction creates the Buffer account, with the Ephemeral authority as the authority
///     1. Signed by the Ephemeral authority
///  2. Second instruction sets the program length
///
/// ### Transaction 3 - X: Write to Buffer
///  1. Ephemeral authority writes to Buffer
///
/// ### Transaction X + 1: Deploy Program (from source)
///  1. Instruction to deploy the program from the buffer account
///
/// ### Transaction X + 2: Transfer leftover Ephemeral authority funds to the Payer
///  1. Ephemeral authority signs
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
    pub buffer_pubkey: Option<Pubkey>,
    /// The keypair of the buffer account.
    pub buffer_keypair: Option<Keypair>,
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
        let is_program_upgrade = !UpgradeableProgramDeployer::should_do_initial_deploy(
            &rpc_client,
            &program_keypair.pubkey(),
            &final_upgrade_authority_pubkey,
        )?;

        let (buffer_pubkey, buffer_keypair, buffer_data) = if is_program_upgrade {
            // if we're doing a program upgrade, we'll need a buffer account to write the new program to.
            // if the user provided an existing buffer account, use it. Otherwise, create a new one.
            match existing_program_buffer_opts {
                Some((buffer_pubkey, buffer_keypair, buffer_data)) => {
                    (Some(buffer_pubkey), Some(buffer_keypair), buffer_data)
                }
                None => {
                    let (_buffer_words, _buffer_mnemonic, buffer_keypair) =
                        create_ephemeral_keypair();
                    (Some(buffer_keypair.pubkey()), Some(buffer_keypair), vec![0; binary.len()])
                }
            }
        } else {
            // initial deployments don't need a buffer - we can just write to the program account directly.
            (None, None, vec![0; binary.len()])
        };

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

                // create the buffer account (or for a deployment, initialize the program undeployed account)
                let create_account_transaction =
                    self.get_create_buffer_transaction(&recent_blockhash)?;

                // write transaction data to the buffer account
                let mut write_transactions =
                    self.get_write_to_buffer_transactions(&recent_blockhash)?;

                // deploy the program, with the final authority as program authority
                let deploy_and_transfer_authority_transaction = self.get_deploy_program_and_set_final_authority_transaction(&recent_blockhash)?;

                let mut transactions = vec![create_account_transaction];
                transactions.append(&mut write_transactions);
                transactions.push(deploy_and_transfer_authority_transaction);
                transactions
            }
            // transactions for upgrading an existing program
            else {

                // create the buffer account
                let create_account_transaction =
                    self.get_create_buffer_transaction(&recent_blockhash)?;

                // write transaction data to the buffer account
                let mut write_transactions =
                    self.get_write_to_buffer_transactions(&recent_blockhash)?;

                // transfer the buffer authority from the temp authority to the final authority
                let deploy_from_source_transaction = self.get_deploy_program_from_source_transaction(&recent_blockhash)?;


                let mut transactions = vec![create_account_transaction];
                transactions.append(&mut write_transactions);
                transactions.push(deploy_from_source_transaction);
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
            let finalize_tx_count = 1;
            let write_tx_count = transaction_count - buffer_create_tx_count - finalize_tx_count;
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
                * (buffer_create_tx_count + write_tx_count + return_funds_tx_count) as u64;

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
            .get_minimum_balance_for_rent_exemption(
                LoaderV4State::program_data_offset().saturating_add(program_data_length),
            )
            .map_err(|e| {
                diagnosed_error!("failed to get minimum balance for rent exemption: {e}")
            })?;

        if self.is_program_upgrade {
            if let Some(buffer_pubkey) = &self.buffer_pubkey {
                Ok(loader_v4::create_buffer(
                    &self.temp_upgrade_authority_pubkey,
                    &buffer_pubkey,
                    rent_lamports,
                    &self.temp_upgrade_authority_pubkey,
                    program_data_length as u32,
                    &self.payer_pubkey,
                ))
            } else {
                return Err(diagnosed_error!("buffer pubkey not set for program upgrade"));
            }
        } else {
            Ok(loader_v4::create_buffer(
                &self.temp_upgrade_authority_pubkey,
                &self.program_pubkey,
                rent_lamports,
                &self.temp_upgrade_authority_pubkey,
                program_data_length as u32,
                &self.payer_pubkey,
            ))
        }
    }

    fn get_create_buffer_transaction(&self, blockhash: &Hash) -> Result<Value, Diagnostic> {
        let create_buffer_instruction = self.get_create_buffer_instruction()?;

        let message = Message::new_with_blockhash(
            &create_buffer_instruction,
            Some(&self.temp_upgrade_authority_pubkey), // todo: can this be none? isn't the payer already set in the instruction
            &blockhash,
        );

        let transaction = Transaction::new_unsigned(message);

        if self.is_program_upgrade {
            DeploymentTransaction::create_buffer(
                &transaction,
                vec![
                    &self.temp_upgrade_authority,
                    &self
                        .buffer_keypair
                        .as_ref()
                        .expect("buffer keypair not set for program upgrade"),
                ],
            )
            .to_value()
        } else {
            DeploymentTransaction::create_buffer(
                &transaction,
                vec![&self.temp_upgrade_authority, &self.program_keypair],
            )
            .to_value()
        }
    }

    // Mostly copied from solana cli: https://github.com/txtx/solana/blob/8116c10021f09c806159852f65d37ffe6d5a118e/cli/src/program.rs#L2455
    fn get_write_to_buffer_transactions(&self, blockhash: &Hash) -> Result<Vec<Value>, Diagnostic> {
        let create_msg = |offset: u32, bytes: Vec<u8>| {
            let instruction = if self.is_program_upgrade {
                bpf_loader_upgradeable::write(
                    &self.buffer_pubkey.expect("buffer pubkey not set for program upgrade"),
                    &self.temp_upgrade_authority_pubkey,
                    offset,
                    bytes,
                )
            } else {
                bpf_loader_upgradeable::write(
                    &self.program_pubkey,
                    &self.temp_upgrade_authority_pubkey,
                    offset,
                    bytes,
                )
            };

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

    fn get_deploy_program_and_set_final_authority_transaction(
        &self,
        blockhash: &Hash,
    ) -> Result<Value, Diagnostic> {
        let deploy_instruction =
            loader_v4::deploy(&self.program_pubkey, &self.temp_upgrade_authority_pubkey);

        let transfer_authority_instruction = loader_v4::transfer_authority(
            &self.program_pubkey,
            &self.temp_upgrade_authority_pubkey,
            &self.final_upgrade_authority_pubkey,
        );

        let message = Message::new_with_blockhash(
            &[deploy_instruction, transfer_authority_instruction],
            Some(&self.temp_upgrade_authority_pubkey),
            &blockhash,
        );

        let transaction = Transaction::new_unsigned(message);

        DeploymentTransaction::deploy_program(&transaction, vec![&self.temp_upgrade_authority])
            .to_value()
    }

    fn get_deploy_program_from_source_transaction(
        &self,
        blockhash: &Hash,
    ) -> Result<Value, Diagnostic> {
        let deploy_from_source_instruction = loader_v4::deploy_from_source(
            &self.program_pubkey,
            &self.final_upgrade_authority_pubkey,
            &self.buffer_pubkey.expect("buffer pubkey not set for program upgrade"),
        );

        let message = Message::new_with_blockhash(
            &[deploy_from_source_instruction],
            Some(&self.temp_upgrade_authority_pubkey),
            &blockhash,
        );

        let transaction = Transaction::new_unsigned(message);

        DeploymentTransaction::upgrade_program(&transaction, vec![&self.temp_upgrade_authority])
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
            if !loader_v4::check_id(&account.owner) {
                return Err(diagnosed_error!(
                    "account {} is already in use by another program",
                    program_pubkey
                )
                .into());
            }
            if let Ok(LoaderV4State { slot: _, authority_address_or_next_version, status }) =
                solana_loader_v4_program::get_state(&account.data)
            {
                if final_upgrade_authority_pubkey != authority_address_or_next_version {
                    return Err(
                        diagnosed_error!("the authority ({}) of program ({}) does not match with the provided authority ({})", authority_address_or_next_version, program_pubkey, final_upgrade_authority_pubkey)
                            ,
                    );
                }
                return match status {
                    LoaderV4Status::Retracted => Ok(true),
                    LoaderV4Status::Deployed => Ok(true),
                    LoaderV4Status::Finalized => Err(diagnosed_error!(
                        "program ({}) is already finalized and cannot be upgraded",
                        program_pubkey
                    )),
                };
            } else {
                return Err(diagnosed_error!(
                    "could not deserialize state for program account ({})",
                    program_pubkey
                ));
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

pub enum ProgramArtifacts {
    Native(ClassicRustProgramArtifacts),
    Anchor(AnchorProgramArtifacts),
}

impl ProgramArtifacts {
    pub fn from_value(value: &Value) -> Result<Self, Diagnostic> {
        let map =
            value.as_object().ok_or(diagnosed_error!("program artifacts must be an object"))?;

        let framework = map
            .get("framework")
            .ok_or(diagnosed_error!("program artifacts must have a 'framework' field"))?
            .as_string()
            .ok_or(diagnosed_error!("'framework' field must be a string"))?;

        match framework {
            "native" => {
                let artifacts = ClassicRustProgramArtifacts::from_value(value)?;
                Ok(ProgramArtifacts::Native(artifacts))
            }
            "anchor" => {
                let artifacts = AnchorProgramArtifacts::from_map(map)?;
                Ok(ProgramArtifacts::Anchor(artifacts))
            }
            _ => Err(diagnosed_error!("unsupported framework: {framework}")),
        }
    }
    pub fn keypair(&self) -> Result<Keypair, Diagnostic> {
        let keypair_bytes = self.keypair_bytes();
        Keypair::from_bytes(&keypair_bytes)
            .map_err(|e| diagnosed_error!("failed to deserialize keypair: {e}"))
    }
    pub fn keypair_bytes(&self) -> Vec<u8> {
        match self {
            ProgramArtifacts::Native(artifacts) => artifacts.keypair.to_bytes().to_vec(),
            ProgramArtifacts::Anchor(artifacts) => artifacts.keypair.to_bytes().to_vec(),
        }
    }
    pub fn bin(&self) -> &Vec<u8> {
        match self {
            ProgramArtifacts::Native(artifacts) => &artifacts.bin,
            ProgramArtifacts::Anchor(artifacts) => &artifacts.bin,
        }
    }
    pub fn idl(&self) -> Result<Option<String>, Diagnostic> {
        match self {
            ProgramArtifacts::Native(_) => Ok(None),
            ProgramArtifacts::Anchor(artifacts) => Some(
                serde_json::to_string(&artifacts.idl)
                    .map_err(|e| diagnosed_error!("invalid anchor idl: {e}")),
            )
            .transpose(),
        }
    }
}
