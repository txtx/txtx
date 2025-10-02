use std::collections::HashMap;

use multisig::Multisig;
use proposal::{get_proposal_ix_data, Proposal, ProposalStatus};
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::{GetConfirmedSignaturesForAddress2Config, RpcClient};
use solana_instruction::{AccountMeta, Instruction};
use solana_message::Message;
use solana_pubkey::Pubkey;
use solana_sdk_ids::system_program;
use solana_transaction::Transaction;
use txtx_addon_kit::types::{diagnostics::Diagnostic, types::Value, ConstructDid};
use txtx_addon_network_svm_types::{SvmValue, SVM_SQUAD_MULTISIG};
use vault_transaction::get_create_vault_transaction_ix_data;

use crate::codec::ui_encode::message_to_formatted_tx;

pub mod compiled_keys;
pub mod multisig;
pub mod pda;
pub mod proposal;
pub mod small_vec;
pub mod vault_transaction;

const SQUADS_MULTISIG_PROGRAM_ID: Pubkey =
    Pubkey::from_str_const("SQDS4ep65T869zMMBKyuUq6aD6EgTu8psMjkvj52pCf");
const DEFAULT_SQUADS_FRONTEND_URL: &str = "https://app.squads.so";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SquadsMultisig {
    pub multisig_pda: Pubkey,
    pub program_id: Pubkey,
    pub starting_transaction_index: u64,
    pub transaction_index_map: HashMap<ConstructDid, u64>,
    pub vault_index: u8,
    pub vault_pda: Pubkey,
    pub frontend_url: Option<String>,
}

impl SquadsMultisig {
    pub fn to_value(&self) -> Value {
        let bytes = serde_json::to_vec(self).unwrap();
        SvmValue::squads_multisig(bytes)
    }

    pub fn from_value(value: &Value) -> Self {
        let addon_data = value.expect_addon_data();
        if addon_data.id != SVM_SQUAD_MULTISIG {
            panic!("expected squads multisig, got {}", addon_data.id);
        }
        serde_json::from_slice::<Self>(&addon_data.bytes)
            .expect("failed to deserialize squads multisig")
    }

    fn fetch_multisig_account(
        rpc_client: &RpcClient,
        multisig_pda: &Pubkey,
    ) -> Result<Multisig, Diagnostic> {
        let multisig_account = rpc_client.get_account(multisig_pda).map_err(|e| {
            diagnosed_error!("failed to get multisig account '{multisig_pda}': {e}")
        })?;

        Multisig::checked_deserialize(&multisig_account.data)
            .map_err(|e| diagnosed_error!("invalid multisig account data at '{multisig_pda}': {e}"))
    }

    fn get_proposal_pda(&self, current_transaction_index: u64) -> Pubkey {
        pda::get_proposal_pda(&self.multisig_pda, current_transaction_index, Some(&self.program_id))
            .0
    }

    fn get_transaction_pda(&self, current_transaction_index: u64) -> Pubkey {
        pda::get_transaction_pda(
            &self.multisig_pda,
            current_transaction_index,
            Some(&self.program_id),
        )
        .0
    }

    pub fn from_multisig_pda(
        rpc_client: RpcClient,
        multisig_pda: &Pubkey,
        vault_index: u8,
        program_id: Option<&Pubkey>,
        squads_frontend_url: Option<&str>,
    ) -> Result<Self, Diagnostic> {
        let multisig = Self::fetch_multisig_account(&rpc_client, multisig_pda)?;

        let program_id = *program_id.unwrap_or(&SQUADS_MULTISIG_PROGRAM_ID);

        let (vault_pda, _) = pda::get_vault_pda(&multisig_pda, vault_index, Some(&program_id));

        Ok(Self {
            multisig_pda: *multisig_pda,
            program_id,
            starting_transaction_index: multisig.transaction_index,
            transaction_index_map: HashMap::new(),
            vault_index,
            vault_pda,
            frontend_url: squads_frontend_url.map(|u| u.to_string()),
        })
    }

    pub fn from_create_key(
        rpc_client: RpcClient,
        create_key: &Pubkey,
        vault_index: u8,
        program_id: Option<&Pubkey>,
        squads_frontend_url: Option<&str>,
    ) -> Result<Self, Diagnostic> {
        let (multisig_pda, _) = pda::get_multisig_pda(create_key, program_id);
        Self::from_multisig_pda(
            rpc_client,
            &multisig_pda,
            vault_index,
            program_id,
            squads_frontend_url,
        )
    }

