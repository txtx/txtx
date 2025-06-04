use std::{collections::VecDeque, str::FromStr};

use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use txtx_addon_kit::{
    indexmap::IndexMap,
    types::{diagnostics::Diagnostic, stores::ValueStore, types::Value},
};
use txtx_addon_network_svm_types::anchor::types::{
    IdlInstruction, IdlInstructionAccount, IdlInstructionAccountItem, IdlSeed,
};

use crate::{
    constants::{INSTRUCTION, PROGRAM_ID, PUBLIC_KEY, SIGNER},
    typing::SvmValue,
};

use super::idl::IdlRef;

pub const RAW_BYTES: &str = "raw_bytes";
pub const DATA: &str = "data";
pub const ACCOUNTS: &str = "account";
pub const PROGRAM_IDL: &str = "program_idl";
pub const INSTRUCTION_NAME: &str = "instruction_name";
pub const INSTRUCTION_ARGS: &str = "instruction_args";

pub fn parse_instructions_map(values: &ValueStore) -> Result<Vec<Instruction>, Diagnostic> {
    let mut instructions = vec![];
    let mut instructions_data = values
        .get_expected_map(INSTRUCTION)?
        .iter()
        .map(|i| {
            i.as_object()
                .map(|o| o.clone())
                .ok_or(diagnosed_error!("'instruction' must be a map type"))
        })
        .collect::<Result<Vec<_>, _>>()?;

    for instruction_data in instructions_data.iter_mut() {
        // if the raw_bytes key was provided, treat it as a serialized instruction
        if let Some(value) = instruction_data.swap_remove(RAW_BYTES) {
            let instruction = serde_json::from_slice(&value.to_bytes())
                .map_err(|e| diagnosed_error!("failed to deserialize raw instruction: {e}"))?;
            instructions.push(instruction);
            continue;
        }
        let mut accounts: Vec<AccountMeta> = Vec::new();
        let program_id: Pubkey;
        let mut ix_data = Vec::new();

        if instruction_data.contains_key(PROGRAM_IDL) {
            let mut instruction_builder: InstructionBuilder;
            let some_program_idl = instruction_data.swap_remove(PROGRAM_IDL);
            let program_idl = some_program_idl
                .as_ref()
                .ok_or(diagnosed_error!("'program_idl' is required for each instruction"))?
                .as_string()
                .ok_or(diagnosed_error!(
                    "'program_idl' field for an instruction must be a string"
                ))?;

            let idl = IdlRef::from_str(program_idl)
                .map_err(|e| diagnosed_error!("failed to parse program idl: {e}"))?;

            let idl_pubkey = &idl.get_program_pubkey()?;

            program_id = instruction_data
                .swap_remove(PROGRAM_ID)
                .map(|p| {
                    SvmValue::to_pubkey(&p)
                        .map_err(|e| diagnosed_error!("invalid 'program_id' for instruction: {e}"))
                })
                .transpose()?
                .unwrap_or(idl_pubkey.clone());

            let some_instruction_name = instruction_data.swap_remove(INSTRUCTION_NAME);
            let instruction_name = some_instruction_name
                .as_ref()
                .ok_or(diagnosed_error!("'instruction_name' is required for each instruction"))?
                .as_string()
                .ok_or(diagnosed_error!(
                    "'instruction_name' field for an instruction must be a string"
                ))?;

            let some_instruction_args = instruction_data.swap_remove(INSTRUCTION_ARGS);
            let instruction_args = some_instruction_args
                .as_ref()
                .map(|v| {
                    v.as_array()
                        .ok_or(diagnosed_error!(
                            "'instruction_args' field for an instruction must be an array"
                        ))
                        .map(|a| a.to_vec())
                })
                .transpose()?
                .unwrap_or(vec![]);

            instruction_builder = InstructionBuilder::new(
                &idl,
                &program_id,
                instruction_name,
                instruction_args.to_vec(),
            )
            .map_err(|e| {
                diagnosed_error!("failed to build instruction '{instruction_name}': {e}")
            })?;
            accounts =
                instruction_builder.get_instruction_accounts(instruction_data).map_err(|e| {
                    diagnosed_error!(
                        "failed to get accounts for instruction '{instruction_name}': {e}"
                    )
                })?;
            ix_data = instruction_builder.get_instruction_data();
        } else {
            let value = instruction_data.swap_remove(DATA).expect("data not found");
            program_id = instruction_data
                .swap_remove(PROGRAM_ID)
                .map(|p| {
                    SvmValue::to_pubkey(&p)
                        .map_err(|e| diagnosed_error!("invalid 'program_id' for instruction: {e}"))
                })
                .transpose()?
                .unwrap();
            let _ = instruction_data.swap_remove(ACCOUNTS).iter().try_for_each(|acc| {
                let acc_obj = acc.as_array().unwrap(); //acc.expect_object();
                let _ = acc_obj.iter().try_for_each(|item| {
                    let item_obj = item.as_object().unwrap();
                    let public_key = item_obj.get("public_key").ok_or("public_key not found")?;
                    let public_key = SvmValue::to_pubkey(public_key)
                        .map_err(|e| diagnosed_error!("invalid 'account' for instruction: {e}"))?;
                    let is_writable = item_obj
                        .get("is_writable")
                        .unwrap_or(&Value::Bool(false))
                        .as_bool()
                        .unwrap();
                    let is_signer =
                        item_obj.get("is_signer").unwrap_or(&Value::Bool(false)).as_bool().unwrap();

                    let account_meta = AccountMeta { pubkey: public_key, is_signer, is_writable };
                    accounts.push(account_meta);
                    Ok::<(), Diagnostic>(())
                })?;
                Ok::<(), Diagnostic>(())
            })?;

            ix_data = value.to_bytes();
        }

        if accounts.is_empty() {
            let signer_value = instruction_data.swap_remove(SIGNER);
            if let Some(signer_value) = signer_value {
                let signer_map = signer_value
                    .as_map()
                    .ok_or(diagnosed_error!("each account field must be a map"))?;
                if signer_map.len() != 1 {
                    return Err(diagnosed_error!("each account field must have exactly one entry"));
                }
                let signer = signer_map
                    .first()
                    .unwrap()
                    .as_object()
                    .expect("expected map entry to be an object");

                let pubkey = signer
                    .get(PUBLIC_KEY)
                    .map(|p| SvmValue::to_pubkey(p))
                    .ok_or(diagnosed_error!("each account entry must have a 'public_key' field"))?
                    .map_err(|e| diagnosed_error!("invalid 'public_key': {e}"))?;
                accounts.push(AccountMeta { pubkey, is_signer: true, is_writable: true });
            } else {
                return Err(diagnosed_error!(
                    "no account fields provided, or provided accounts do not match IDL accounts"
                ));
            }
        }
        if !instruction_data.is_empty() {
            return Err(diagnosed_error!(
                "instruction data contains unrecognized fields: {}",
                instruction_data.iter().map(|(k, _)| k.as_ref()).collect::<Vec<&str>>().join(", ")
            ));
        }
        let instruction = Instruction { program_id, accounts, data: ix_data };
        instructions.push(instruction);
    }
    Ok(instructions)
}

