use std::collections::HashMap;

use solana_instruction::Instruction;
use solana_message::Message;
use txtx_addon_kit::{
    hex,
    types::{
        signers::SignerInstance,
        types::{ObjectType, Value},
        ConstructDid,
    },
};

pub fn message_to_formatted_tx(message: &Message) -> Value {
    let mut instructions = Vec::new();
    let message_account_keys = message.account_keys.clone();
    for instruction in message.instructions.iter() {
        let Some(account) = message_account_keys.get(instruction.program_id_index as usize) else {
            continue;
        };
        let accounts = instruction
            .accounts
            .iter()
            .filter_map(|a| {
                let Some(pubkey) = message_account_keys.get(*a as usize) else {
                    return None;
                };
                Some(
                    ObjectType::from([
                        ("pubkey", Value::string(pubkey.to_string())),
                        ("is_signer", Value::bool(message.is_signer(*a as usize))),
                        ("is_writable", Value::bool(message.is_maybe_writable(*a as usize, None))),
                    ])
                    .to_value(),
                )
            })
            .collect::<Vec<Value>>();
        let account_name = account.to_string();

        instructions.push(
            ObjectType::from(vec![
                ("program_id", Value::string(account_name)),
                ("data", Value::string(format!("0x{}", hex::encode(&instruction.data)))),
                ("accounts", Value::array(accounts)),
            ])
            .to_value(),
        );
    }
    message_data_to_formatted_value(
        &instructions,
        message.header.num_required_signatures,
        message.header.num_readonly_signed_accounts,
        message.header.num_readonly_unsigned_accounts,
    )
}

pub fn ix_to_formatted_value(ix: &Instruction) -> Value {
    ObjectType::from([
        ("program_id", Value::string(ix.program_id.to_string())),
        ("data", Value::string(format!("0x{}", hex::encode(&ix.data)))),
        (
            "accounts",
            Value::array(
                ix.accounts
                    .iter()
                    .map(|a| {
                        ObjectType::from([
                            ("pubkey", Value::string(a.pubkey.to_string())),
                            ("is_signer", Value::bool(a.is_signer)),
                            ("is_writable", Value::bool(a.is_writable)),
                        ])
                        .to_value()
                    })
                    .collect::<Vec<_>>(),
            ),
        ),
    ])
    .to_value()
}

pub fn message_data_to_formatted_value(
    ix_values: &[Value],
    num_required_signatures: u8,
    num_readonly_signed_accounts: u8,
    num_readonly_unsigned_accounts: u8,
) -> Value {
    ObjectType::from(vec![
        ("instructions", Value::array(ix_values.to_vec())),
        ("num_required_signatures", Value::integer(num_required_signatures as i128)),
        ("num_readonly_signed_accounts", Value::integer(num_readonly_signed_accounts as i128)),
        ("num_readonly_unsigned_accounts", Value::integer(num_readonly_unsigned_accounts as i128)),
    ])
    .to_value()
}

pub fn get_formatted_signer_names(
    signer_dids: &Vec<ConstructDid>,
    signers_instances: &HashMap<ConstructDid, SignerInstance>,
) -> String {
    let mut signer_names = String::new();
    let signer_count = signer_dids.len();
    for (i, did) in signer_dids.iter().enumerate() {
        let signer_instance = signers_instances.get(did).expect("Signer instance not found");
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
    signer_names
}

pub fn get_formatted_transaction_meta_description(
    descriptions: &Vec<String>,
    signer_dids: &Vec<ConstructDid>,
    signers_instances: &HashMap<ConstructDid, SignerInstance>,
) -> String {
    let signer_names = get_formatted_signer_names(signer_dids, signers_instances);
    let signer_count = signer_dids.len();
    format!(
        "A transaction with {} instruction{} will be signed by the {} signer{}. {}",
        descriptions.len(),
        if descriptions.len() == 1 { "" } else { "s" },
        signer_names,
        if signer_count == 1 { "" } else { "s" },
        descriptions.join(" ")
    )
}
