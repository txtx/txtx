use std::str::FromStr;

use solana_sdk::pubkey::Pubkey;
use txtx_addon_kit::types::{diagnostics::Diagnostic, types::Value};

pub fn get_seeds_from_value(value: &Value) -> Result<Vec<Vec<u8>>, Diagnostic> {
    let seeds = value
        .as_array()
        .ok_or_else(|| diagnosed_error!("seeds must be an array"))?
        .iter()
        .map(|s| {
            let bytes = s.to_bytes();
            if bytes.is_empty() {
                return Err(diagnosed_error!("seed cannot be empty"));
            }
            if bytes.len() > 32 {
                if let Ok(pubkey) = Pubkey::from_str(&s.to_string()) {
                    return Ok(pubkey.to_bytes().to_vec());
                } else {
                    return Err(diagnosed_error!("seed cannot be longer than 32 bytes",));
                }
            }
            Ok(bytes)
        })
        .collect::<Result<Vec<_>, _>>()?;

    if seeds.len() > 16 {
        return Err(diagnosed_error!("seeds a maximum of 16 seeds can be used"));
    }

    Ok(seeds)
}