struct InstructionBuilder {
    idl_instruction: IdlInstruction,
    encoded_instruction_args: IndexMap<String, Vec<u8>>,
    instruction_discriminator: Vec<u8>,
    program_id: Pubkey,
    accounts_map: IndexMap<String, AccountMeta>,
}

impl InstructionBuilder {
    fn new(
        idl: &IdlRef,
        program_id: &Pubkey,
        instruction_name: &str,
        instruction_args: Vec<Value>,
    ) -> Result<Self, Diagnostic> {
        let idl_instruction = idl.get_instruction(&instruction_name)?;

        let encoded_args = idl
            .get_encoded_args_map(&instruction_name, instruction_args.clone())
            .map_err(|e| diagnosed_error!("failed to encode instruction data: {e}"))?;

        let data = idl
            .get_discriminator(&instruction_name)
            .map_err(|e| diagnosed_error!("failed to encode instruction data: {e}"))?;

        Ok(Self {
            idl_instruction: idl_instruction.clone(),
            program_id: *program_id,
            encoded_instruction_args: encoded_args,
            instruction_discriminator: data,
            accounts_map: IndexMap::new(),
        })
    }

    fn get_instruction_data(&self) -> Vec<u8> {
        let mut data = self.instruction_discriminator.clone();
        data.extend(self.encoded_instruction_args.values().flat_map(|v| v));
        data
    }

