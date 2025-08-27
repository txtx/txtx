use multisig::Multisig;
use proposal::{get_proposal_ix_data, Proposal, ProposalStatus};
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::{GetConfirmedSignaturesForAddress2Config, RpcClient};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    message::Message,
    pubkey::Pubkey,
    system_program,
    transaction::Transaction,
};
use txtx_addon_kit::types::{diagnostics::Diagnostic, types::Value};
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
    pub transaction_index: u64,
    pub vault_index: u8,
    pub proposal_pda: Pubkey,
    pub transaction_pda: Pubkey,
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

    pub fn from_multisig_pda(
        rpc_client: RpcClient,
        multisig_pda: &Pubkey,
        vault_index: u8,
        program_id: Option<&Pubkey>,
        squads_frontend_url: Option<&str>,
    ) -> Result<Self, Diagnostic> {
        let multisig_account = rpc_client.get_account(multisig_pda).map_err(|e| {
            diagnosed_error!("failed to get multisig account '{multisig_pda}': {e}")
        })?;

        let multisig = Multisig::checked_deserialize(&multisig_account.data).map_err(|e| {
            diagnosed_error!("invalid multisig account data at '{multisig_pda}': {e}")
        })?;
        let program_id = *program_id.unwrap_or(&SQUADS_MULTISIG_PROGRAM_ID);

        let (proposal_pda, _) =
            pda::get_proposal_pda(&multisig_pda, multisig.transaction_index + 1, Some(&program_id));

        let (transaction_pda, _) = pda::get_transaction_pda(
            &multisig_pda,
            multisig.transaction_index + 1,
            Some(&program_id),
        );

        let (vault_pda, _) = pda::get_vault_pda(&multisig_pda, vault_index, Some(&program_id));

        Ok(Self {
            multisig_pda: *multisig_pda,
            program_id,
            transaction_index: multisig.transaction_index + 1,
            starting_transaction_index: multisig.transaction_index,
            vault_index,
            proposal_pda,
            transaction_pda,
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
        &self,
        rpc_client: RpcClient,
        initiator: &Pubkey,
        rent_payer: &Pubkey,
        mut input_message: Message,
    ) -> Result<(Value, Value), Diagnostic> {
        let latest_blockhash = rpc_client.get_latest_blockhash().map_err(|e| {
            diagnosed_error!("failed to retrieve latest blockhash: {}", e.to_string())
        })?;
        input_message.recent_blockhash = latest_blockhash;

        let mut message = Message::new(
            &vec![
                self.get_create_vault_transaction_ix(initiator, rent_payer, input_message.clone())
                    .map_err(|e| {
                        diagnosed_error!("failed to create vault transaction instruction: {}", e)
                    })?,
                self.get_create_proposal_ix(initiator, rent_payer),
            ],
            None,
        );

        let formatted_transaction = message_to_formatted_tx(&message);

        message.recent_blockhash = latest_blockhash;
        let tx_value = SvmValue::transaction(&Transaction::new_unsigned(message))?;
        Ok((tx_value, formatted_transaction))
    }

    fn get_create_proposal_ix(&self, initiator: &Pubkey, rent_payer: &Pubkey) -> Instruction {
        Instruction::new_with_bytes(
            self.program_id,
            &get_proposal_ix_data(self.transaction_index, false),
            vec![
                AccountMeta::new_readonly(self.multisig_pda, false),
                AccountMeta::new(self.proposal_pda, false),
                AccountMeta::new_readonly(*initiator, true),
                AccountMeta::new(*rent_payer, true),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
        )
    }

    fn get_create_vault_transaction_ix(
        &self,
        initiator: &Pubkey,
        rent_payer: &Pubkey,
        message: Message,
    ) -> Result<Instruction, Diagnostic> {
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
                AccountMeta::new(self.transaction_pda, false),
                AccountMeta::new_readonly(*initiator, true),
                AccountMeta::new(*rent_payer, true),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
        ))
    }

    pub fn get_proposal_status(
        &self,
        rpc_client: &RpcClient,
    ) -> Result<ProposalStatus, Diagnostic> {
        let proposal_account = rpc_client.get_account(&self.proposal_pda).map_err(|e| {
            diagnosed_error!("failed to get proposal account '{}': {}", self.proposal_pda, e)
        })?;

        let proposal = Proposal::checked_deserialize(&proposal_account.data).map_err(|e| {
            diagnosed_error!("invalid proposal account data at '{}': {}", self.proposal_pda, e)
        })?;

        Ok(proposal.status)
    }

    pub fn vault_transaction_url(&self) -> String {
        if let Some(frontend_url) = &self.frontend_url {
            frontend_url.clone()
        } else {
            format!(
                "{}/squads/{}/transactions/{}",
                DEFAULT_SQUADS_FRONTEND_URL, self.vault_pda, self.transaction_pda
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
