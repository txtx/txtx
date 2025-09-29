use std::{str::FromStr, thread::sleep, time::Duration};

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    bpf_loader_upgradeable::{self, get_program_data_address, UpgradeableLoaderState},
    clock::DEFAULT_MS_PER_SLOT,
    pubkey::Pubkey,
};
use txtx_addon_kit::types::{diagnostics::Diagnostic, types::Value};

use crate::commands::setup_surfnet::set_account::SurfpoolAccountUpdate;

pub fn get_seeds_from_value(value: &Value) -> Result<Vec<Vec<u8>>, Diagnostic> {
    let seeds = value
        .as_array()
        .ok_or_else(|| diagnosed_error!("seeds must be an array"))?
        .iter()
        .map(|s| {
            let bytes = s.to_le_bytes();
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

pub async fn cheatcode_deploy_program(
    rpc_client: &RpcClient,
    program_id: Pubkey,
    data: &Vec<u8>,
    upgrade_authority: Option<Pubkey>,
) -> Result<(), Diagnostic> {
    let program_data_address = get_program_data_address(&program_id);
    let rent_lamports = rpc_client
        .get_minimum_balance_for_rent_exemption(data.len())
        .await
        .map_err(|e| diagnosed_error!("failed to get rent exemption: {e}"))?;

    let slot = rpc_client
        .get_slot()
        .await
        .map_err(|e| diagnosed_error!("failed to get current slot: {e}"))?;

    let mut program_data = bincode::serialize(&UpgradeableLoaderState::ProgramData {
        slot,
        // LiteSVM rejects setting a program account without an upgrade authority for some reason,
        // so we set one to the default Pubkey if none is provided.
        upgrade_authority_address: Some(upgrade_authority.unwrap_or_default()),
    })
    .map_err(|e| diagnosed_error!("failed to serialize program data state: {e}"))?;
    program_data.extend(data);

    let program_data_address_payload = SurfpoolAccountUpdate {
        public_key: program_data_address,
        lamports: Some(rent_lamports),
        data: Some(txtx_addon_kit::hex::encode(program_data)),
        owner: Some(bpf_loader_upgradeable::id().to_string()),
        executable: Some(false),
        rent_epoch: Some(0),
    };

    let program_data = bincode::serialize(&UpgradeableLoaderState::Program {
        programdata_address: program_data_address,
    })
    .map_err(|e| diagnosed_error!("failed to serialize program state: {e}"))?;

    let rent_lamports = rpc_client
        .get_minimum_balance_for_rent_exemption(program_data.len())
        .await
        .map_err(|e| diagnosed_error!("failed to get rent exemption: {e}"))?;
    let program_payload = SurfpoolAccountUpdate {
        public_key: program_id,
        lamports: Some(rent_lamports),
        data: Some(txtx_addon_kit::hex::encode(&program_data)),
        owner: Some(bpf_loader_upgradeable::id().to_string()),
        executable: Some(true),
        rent_epoch: Some(0),
    };

    program_data_address_payload.send_request(&rpc_client).await?;

    program_payload.send_request(&rpc_client).await?;

    Ok(())
}

pub fn wait_n_slots(rpc_client: &solana_client::rpc_client::RpcClient, n: u64) -> u64 {
    let slot = rpc_client.get_slot().unwrap();
    loop {
        sleep(Duration::from_millis(DEFAULT_MS_PER_SLOT));
        let new_slot = rpc_client.get_slot().unwrap();
        if new_slot.saturating_sub(slot) >= n {
            return new_slot;
        }
    }
}
