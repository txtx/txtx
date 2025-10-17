use serde::{Deserialize, Serialize};
use serde_json::json;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_request::RpcRequest;
use solana_pubkey::Pubkey;
use txtx_addon_kit::{
    indexmap::IndexMap,
    types::{diagnostics::Diagnostic, frontend::LogDispatcher, stores::ValueStore, types::Value},
};
use txtx_addon_network_svm_types::SvmValue;

use crate::constants::RESET_ACCOUNT;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfpoolResetAccount {
    // Skipping serialization of public_key to avoid sending it in the request
    // as it is already included in the request parameters.
    #[serde(skip)]
    pub public_key: Pubkey,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_owned_accounts: Option<bool>,
}

impl SurfpoolResetAccount {
    pub fn new(public_key: Pubkey, include_owned_accounts: Option<bool>) -> Self {
        Self { public_key, include_owned_accounts }
    }

    pub fn from_map(map: &mut IndexMap<String, Value>) -> Result<Self, Diagnostic> {
        let some_public_key = map.swap_remove("public_key");
        let public_key = some_public_key
            .map(|p| {
                SvmValue::to_pubkey(&p)
                    .map_err(|e| diagnosed_error!("invalid 'public_key' field: {e}"))
            })
            .ok_or_else(|| diagnosed_error!("missing required 'public_key'"))??;

        let some_include_owned_accounts = map.swap_remove("include_owned_accounts");
        let include_owned_accounts = some_include_owned_accounts
            .map(|v| {
                v.as_bool().ok_or_else(|| {
                    diagnosed_error!("expected 'include_owned_accounts' field to be a boolean")
                })
            })
            .transpose()?;

        Ok(Self::new(public_key, include_owned_accounts))
    }

    pub fn parse_value_store(values: &ValueStore) -> Result<Vec<Self>, Diagnostic> {
        let mut account_resets = vec![];

        let account_reset_data = values
            .get_value(RESET_ACCOUNT)
            .map(|v| {
                v.as_map().ok_or_else(|| diagnosed_error!("'reset_account' must be a map type"))
            })
            .transpose()?;

        let Some(account_reset_data) = account_reset_data else {
            return Ok(vec![]);
        };

        let mut account_reset_data = account_reset_data
            .iter()
            .map(|i| {
                i.as_object()
                    .map(|o| o.clone())
                    .ok_or(diagnosed_error!("'reset_account' must be a map type"))
            })
            .collect::<Result<Vec<_>, _>>()?;

        for (i, account_reset) in account_reset_data.iter_mut().enumerate() {
            let prefix = format!("failed to parse `reset_account` map #{}", i + 1);
            let account = SurfpoolResetAccount::from_map(account_reset)
                .map_err(|e| diagnosed_error!("{prefix}: {e}"))?;

            account_resets.push(account);
        }

        Ok(account_resets)
    }

    fn to_request_params(&self) -> serde_json::Value {
        let pubkey = json![self.public_key.to_string()];
        let mut params = vec![pubkey];

        if self.include_owned_accounts.is_some() {
            let config = serde_json::to_value(&self).unwrap();
            params.push(config);
        }
        json!(params)
    }

    fn rpc_method() -> &'static str {
        "surfnet_resetAccount"
    }

    fn update_status(&self, logger: &LogDispatcher, index: usize, total: usize) {
        logger.success_info(
            "Account Reset",
            &format!(
                "Processed surfpool account reset #{}/{} for {}",
                index + 1,
                total,
                self.public_key.to_string()
            ),
        );
    }

    pub async fn send_request(
        &self,
        rpc_client: &RpcClient,
    ) -> Result<serde_json::Value, Diagnostic> {
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
