use std::{str::FromStr, thread::sleep, time::Duration};

use solana_client::rpc_request::RpcRequest;
use solana_clock::DEFAULT_MS_PER_SLOT;
use solana_loader_v3_interface::get_program_data_address;
use solana_loader_v3_interface::state::UpgradeableLoaderState;
use solana_pubkey::Pubkey;
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

fn set_account_cheatcode(
    rpc_client: &solana_client::rpc_client::RpcClient,
    account_update: &SurfpoolAccountUpdate,
) -> Result<(), Diagnostic> {
    let pubkey = serde_json::json!(account_update.public_key.to_string());
    let account_update_value = serde_json::to_value(account_update).unwrap();
    let params = serde_json::json!(vec![pubkey, account_update_value]);

    let _ = rpc_client
        .send::<serde_json::Value>(RpcRequest::Custom { method: "surfnet_setAccount" }, params)
        .map_err(|e| diagnosed_error!("`surfnet_setAccount` RPC call failed: {e}"))?;

    Ok(())
}

pub fn cheatcode_deploy_program(
    rpc_api_url: &str,
    program_id: Pubkey,
    data: &Vec<u8>,
    upgrade_authority: Pubkey,
) -> Result<(), Diagnostic> {
    let rpc_client = solana_client::rpc_client::RpcClient::new(rpc_api_url.to_string());
    let program_data_address = get_program_data_address(&program_id);
    let rent_lamports = rpc_client
        .get_minimum_balance_for_rent_exemption(data.len())
        .map_err(|e| diagnosed_error!("failed to get rent exemption: {e}"))?;

    let slot =
        rpc_client.get_slot().map_err(|e| diagnosed_error!("failed to get current slot: {e}"))?;

    let mut program_data = bincode::serialize(&UpgradeableLoaderState::ProgramData {
        slot,
        upgrade_authority_address: Some(upgrade_authority),
    })
    .map_err(|e| diagnosed_error!("failed to serialize program data state: {e}"))?;
    program_data.extend(data);

    let program_data_address_payload = SurfpoolAccountUpdate {
        public_key: program_data_address,
        lamports: Some(rent_lamports),
        data: Some(txtx_addon_kit::hex::encode(program_data)),
        owner: Some(solana_sdk_ids::bpf_loader_upgradeable::id().to_string()),
        executable: Some(false),
        rent_epoch: Some(0),
    };

    let program_data = bincode::serialize(&UpgradeableLoaderState::Program {
        programdata_address: program_data_address,
    })
    .map_err(|e| diagnosed_error!("failed to serialize program state: {e}"))?;

    let rent_lamports = rpc_client
        .get_minimum_balance_for_rent_exemption(program_data.len())
        .map_err(|e| diagnosed_error!("failed to get rent exemption: {e}"))?;
    let program_payload = SurfpoolAccountUpdate {
        public_key: program_id,
        lamports: Some(rent_lamports),
        data: Some(txtx_addon_kit::hex::encode(&program_data)),
        owner: Some(solana_sdk_ids::bpf_loader_upgradeable::id().to_string()),
        executable: Some(true),
        rent_epoch: Some(0),
    };

    set_account_cheatcode(&rpc_client, &program_data_address_payload)?;

    set_account_cheatcode(&rpc_client, &program_payload)?;

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