    pub fn get_transaction(
        &mut self,
        rpc_client: RpcClient,
        construct_did: &ConstructDid,
        initiator: &Pubkey,
        rent_payer: &Pubkey,
        mut input_message: Message,
    ) -> Result<(Value, Value), Diagnostic> {
        let latest_blockhash = rpc_client.get_latest_blockhash().map_err(|e| {
            diagnosed_error!("failed to retrieve latest blockhash: {}", e.to_string())
        })?;

        input_message.recent_blockhash = latest_blockhash;
        let map_len = self.transaction_index_map.len();
        let transaction_index =
            self.transaction_index_map.get(construct_did).map(|idx| *idx).unwrap_or_else(|| {
                let idx = self.starting_transaction_index + map_len as u64 + 1;
                self.transaction_index_map.insert(construct_did.clone(), idx);
                idx
            });

        let mut message = Message::new(
            &vec![
                self.get_create_vault_transaction_ix(
                    transaction_index,
                    initiator,
                    rent_payer,
                    input_message.clone(),
                )
                .map_err(|e| {
                    diagnosed_error!("failed to create vault transaction instruction: {}", e)
                })?,
                self.get_create_proposal_ix(transaction_index, initiator, rent_payer),
            ],
            None,
        );

        let formatted_transaction = message_to_formatted_tx(&message);

        message.recent_blockhash = latest_blockhash;
        let tx_value = SvmValue::transaction(&Transaction::new_unsigned(message))?;
        Ok((tx_value, formatted_transaction))
    }

    fn get_create_proposal_ix(
        &self,
        transaction_index: u64,
        initiator: &Pubkey,
        rent_payer: &Pubkey,
    ) -> Instruction {
        let proposal_pda = self.get_proposal_pda(transaction_index);
        Instruction::new_with_bytes(
            self.program_id,
            &get_proposal_ix_data(transaction_index, false),
            vec![
                AccountMeta::new_readonly(self.multisig_pda, false),
                AccountMeta::new(proposal_pda, false),
                AccountMeta::new_readonly(*initiator, true),
                AccountMeta::new(*rent_payer, true),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
        )
    }

    fn get_create_vault_transaction_ix(
        &self,
        transaction_index: u64,
        initiator: &Pubkey,
        rent_payer: &Pubkey,
        message: Message,
    ) -> Result<Instruction, Diagnostic> {
        let transaction_pda = self.get_transaction_pda(transaction_index);
        Ok(Instruction::new_with_bytes(
            self.program_id,
            &get_create_vault_transaction_ix_data(
                &self.vault_pda,
                self.vault_index,
                0,
                message.clone(),
                None,
            )?,
            vec![
                AccountMeta::new(self.multisig_pda, false),
                AccountMeta::new(transaction_pda, false),
                AccountMeta::new_readonly(*initiator, true),
                AccountMeta::new(*rent_payer, true),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
        ))
    }

    pub fn get_proposal_status(
        &self,
        rpc_client: &RpcClient,
        construct_did: &ConstructDid,
    ) -> Result<ProposalStatus, Diagnostic> {
        let transaction_index = self
            .transaction_index_map
            .get(construct_did)
            .unwrap_or(&self.starting_transaction_index);
        let proposal_pda = self.get_proposal_pda(*transaction_index);

        let proposal_account = rpc_client.get_account(&proposal_pda).map_err(|e| {
            diagnosed_error!("failed to get proposal account '{}': {}", proposal_pda, e)
        })?;

        let proposal = Proposal::checked_deserialize(&proposal_account.data).map_err(|e| {
            diagnosed_error!("invalid proposal account data at '{}': {}", proposal_pda, e)
        })?;

        Ok(proposal.status)
    }

    pub fn vault_transaction_url(&self, construct_did: &ConstructDid) -> String {
        let transaction_index = self
            .transaction_index_map
            .get(construct_did)
            .unwrap_or(&self.starting_transaction_index);

        let current_transaction_pda = self.get_transaction_pda(*transaction_index);
        if let Some(frontend_url) = &self.frontend_url {
            frontend_url.clone()
        } else {
            format!(
                "{}/squads/{}/transactions/{}",
                DEFAULT_SQUADS_FRONTEND_URL, self.vault_pda, current_transaction_pda
            )
        }
    }

    pub fn get_executed_signature(&self, rpc_client: &RpcClient) -> Option<String> {
        rpc_client
            .get_signatures_for_address_with_config(
                &self.vault_pda,
                GetConfirmedSignaturesForAddress2Config { limit: Some(1), ..Default::default() },
            )
            .ok()?
            .first()
            .map(|sig| sig.signature.clone())
    }
}