    fn get_instruction_accounts(
        &mut self,
        instruction_data: &mut IndexMap<String, Value>,
    ) -> Result<Vec<AccountMeta>, Diagnostic> {
        let mut idl_instruction_accounts =
            VecDeque::from_iter(self.idl_instruction.accounts.iter());
        let mut attempts = 0;
        // in the worst case of account ordering, it will require (n * (n + 1)) / 2 attempts to find all accounts, where n is the number of accounts
        // this is because there could be a dependency chain of accounts, where each account depends on the previous one
        // for example, if we have 3 accounts: A, B, C
        // and A depends on B, and B depends on C, and they are in the IDL in that order, we would do the following flow:
        // try to compute, A, find that B isn't available
        // try to compute, B, find that C isn't available
        // try to compute, C, find that it's available
        // try again on A, find that B isn't available
        // try again on B, find that it's available
        // try again on A, find that it's available
        let max_attempts =
            (idl_instruction_accounts.len() * (idl_instruction_accounts.len() + 1)) / 2;
        while let Some(idl_account_item) = idl_instruction_accounts.pop_front() {
            attempts += 1;
            let account_name = match idl_account_item {
                IdlInstructionAccountItem::Composite(accounts) => accounts.name.clone(),
                IdlInstructionAccountItem::Single(account) => account.name.clone(),
            };
            let some_user_provided_account = instruction_data.swap_remove(&account_name);

            if let Some(user_provided_account_value) = some_user_provided_account.as_ref() {
                let account_meta = self.parse_user_provided_account_data(
                    &user_provided_account_value,
                    idl_account_item,
                )?;
                self.accounts_map.insert(account_name, account_meta);
            } else {
                match self.parse_idl_account_item(idl_account_item, None) {
                    Ok((Some(pubkey), is_signer, is_writable)) => {
                        self.accounts_map.insert(
                            account_name,
                            AccountMeta { pubkey, is_signer, is_writable },
                        );
                    }
                    Ok((None, _, _)) => {
                        return Err(diagnosed_error!("account '{account_name}' could not be derived by IDL; please provide it as an account in the runbook"))
                    }
                    Err(e) => {
                        match &e {
                            ParseIdlAccountErr::NoAccount(_, _) => {
                                if attempts >= max_attempts {
                                    return Err(e.to_diagnostic())
                                }
                                else {
                                    idl_instruction_accounts.push_back(idl_account_item);
                                }
                            }
                            _ => {
                                return Err(e.to_diagnostic())
                            }
                        }
                    }
                }
            };
        }

        let mut ordered_accounts = vec![];
        for idl_account in self.idl_instruction.accounts.iter() {
            let account_name = match idl_account {
                IdlInstructionAccountItem::Composite(accounts) => accounts.name.clone(),
                IdlInstructionAccountItem::Single(account) => account.name.clone(),
            };
            ordered_accounts.push(self.accounts_map.get(&account_name).unwrap().clone());
        }

        Ok(ordered_accounts)
    }

    fn parse_user_provided_account_data(
        &self,
        account_value: &Value,
        account_spec: &IdlInstructionAccountItem,
    ) -> Result<AccountMeta, Diagnostic> {
        let account =
            account_value.as_map().ok_or(diagnosed_error!("each account field must be a map"))?;
        if account.len() != 1 {
            return Err(diagnosed_error!("each account field must have exactly one entry"));
        }
        let account =
            account.first().unwrap().as_object().expect("expected map entry to be an object");

        let pubkey = account
            .get(PUBLIC_KEY)
            .map(|p| SvmValue::to_pubkey(p))
            .ok_or(diagnosed_error!("each account entry must have a 'public_key' field"))?
            .map_err(|e| diagnosed_error!("invalid 'public_key': {e}"))?;

        let (_, is_signer, is_writable) = self
            .parse_idl_account_item(account_spec, Some(pubkey))
            .map_err(|e| e.to_diagnostic())?;
        Ok(AccountMeta { pubkey, is_signer, is_writable })
    }

