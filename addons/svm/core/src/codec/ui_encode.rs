use std::collections::HashMap;

use solana_sdk::instruction::Instruction;
use txtx_addon_kit::{
    hex,
    types::{
        signers::SignerInstance,
        types::{ObjectType, Value},
        ConstructDid,
    },
};

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

pub fn get_formatted_transaction_description(
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
