use serde::{Deserialize, Serialize};
use serde_json::json;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_request::RpcRequest;
use solana_sdk::pubkey::Pubkey;

use txtx_addon_kit::{
    indexmap::IndexMap,
    types::{diagnostics::Diagnostic, frontend::LogDispatcher, stores::ValueStore, types::Value},
};
use txtx_addon_network_svm_types::SvmValue;

use crate::constants::CLONE_PROGRAM_ACCOUNT;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfpoolProgramCloning {
    #[serde(skip)]
    pub source_program_id: Pubkey,
    #[serde(skip)]
    pub destination_program_id: Pubkey,
}

impl SurfpoolProgramCloning {
    pub fn new(source_program_id: Pubkey, destination_program_id: Pubkey) -> Self {
        Self { source_program_id, destination_program_id }
    }

    pub fn from_map(map: &mut IndexMap<String, Value>) -> Result<Self, Diagnostic> {
        let some_source_program_id = map.swap_remove("source_program_id");
        let source_program_id = some_source_program_id
            .map(|p| {
                SvmValue::to_pubkey(&p)
                    .map_err(|e| diagnosed_error!("invalid 'source_program_id' field: {e}"))
            })
            .ok_or_else(|| diagnosed_error!("missing required 'source_program_id'"))??;

        let some_destination_program_id = map.swap_remove("destination_program_id");
        let destination_program_id = some_destination_program_id
            .map(|p| {
                SvmValue::to_pubkey(&p)
                    .map_err(|e| diagnosed_error!("invalid 'destination_program_id' field: {e}"))
            })
            .ok_or_else(|| diagnosed_error!("missing required 'destination_program_id'"))??;

        Ok(Self::new(source_program_id, destination_program_id))
    }

    pub fn parse_value_store(values: &ValueStore) -> Result<Vec<Self>, Diagnostic> {
        let mut program_clones = vec![];

        let clone_program_data = values
            .get_value(CLONE_PROGRAM_ACCOUNT)
            .map(|v| {
                v.as_map()
                    .ok_or_else(|| diagnosed_error!("'clone_program_account' must be a map type"))
            })
            .transpose()?;

        let Some(clone_program_data) = clone_program_data else {
            return Ok(vec![]);
        };

        let mut clone_program_data = clone_program_data
            .iter()
            .map(|i| {
                i.as_object()
                    .map(|o| o.clone())
                    .ok_or(diagnosed_error!("'clone_program_account' must be a map type"))
            })
            .collect::<Result<Vec<_>, _>>()?;

        for (i, clone_program) in clone_program_data.iter_mut().enumerate() {
            let prefix = format!("failed to parse `clone_program_account` map #{}", i + 1);
            let account = SurfpoolProgramCloning::from_map(clone_program)
                .map_err(|e| diagnosed_error!("{prefix}: {e}"))?;

            program_clones.push(account);
        }

        Ok(program_clones)
    }

    fn to_request_params(&self) -> serde_json::Value {
        let source_program_id = json![self.source_program_id.to_string()];
        let destination_program_id = json![self.destination_program_id.to_string()];
        json!(vec![source_program_id, destination_program_id])
    }

    fn rpc_method() -> &'static str {
        "surfnet_cloneProgramAccount"
    }

    fn update_status(&self, logger: &LogDispatcher, index: usize, total: usize) {
        logger.success_info(
            "Program Account Cloned",
            &format!("Cloned program account #{}/{}", index + 1, total,),
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
