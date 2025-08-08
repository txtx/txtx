use std::collections::HashMap;

use anchor_lang_idl::types::{Idl, IdlInstruction, IdlInstructionAccount};
use serde::{Deserialize, Serialize};
use solana_clock::Slot;
use solana_message::compiled_instruction::CompiledInstruction;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_transaction_status_client_types::TransactionStatusMeta;
use txtx_addon_kit::{
    diagnosed_error,
    types::{diagnostics::Diagnostic, types::Value},
};

use crate::subgraph::{
    find_idl_instruction_account, idl::match_idl_accounts, IntrinsicField, SubgraphRequest,
    SubgraphSourceType, LAMPORTS_INTRINSIC_FIELD, OWNER_INTRINSIC_FIELD, PUBKEY_INTRINSIC_FIELD,
    SLOT_INTRINSIC_FIELD, TOKEN_AMOUNT_INTRINSIC_FIELD, TOKEN_DECIMALS_INTRINSIC_FIELD,
    TOKEN_MINT_INTRINSIC_FIELD, TOKEN_PROGRAM_INTRINSIC_FIELD, TOKEN_UI_AMOUNT_INTRINSIC_FIELD,
    TOKEN_UI_AMOUNT_STRING_INTRINSIC_FIELD, TRANSACTION_SIGNATURE_INTRINSIC_FIELD,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenAccountSubgraphSource {
    /// The account definitions from the instructions that use this account type.
    /// Each account definition should have the same `pda` definition.
    pub instruction_accounts: Vec<(
        anchor_lang_idl::types::IdlInstruction,
        anchor_lang_idl::types::IdlInstructionAccount,
    )>,
}

impl SubgraphSourceType for TokenAccountSubgraphSource {
    fn intrinsic_fields() -> Vec<IntrinsicField> {
        vec![
            SLOT_INTRINSIC_FIELD.clone(),
            TRANSACTION_SIGNATURE_INTRINSIC_FIELD.clone(),
            PUBKEY_INTRINSIC_FIELD.clone(),
            LAMPORTS_INTRINSIC_FIELD.clone(),
            OWNER_INTRINSIC_FIELD.clone(),
            TOKEN_MINT_INTRINSIC_FIELD.clone(),
            TOKEN_PROGRAM_INTRINSIC_FIELD.clone(),
            TOKEN_AMOUNT_INTRINSIC_FIELD.clone(),
            TOKEN_DECIMALS_INTRINSIC_FIELD.clone(),
            TOKEN_UI_AMOUNT_INTRINSIC_FIELD.clone(),
            TOKEN_UI_AMOUNT_STRING_INTRINSIC_FIELD.clone(),
        ]
    }
}

impl TokenAccountSubgraphSource {
    pub fn from_value(value: &Value, idl: &Idl) -> Result<(Self, Option<Vec<Value>>), Diagnostic> {
        let token_account_map = value
            .as_map()
            .ok_or(diagnosed_error!("subgraph 'token_account' field must be a map"))?;

        if token_account_map.len() != 1 {
            return Err(diagnosed_error!("exactly one 'token_account' map should be defined"));
        }
        let entry = token_account_map.get(0).unwrap();

        let entry = entry
            .as_object()
            .ok_or(diagnosed_error!("a subgraph 'token_account' field should contain an object"))?;

        let instruction_account_path = entry.get("instruction").and_then(|v| v.as_map()).ok_or(
            diagnosed_error!("a subgraph 'token_account' field must have an 'instruction' map"),
        )?;

        let mut instruction_values = Vec::with_capacity(instruction_account_path.len());
        for instruction_value in instruction_account_path.iter() {
            let instruction_value = instruction_value.as_object().ok_or(diagnosed_error!(
                "each entry of a subgraph 'token_account' instruction should contain an object"
            ))?;
            let instruction_name = instruction_value.get("name").ok_or(diagnosed_error!(
                "a subgraph 'token_account' instruction must have a 'name' key"
            ))?;
            let instruction_name = instruction_name.as_string().ok_or(diagnosed_error!(
                "a subgraph 'token_account' instruction's 'name' value must be a string"
            ))?;
            let account_name = instruction_value.get("account_name").ok_or(diagnosed_error!(
                "a subgraph 'token_account' instruction must have an 'account_name' key"
            ))?;
            let account_name = account_name.as_string().ok_or(diagnosed_error!(
                "a subgraph 'token_account' instruction's 'account_name' value must be a string"
            ))?;
            instruction_values.push((instruction_name, account_name));
        }
        let token_account_source = Self::new(&instruction_values, idl)?;
        let intrinsic_fields =
            entry.get("intrinsic_field").and_then(|v| v.as_map().map(|s| s.to_vec()));
        Ok((token_account_source, intrinsic_fields))
    }

    pub fn new(instruction_account_path: &[(&str, &str)], idl: &Idl) -> Result<Self, Diagnostic> {
        let mut instruction_accounts = vec![];
        for (instruction_name, account_name) in instruction_account_path {
            let instruction = idl.instructions.iter().find(|i| i.name.eq(instruction_name)).ok_or(
                diagnosed_error!("could not find instruction '{}' in IDL", instruction_name),
            )?;
            let account_item = instruction
                .accounts
                .iter()
                .find_map(|a| find_idl_instruction_account(a, account_name))
                .ok_or(diagnosed_error!(
                    "could not find account '{}' in instruction '{}' in IDL",
                    account_name,
                    instruction_name
                ))?;

            if account_item.pda.is_none() {
                return Err(diagnosed_error!(
                    "account '{}' in instruction '{}' is not a PDA",
                    account_name,
                    instruction_name
                ));
            }

            if instruction_accounts.len() > 1 {
                let last: &(IdlInstruction, IdlInstructionAccount) =
                    instruction_accounts.last().unwrap();
                if last.1.pda != account_item.pda {
                    return Err(diagnosed_error!(
                        "account '{}' in instruction '{}' has different PDA definitions",
                        account_name,
                        instruction_name
                    ));
                }
            }

            instruction_accounts.push((instruction.clone(), account_item));
        }
        Ok(Self { instruction_accounts })
    }

    pub fn evaluate_instruction(
        &self,
        instruction: &CompiledInstruction,
        account_pubkeys: &[Pubkey],
        transaction_status_meta: &TransactionStatusMeta,
        slot: Slot,
        transaction_signature: Signature,
        subgraph_request: &SubgraphRequest,
        already_found_token_accounts: &mut Vec<Pubkey>,
        entries: &mut Vec<HashMap<String, Value>>,
    ) -> Result<(), String> {
        let SubgraphRequest::V0(subgraph_request) = subgraph_request;
        let Some((matching_idl_instruction, idl_instruction_account)) =
            self.instruction_accounts.iter().find_map(|(ix, ix_account)| {
                if instruction.data.starts_with(&ix.discriminator) {
                    Some((ix, ix_account))
                } else {
                    None
                }
            })
        else {
            // This instruction does not match any of the instructions that use this Token Account
            return Ok(());
        };

        let idl_accounts =
            match_idl_accounts(matching_idl_instruction, &instruction.accounts, &account_pubkeys);

        let Some((token_account_pubkey, token_account_index)) =
            idl_accounts.iter().find_map(|(name, pubkey, index)| {
                if idl_instruction_account.name.eq(name) {
                    Some((*pubkey, index))
                } else {
                    None
                }
            })
        else {
            return Ok(());
        };

        if already_found_token_accounts.contains(&token_account_pubkey) {
            // This token account has already been processed, prevent double processing
            return Ok(());
        }

        let some_pre_balance_entry = if let Some(pre_token_balances) =
            transaction_status_meta.pre_token_balances.as_ref()
        {
            if let Some(pre_token_balance) =
                pre_token_balances.iter().find(|b| b.account_index == *token_account_index as u8)
            {
                let mint = Pubkey::from_str_const(&pre_token_balance.mint);
                let owner = Pubkey::from_str_const(&pre_token_balance.owner);
                let token_program = Pubkey::from_str_const(&pre_token_balance.program_id);
                let amount = pre_token_balance.ui_token_amount.amount.clone();
                let decimals = pre_token_balance.ui_token_amount.decimals;
                let ui_amount = pre_token_balance.ui_token_amount.ui_amount;
                let ui_amount_string = pre_token_balance.ui_token_amount.ui_amount_string.clone();
                let lamports = transaction_status_meta
                    .pre_balances
                    .get(*token_account_index)
                    .ok_or(format!(
                        "could not find pre-balance for token account {}",
                        token_account_pubkey
                    ))?;
                let mut entry = HashMap::new();
                subgraph_request.intrinsic_fields.iter().for_each(|field| {
                    if let Some((entry_key, entry_value)) = field.extract_intrinsic(
                        Some(slot),
                        Some(transaction_signature),
                        Some(token_account_pubkey),
                        Some(owner),
                        Some(*lamports),
                        None,
                        Some(mint),
                        Some(token_program),
                        Some(amount.clone()),
                        Some(decimals),
                        ui_amount,
                        Some(ui_amount_string.clone()),
                    ) {
                        entry.insert(entry_key, entry_value);
                    }
                });
                Some(entry)
            } else {
                None
            }
        } else {
            None
        };

        let some_post_balance_entry = if let Some(post_token_balances) =
            transaction_status_meta.post_token_balances.as_ref()
        {
            if let Some(post_token_balance) =
                post_token_balances.iter().find(|b| b.account_index == *token_account_index as u8)
            {
                let mint = Pubkey::from_str_const(&post_token_balance.mint);
                let owner = Pubkey::from_str_const(&post_token_balance.owner);
                let token_program = Pubkey::from_str_const(&post_token_balance.program_id);
                let amount = post_token_balance.ui_token_amount.amount.clone();
                let decimals = post_token_balance.ui_token_amount.decimals;
                let ui_amount = post_token_balance.ui_token_amount.ui_amount;
                let ui_amount_string = post_token_balance.ui_token_amount.ui_amount_string.clone();
                let lamports = transaction_status_meta
                    .post_balances
                    .get(*token_account_index)
                    .ok_or(format!(
                        "could not find post-balance for token account {}",
                        token_account_pubkey
                    ))?;
                let mut entry = HashMap::new();
                subgraph_request.intrinsic_fields.iter().for_each(|field| {
                    if let Some((entry_key, entry_value)) = field.extract_intrinsic(
                        Some(slot),
                        Some(transaction_signature),
                        Some(token_account_pubkey),
                        Some(owner),
                        Some(*lamports),
                        None,
                        Some(mint),
                        Some(token_program),
                        Some(amount.clone()),
                        Some(decimals),
                        ui_amount,
                        Some(ui_amount_string.clone()),
                    ) {
                        entry.insert(entry_key, entry_value);
                    }
                });
                Some(entry)
            } else {
                None
            }
        } else {
            None
        };

        match (some_pre_balance_entry, some_post_balance_entry) {
            (Some(pre_entry), Some(post_entry)) => {
                if pre_entry.eq(&post_entry) {
                    // If both pre and post entries are the same, we only need to add one entry
                    entries.push(pre_entry);
                } else {
                    // If they are different, we add both
                    entries.push(pre_entry);
                    entries.push(post_entry);
                }
            }
            (Some(entry), None) | (None, Some(entry)) => {
                entries.push(entry);
            }
            (None, None) => {}
        }

        if !already_found_token_accounts.contains(&token_account_pubkey) {
            already_found_token_accounts.push(token_account_pubkey);
        }

        Ok(())
    }
}
