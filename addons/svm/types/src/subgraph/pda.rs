use std::collections::HashMap;

use anchor_lang_idl::types::{Idl, IdlInstruction, IdlInstructionAccount};
use serde::{Deserialize, Serialize};
use solana_clock::Slot;
use solana_message::compiled_instruction::CompiledInstruction;
use solana_pubkey::Pubkey;
use txtx_addon_kit::{
    diagnosed_error,
    types::{diagnostics::Diagnostic, types::Value},
};

use crate::subgraph::{
    find_idl_instruction_account,
    idl::{match_idl_accounts, parse_bytes_to_value_with_expected_idl_type_def_ty},
    IntrinsicField, SubgraphRequest, SubgraphSourceType, LAMPORTS_INTRINSIC_FIELD,
    OWNER_INTRINSIC_FIELD, PUBKEY_INTRINSIC_FIELD, SLOT_INTRINSIC_FIELD,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdaSubgraphSource {
    /// The account being indexed
    pub account: anchor_lang_idl::types::IdlAccount,
    /// The type of the account
    pub account_type: anchor_lang_idl::types::IdlTypeDef,
    /// The account definitions from the instructions that use this account type.
    /// Each account definition should have the same `pda` definition.
    pub instruction_accounts: Vec<(
        anchor_lang_idl::types::IdlInstruction,
        anchor_lang_idl::types::IdlInstructionAccount,
    )>,
}

impl SubgraphSourceType for PdaSubgraphSource {
    fn intrinsic_fields() -> Vec<IntrinsicField> {
        vec![
            SLOT_INTRINSIC_FIELD.clone(),
            PUBKEY_INTRINSIC_FIELD.clone(),
            LAMPORTS_INTRINSIC_FIELD.clone(),
            OWNER_INTRINSIC_FIELD.clone(),
        ]
    }
}

impl PdaSubgraphSource {
    pub fn from_value(
        value: &Value,
        idl: &Idl,
    ) -> Result<(Self, Option<Vec<Value>>, Option<Vec<Value>>), Diagnostic> {
        let pda_map =
            value.as_map().ok_or(diagnosed_error!("subgraph 'pda' field must be a map"))?;

        if pda_map.len() != 1 {
            return Err(diagnosed_error!("exactly one 'pda' map should be defined"));
        }
        let entry = pda_map.get(0).unwrap();

        let entry = entry
            .as_object()
            .ok_or(diagnosed_error!("a subgraph 'pda' field should contain an object"))?;

        let type_name = entry
            .get("type")
            .ok_or(diagnosed_error!("a subgraph 'pda' field must have a 'type' key"))?;
        let type_name = type_name
            .as_string()
            .ok_or(diagnosed_error!("a subgraph 'pda' field's 'type' value must be a string"))?;
        let instruction_account_path = entry
            .get("instruction")
            .and_then(|v| v.as_map())
            .ok_or(diagnosed_error!("a subgraph 'pda' field must have an 'instruction' map"))?;

        let mut instruction_values = Vec::with_capacity(instruction_account_path.len());
        for instruction_value in instruction_account_path.iter() {
            let instruction_value = instruction_value.as_object().ok_or(diagnosed_error!(
                "each entry of a subgraph 'pda' instruction should contain an object"
            ))?;
            let instruction_name = instruction_value
                .get("name")
                .ok_or(diagnosed_error!("a subgraph 'pda' instruction must have a 'name' key"))?;
            let instruction_name = instruction_name.as_string().ok_or(diagnosed_error!(
                "a subgraph 'pda' instruction's 'name' value must be a string"
            ))?;
            let account_name = instruction_value.get("account_name").ok_or(diagnosed_error!(
                "a subgraph 'pda' instruction must have an 'account_name' key"
            ))?;
            let account_name = account_name.as_string().ok_or(diagnosed_error!(
                "a subgraph 'pda' instruction's 'account_name' value must be a string"
            ))?;
            instruction_values.push((instruction_name, account_name));
        }
        let pda_source = Self::new(type_name, &instruction_values, idl)?;
        let fields = entry.get("field").and_then(|v| v.as_map().map(|s| s.to_vec()));
        let intrinsic_fields =
            entry.get("intrinsic_field").and_then(|v| v.as_map().map(|s| s.to_vec()));
        Ok((pda_source, fields, intrinsic_fields))
    }

    pub fn new(
        account_name: &str,
        instruction_account_path: &[(&str, &str)],
        idl: &Idl,
    ) -> Result<Self, Diagnostic> {
        let account = idl
            .accounts
            .iter()
            .find(|a| a.name == account_name)
            .cloned()
            .ok_or(diagnosed_error!("could not find account '{}' in IDL", account_name))?;
        let account_type = idl
            .types
            .iter()
            .find(|t| t.name == account_name)
            .cloned()
            .ok_or(diagnosed_error!("could not find type '{}' in IDL", account_name))?;

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
        Ok(Self { account, account_type, instruction_accounts })
    }

    pub fn evaluate_account_update(
        &self,
        data: &[u8],
        subgraph_request: &SubgraphRequest,
        slot: Slot,
        pubkey: Pubkey,
        owner: Pubkey,
        lamports: u64,
        entries: &mut Vec<HashMap<String, Value>>,
    ) -> Result<(), String> {
        let SubgraphRequest::V0(subgraph_request) = subgraph_request;
        let actual_account_discriminator = data[0..8].to_vec();
        if actual_account_discriminator != self.account.discriminator {
            // This is not the expected account, so we skip it
            return Ok(());
        }
        let rest = data[8..].to_vec();

        let idl_type_def_generics = subgraph_request
            .idl_types
            .iter()
            .find(|t| t.name == self.account_type.name)
            .map(|t| &t.generics);
        let empty_vec = vec![];
        let parsed_value = parse_bytes_to_value_with_expected_idl_type_def_ty(
            &rest,
            &self.account_type.ty,
            &subgraph_request.idl_types,
            &vec![],
            idl_type_def_generics.unwrap_or(&empty_vec),
        )?;

        let obj = parsed_value.as_object().unwrap().clone();
        let mut entry = HashMap::new();
        for field in subgraph_request.defined_fields.iter() {
            let v = obj.get(&field.source_key).unwrap().clone();
            entry.insert(field.display_name.clone(), v);
        }

        subgraph_request.intrinsic_fields.iter().for_each(|field| {
            if let Some((entry_key, entry_value)) = field.extract_intrinsic(
                Some(slot),
                None,
                Some(pubkey),
                Some(owner),
                Some(lamports),
                None,
                None,
                None,
                None,
                None,
                None,
            ) {
                entry.insert(entry_key, entry_value);
            }
        });

        if !entry.is_empty() {
            entries.push(entry);
        }

        Ok(())
    }

    pub fn evaluate_instruction(
        &self,
        instruction: &CompiledInstruction,
        account_pubkeys: &[Pubkey],
    ) -> Option<Pubkey> {
        let Some((matching_idl_instruction, idl_instruction_account)) =
            self.instruction_accounts.iter().find_map(|(ix, ix_account)| {
                if instruction.data.starts_with(&ix.discriminator) {
                    Some((ix, ix_account))
                } else {
                    None
                }
            })
        else {
            // This instruction does not match any of the instructions that use this PDA account type
            return None;
        };

        let idl_accounts =
            match_idl_accounts(matching_idl_instruction, &instruction.accounts, &account_pubkeys);
        let some_pda = idl_accounts.iter().find_map(|(name, pubkey, _)| {
            if idl_instruction_account.name.eq(name) {
                Some(*pubkey)
            } else {
                None
            }
        });

        some_pda
    }
}