    fn parse_idl_account_pubkey(
        &self,
        account: &IdlInstructionAccount,
    ) -> Result<Option<Pubkey>, ParseIdlAccountErr> {
        if account.name == "program" {
            return Ok(Some(self.program_id.clone()));
        }

        if let Some(pda) = &account.pda {
            let mut seeds = vec![];
            for seed in pda.seeds.iter() {
                match seed {
                    IdlSeed::Const(seed) => {
                        seeds.push(seed.value.as_ref());
                    }
                    IdlSeed::Arg(arg) => {
                        let Some(seed) = self.encoded_instruction_args.get(&arg.path) else {
                            return Err(ParseIdlAccountErr::NoArg(
                                account.name.clone(),
                                arg.path.clone(),
                            ));
                        };
                        seeds.push(seed.as_ref());
                    }
                    IdlSeed::Account(seed_account) => {
                        let Some(account) = self.accounts_map.get(&seed_account.path) else {
                            return Err(ParseIdlAccountErr::NoAccount(
                                account.name.clone(),
                                seed_account.path.clone(),
                            ));
                        };
                        seeds.push(account.pubkey.as_ref());
                    }
                }
            }

            let program_id = if let Some(program_seed) = &pda.program {
                match program_seed {
                    IdlSeed::Const(idl_seed_const) => {
                        let idl_seed_array: [u8; 32] = idl_seed_const.value[..]
                            .try_into()
                            .expect("anchor idl contained invalid pubkey");
                        Pubkey::new_from_array(idl_seed_array)
                    }
                    IdlSeed::Arg(arg) => {
                        let Some(seed) = self.encoded_instruction_args.get(&arg.path) else {
                            return Err(ParseIdlAccountErr::NoArg(
                                account.name.clone(),
                                arg.path.clone(),
                            ));
                        };
                        let idl_seed_array: [u8; 32] = seed[..].try_into().map_err(|_| {
                            ParseIdlAccountErr::InvalidArg(
                                account.name.clone(),
                                arg.path.clone(),
                                format!("could not convert arg to 32-byte program id"),
                            )
                        })?;
                        Pubkey::new_from_array(idl_seed_array)
                    }
                    IdlSeed::Account(seed_account) => {
                        let Some(account) = self.accounts_map.get(&seed_account.path) else {
                            return Err(ParseIdlAccountErr::NoAccount(
                                account.name.clone(),
                                seed_account.path.clone(),
                            ));
                        };
                        account.pubkey.clone()
                    }
                }
            } else {
                self.program_id.clone()
            };
            return Ok(Pubkey::try_find_program_address(&seeds, &program_id).map(|pda| pda.0));
        }

        Ok(account
            .address
            .as_ref()
            .map(|p| Pubkey::from_str(&p).expect("anchor idl contained invalid pubkey")))
    }

    fn parse_idl_account_item(
        &self,
        account_spec: &IdlInstructionAccountItem,
        found_pubkey: Option<Pubkey>,
    ) -> Result<(Option<Pubkey>, bool, bool), ParseIdlAccountErr> {
        match account_spec {
            IdlInstructionAccountItem::Composite(accounts) => {
                // todo, is this right? for composite accounts, if one is writable are they all?
                let account = accounts.accounts.first().unwrap();
                self.parse_idl_account_item(account, found_pubkey)
            }
            IdlInstructionAccountItem::Single(account) => {
                if let Some(found_pubkey) = found_pubkey {
                    Ok((Some(found_pubkey), account.signer, account.writable))
                } else {
                    Ok((self.parse_idl_account_pubkey(&account)?, account.signer, account.writable))
                }
            }
        }
    }
}

enum ParseIdlAccountErr {
    NoArg(String, String),
    InvalidArg(String, String, String),
    NoAccount(String, String),
}

impl ParseIdlAccountErr {
    fn to_diagnostic(&self) -> Diagnostic {
        match self {
            ParseIdlAccountErr::InvalidArg(account_name, arg_name, e) => {
                diagnosed_error!(
                    "account '{account_name}' is a PDA derived from instruction arguments, but the argument '{arg_name}' was invalid: {e}"
                )
            }
            ParseIdlAccountErr::NoArg(account_name, arg_name) => {
                diagnosed_error!(
                    "account '{account_name}' is a PDA derived from instruction arguments, but no value was provided for argument '{arg_name}'"
                )
            }
            ParseIdlAccountErr::NoAccount(account_name, pda_seed_account_name) => {
                diagnosed_error!(
                    "account '{account_name}' is a PDA derived from another account '{pda_seed_account_name}', but no value was provided for that account"
                )
            }
        }
    }
}
