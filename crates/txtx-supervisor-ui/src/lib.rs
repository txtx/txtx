pub mod cloud_relayer;
pub mod http;

use std::{collections::BTreeMap, sync::Arc};

use actix_web::dev::ServerHandle;
use cloud_relayer::{start_relayer_event_runloop, RelayerChannelEvent, RelayerContext};
use include_dir::{include_dir, Dir};
use tokio::sync::{broadcast::Sender as TokioBroadcastSender, RwLock};
use txtx_addon_kit::{
    channel::{Receiver, Sender},
    types::frontend::{
        ActionItemResponse, Block as ActionBlock, BlockEvent, LogEvent, SupervisorAddonData,
    },
};
use txtx_gql::Context as GqlContext;

#[cfg(feature = "crates_build")]
pub const CARGO_MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
#[cfg(feature = "crates_build")]
pub static ASSETS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/supervisor-dist");
#[cfg(feature = "bin_build")]
pub const OUT_DIR: &str = env!("OUT_DIR");
#[cfg(feature = "bin_build")]
pub static ASSETS: Dir<'_> = include_dir!("$OUT_DIR/supervisor");

pub const DEFAULT_BINDING_PORT: &str = "8488";
pub const DEFAULT_BINDING_ADDRESS: &str = "localhost";

#[derive(Debug, Clone)]
pub enum SupervisorEvents {
    /// The supervisor has started, with the network binding it's running on.
    Started(String),
}

pub async fn start_supervisor_ui(
    runbook_name: String,
    runbook_description: Option<String>,
    supervisor_addon_data: Vec<SupervisorAddonData>,
    block_store: Arc<RwLock<BTreeMap<usize, ActionBlock>>>,
    log_store: Arc<RwLock<Vec<LogEvent>>>,
    block_broadcaster: TokioBroadcastSender<BlockEvent>,
    log_broadcaster: TokioBroadcastSender<LogEvent>,
    action_item_events_tx: TokioBroadcastSender<ActionItemResponse>,
    relayer_channel_tx: Sender<RelayerChannelEvent>,
    relayer_channel_rx: Receiver<RelayerChannelEvent>,
    kill_loops_tx: Sender<bool>,
    network_binding_ip_address: &str,
    network_binding_port: u16,
    supervisor_events_tx: Sender<SupervisorEvents>,
) -> Result<ServerHandle, String> {
    let gql_context = GqlContext {
        protocol_name: runbook_name.clone(),
        runbook_name,
        supervisor_addon_data,
        runbook_description,
        block_store,
        log_store,
        block_broadcaster: block_broadcaster.clone(),
        log_broadcaster: log_broadcaster.clone(),
        action_item_events_tx: action_item_events_tx.clone(),
    };

    let channel_data = Arc::new(RwLock::new(None));
    let relayer_context = RelayerContext {
        relayer_channel_tx: relayer_channel_tx.clone(),
        channel_data: channel_data.clone(),
    };

    let network_binding = format!("{}:{}", network_binding_ip_address, network_binding_port);
    let _ = supervisor_events_tx.send(SupervisorEvents::Started(network_binding.clone()));

    let handle = http::start_server(gql_context, relayer_context, &network_binding)
        .await
        .map_err(|e| format!("Failed to start web ui: {e}"))?;

    let _ = hiro_system_kit::thread_named("Relayer Interaction").spawn(move || {
        let future = start_relayer_event_runloop(
            channel_data,
            relayer_channel_rx,
            relayer_channel_tx,
            action_item_events_tx,
            kill_loops_tx,
        );
        hiro_system_kit::nestable_block_on(future)
    });

    Ok(handle)
}
