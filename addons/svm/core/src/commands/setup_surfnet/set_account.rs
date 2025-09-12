use serde::{Deserialize, Serialize};
use serde_json::json;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_request::RpcRequest;
use solana_sdk::pubkey::Pubkey;
use txtx_addon_kit::{
    hex,
    indexmap::IndexMap,
    types::{diagnostics::Diagnostic, frontend::LogDispatcher, stores::ValueStore, types::Value},
};
use txtx_addon_network_svm_types::SvmValue;

use crate::constants::SET_ACCOUNT;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfpoolAccountUpdate {
    // Skipping serialization of public_key to avoid sending it in the request
    // as it is already included in the request parameters.
    #[serde(skip)]
    pub public_key: Pubkey,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lamports: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rent_epoch: Option<u64>,
}

impl SurfpoolAccountUpdate {
    pub fn new(
        public_key: Pubkey,
        lamports: Option<u64>,
        data: Option<String>,
        owner: Option<String>,
        executable: Option<bool>,
        rent_epoch: Option<u64>,
    ) -> Self {
        Self { public_key, lamports, data, owner, executable, rent_epoch }
    }

    pub fn from_map(map: &mut IndexMap<String, Value>) -> Result<Self, Diagnostic> {
        let some_public_key = map.swap_remove("public_key");
        let public_key = some_public_key
            .map(|p| {
                SvmValue::to_pubkey(&p)
                    .map_err(|e| diagnosed_error!("invalid 'public_key' field: {e}"))
            })
            .ok_or_else(|| diagnosed_error!("missing required 'public_key'"))??;

        let some_lamports = map.swap_remove("lamports");
        let lamports = some_lamports
            .map(|v| {
                v.as_uint()
                    .map(|r| r.map_err(|e| diagnosed_error!("{e}")))
                    .ok_or_else(|| diagnosed_error!("expected 'lamports' field to be a u64"))
            })
            .transpose()?
            .transpose()?;

        let some_data = map.swap_remove("data");
        let data = some_data.map(|v| hex::encode(v.to_be_bytes()));

        let some_owner = map.swap_remove("owner");
        let owner = some_owner
            .map(|p| {
                SvmValue::to_pubkey(&p)
                    .map_err(|e| diagnosed_error!("invalid 'owner' field: {e}"))
                    .map(|p| p.to_string())
            })
            .transpose()?;

        let some_executable = map.swap_remove("executable");
        let executable = some_executable
            .map(|v| {
                v.as_bool()
                    .ok_or_else(|| diagnosed_error!("expected 'executable' field to be a boolean"))
            })
            .transpose()?;

        let some_rent_epoch = map.swap_remove("rent_epoch");
        let rent_epoch = some_rent_epoch
            .map(|v| {
                v.as_uint()
                    .map(|r| r.map_err(|e| diagnosed_error!("{e}")))
                    .ok_or_else(|| diagnosed_error!("expected 'rent_epoch' field to be a u64"))
            })
            .transpose()?
            .transpose()?;

        if lamports.is_none()
            && data.is_none()
            && owner.is_none()
            && executable.is_none()
            && rent_epoch.is_none()
        {
            return Err(diagnosed_error!("at least one of 'lamports', 'data', 'owner', 'executable', or 'rent_epoch' must be provided"));
        }
        Ok(Self::new(public_key, lamports, data, owner, executable, rent_epoch))
    }

    pub fn parse_value_store(values: &ValueStore) -> Result<Vec<Self>, Diagnostic> {
        let mut account_updates = vec![];

        let account_update_data = values
            .get_value(SET_ACCOUNT)
            .map(|v| v.as_map().ok_or_else(|| diagnosed_error!("'set_account' must be a map type")))
            .transpose()?;

        let Some(account_update_data) = account_update_data else {
            return Ok(vec![]);
        };

        let mut account_update_data = account_update_data
            .iter()
            .map(|i| {
                i.as_object()
                    .map(|o| o.clone())
                    .ok_or(diagnosed_error!("'set_account' must be a map type"))
            })
            .collect::<Result<Vec<_>, _>>()?;

        for (i, account_update) in account_update_data.iter_mut().enumerate() {
            let prefix = format!("failed to parse `set_account` map #{}", i + 1);
            let account = SurfpoolAccountUpdate::from_map(account_update)
                .map_err(|e| diagnosed_error!("{prefix}: {e}"))?;

            account_updates.push(account);
        }

        Ok(account_updates)
    }

    fn to_request_params(&self) -> serde_json::Value {
        let pubkey = json![self.public_key.to_string()];
        let account_update = serde_json::to_value(&self).unwrap();
        json!(vec![pubkey, account_update])
    }

    fn rpc_method() -> &'static str {
        "surfnet_setAccount"
    }

    fn update_status(&self, logger: &LogDispatcher, index: usize, total: usize) {
        logger.success_info(
            "Account Updated",
            &format!(
                "Processed surfpool account update #{}/{} for {}",
                index + 1,
                total,
                self.public_key.to_string()
            ),
        );
    }

    async fn send_request(&self, rpc_client: &RpcClient) -> Result<serde_json::Value, Diagnostic> {
        rpc_client
            .send::<serde_json::Value>(
                RpcRequest::Custom { method: Self::rpc_method() },
                self.to_request_params(),
            )
            .await
            .map_err(|e| diagnosed_error!("`{}` RPC call failed: {e}", Self::rpc_method()))
    }

    pub async fn process_updates(
        account_updates: Vec<Self>,
        rpc_client: &RpcClient,
        logger: &LogDispatcher,
    ) -> Result<(), Diagnostic> {
        for (i, account_update) in account_updates.iter().enumerate() {
            let _ = account_update.send_request(rpc_client).await?;
            account_update.update_status(logger, i, account_updates.len());
        }
        Ok(())
    }
}
