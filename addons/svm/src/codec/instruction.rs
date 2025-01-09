use solana_sdk::instruction::{AccountMeta, Instruction};
use txtx_addon_kit::types::stores::ValueStore;

use crate::{
    constants::{ACCOUNT, DATA, INSTRUCTION, IS_SIGNER, IS_WRITABLE, PROGRAM_ID, PUBLIC_KEY},
    typing::SvmValue,
};

pub fn parse_instructions_map(values: &ValueStore) -> Result<Vec<Instruction>, String> {
    let mut instructions = vec![];
    let instructions_data = values
        .get_expected_map(INSTRUCTION)
        .map_err(|diag| diag.message)?
        .iter()
        .map(|i| i.as_object().ok_or("'instruction' must be a map type".to_string()))
        .collect::<Result<Vec<_>, _>>()?;

    for instruction_data in instructions_data.iter() {
        let program_id = instruction_data
            .get(PROGRAM_ID)
            .map(|p| SvmValue::to_pubkey(p))
            .ok_or("'program_id' is required for each instruction".to_string())?
            .map_err(|e| format!("invalid 'program_id' for instruction: {e}"))?;

        let accounts = instruction_data
            .get(ACCOUNT)
            .ok_or("'account' is required for each instruction".to_string())?
            .as_map()
            .ok_or("'account' field for an instruction must be a map".to_string())?
            .iter()
            .map::<Result<AccountMeta, String>, _>(|a| {
                let account = a
                    .as_object()
                    .ok_or("each map entry of 'account' field must be an object".to_string())?;

                let pubkey = account
                    .get(PUBLIC_KEY)
                    .map(|p| SvmValue::to_pubkey(p))
                    .ok_or(
                        "each map entry of 'account' field must have a 'public_key' field"
                            .to_string(),
                    )?
                    .map_err(|e| format!("invalid 'public_key' for 'account' field: {e}"))?;

                let is_signer = account.get(IS_SIGNER).and_then(|b| b.as_bool()).unwrap_or(false);
                let is_writable =
                    account.get(IS_WRITABLE).and_then(|b| b.as_bool()).unwrap_or(false);
                Ok(AccountMeta { pubkey, is_signer, is_writable })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let data = instruction_data
            .get(DATA)
            .ok_or("'data' is required for each instruction".to_string())?
            .expect_buffer_bytes_result()
            .map_err(|e| format!("invalid 'data' for instruction: {e}"))?;

        let instruction = Instruction { program_id, accounts, data };
        instructions.push(instruction);
    }
    Ok(instructions)
}
