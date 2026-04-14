use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_request::RpcRequest;
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::types::frontend::LogDispatcher;

pub trait SurfnetAccountUpdate {
    fn rpc_method() -> &'static str
    where
        Self: Sized;

    fn to_request_params(&self) -> serde_json::Value;

    fn update_status(&self, logger: &LogDispatcher, index: usize, total: usize);

    async fn send_request(
        &self,
        rpc_client: &RpcClient,
    ) -> Result<serde_json::Value, Diagnostic>
    where
        Self: Sized,
    {
        rpc_client
            .send::<serde_json::Value>(
                RpcRequest::Custom { method: Self::rpc_method() },
                self.to_request_params(),
            )
            .await
            .map_err(|e| diagnosed_error!("`{}` RPC call failed: {e}", Self::rpc_method()))
    }

    async fn process_updates(
        updates: Vec<Self>,
        rpc_client: &RpcClient,
        logger: &LogDispatcher,
    ) -> Result<(), Diagnostic>
    where
        Self: Sized,
    {
        let total = updates.len();
        for (i, update) in updates.iter().enumerate() {
            let _ = update.send_request(rpc_client).await?;
            update.update_status(logger, i, total);
        }
        Ok(())
    }
}
