use solana_sdk::instruction::{AccountMeta, Instruction};
use txtx_addon_kit::types::{diagnostics::Diagnostic, stores::ValueStore};

use crate::{
    constants::{ACCOUNT, DATA, INSTRUCTION, IS_SIGNER, IS_WRITABLE, PROGRAM_ID, PUBLIC_KEY},
    typing::SvmValue,
};

pub fn parse_instructions_map(values: &ValueStore) -> Result<Vec<Instruction>, Diagnostic> {
    let mut instructions = vec![];
    let instructions_data = values
        .get_expected_map(INSTRUCTION)
        .map_err(|diag| diag)?
        .iter()
        .map(|i| i.as_object().ok_or(diagnosed_error!("'instruction' must be a map type")))
        .collect::<Result<Vec<_>, _>>()?;

    for instruction_data in instructions_data.iter() {
        // if the value key was provided, treat it as a serialized instruction
        if let Some(value) = instruction_data.get("value") {
            let instruction = serde_json::from_slice(&value.to_bytes())
                .map_err(|e| diagnosed_error!("failed to deserialize instruction: {e}"))?;
            instructions.push(instruction);
            continue;
        }
        let program_id = instruction_data
            .get(PROGRAM_ID)
            .map(|p| SvmValue::to_pubkey(p))
            .ok_or(diagnosed_error!("'program_id' is required for each instruction"))?
            .map_err(|e| diagnosed_error!("invalid 'program_id' for instruction: {e}"))?;

        let accounts = instruction_data
            .get(ACCOUNT)
            .ok_or(diagnosed_error!("'account' is required for each instruction"))?
            .as_map()
            .ok_or(diagnosed_error!("'account' field for an instruction must be a map"))?
            .iter()
            .map::<Result<AccountMeta, Diagnostic>, _>(|a| {
                let account = a.as_object().ok_or(diagnosed_error!(
                    "each map entry of 'account' field must be an object"
                ))?;

                let pubkey = account
                    .get(PUBLIC_KEY)
                    .map(|p| SvmValue::to_pubkey(p))
                    .ok_or(diagnosed_error!(
                        "each map entry of 'account' field must have a 'public_key' field"
                    ))?
                    .map_err(|e| {
                        diagnosed_error!("invalid 'public_key' for 'account' field: {e}")
                    })?;

                let is_signer = account.get(IS_SIGNER).and_then(|b| b.as_bool()).unwrap_or(false);
                let is_writable =
                    account.get(IS_WRITABLE).and_then(|b| b.as_bool()).unwrap_or(false);
                Ok(AccountMeta { pubkey, is_signer, is_writable })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let data = instruction_data
            .get(DATA)
            .map(|d| {
                d.get_buffer_bytes_result()
                    .map_err(|e| diagnosed_error!("invalid 'data' for instruction: {e}"))
            })
            .transpose()?
            .unwrap_or_default();

        let instruction = Instruction { program_id, accounts, data };
        instructions.push(instruction);
    }
    Ok(instructions)
}
