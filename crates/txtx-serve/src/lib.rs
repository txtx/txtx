use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use actix_cors::Cors;
use actix_web::dev::ServerHandle;
use actix_web::http::header::{self};
use actix_web::http::StatusCode;
use actix_web::web::{self, Data, Json};
use actix_web::{middleware, App, HttpRequest, HttpResponse, HttpServer};
use actix_web::{Error, HttpResponseBuilder, Responder};
use hiro_system_kit::green;
use juniper_actix::{graphql_handler, subscriptions};
use juniper_graphql_ws::ConnectionConfig;
use serde::ser::StdError;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tokio::sync::RwLock;
use txtx_addon_kit::types::cloud_interface::CloudServiceContext;
use txtx_addon_kit::types::frontend::SupervisorAddonData;
use txtx_addon_kit::Addon;
use txtx_addon_network_bitcoin::BitcoinNetworkAddon;
use txtx_addon_network_evm::EvmNetworkAddon;
#[cfg(feature = "ovm")]
use txtx_addon_network_ovm::OvmNetworkAddon;
#[cfg(feature = "stacks")]
use txtx_addon_network_stacks::StacksNetworkAddon;
use txtx_addon_network_svm::SvmNetworkAddon;
#[cfg(feature = "sp1")]
use txtx_addon_sp1::Sp1Addon;
use txtx_addon_telegram::TelegramAddon;
use txtx_core::kit::channel;
use txtx_core::kit::helpers::fs::FileLocation;
use txtx_core::kit::types::frontend::{
    ActionItemRequest, BlockEvent, ClientType, DiscoveryResponse,
};
use txtx_core::kit::types::{AuthorizationContext, RunbookId};
use txtx_core::kit::uuid::Uuid;
use txtx_core::runbook::RunbookTopLevelInputsMap;
use txtx_core::start_supervised_runbook_runloop;
use txtx_core::std::StdAddon;
use txtx_core::types::{Runbook, RunbookSources};
use txtx_gql::Context as GqlContext;
use txtx_gql::{new_graphql_schema, Context as GraphContext, GraphqlSchema};
use txtx_supervisor_ui::cloud_relayer::{
    start_relayer_event_runloop, RelayerChannelEvent, RelayerContext,
};

pub const SERVE_BINDING_PORT: &str = "18488";
pub const SERVE_BINDING_ADDRESS: &str = "localhost";

fn get_available_addons() -> Vec<Box<dyn Addon>> {
    vec![
        Box::new(StdAddon::new()),
        Box::new(SvmNetworkAddon::new()),
        #[cfg(feature = "stacks")]
        Box::new(StacksNetworkAddon::new()),
        Box::new(EvmNetworkAddon::new()),
        Box::new(BitcoinNetworkAddon::new()),
        Box::new(TelegramAddon::new()),
        #[cfg(feature = "sp1")]
        Box::new(Sp1Addon::new()),
        #[cfg(feature = "ovm")]
        Box::new(OvmNetworkAddon::new()),
    ]
}

fn get_addon_by_namespace(namespace: &str) -> Option<Box<dyn Addon>> {
    let available_addons = get_available_addons();
    for addon in available_addons.into_iter() {
        if namespace.starts_with(&format!("{}", addon.get_namespace())) {
            return Some(addon);
        }
    }
    None
}

pub async fn start_server(
    network_binding: &str,
    // ctx: &Context,
) -> Result<ServerHandle, Box<dyn StdError>> {
    // info!(ctx.expect_logger(), "Starting server {}", network_binding);

    // let boxed_ctx = Data::new(ctx.clone());

    let gql_context: Data<RwLock<Option<GqlContext>>> = Data::new(RwLock::new(None));

    let server = HttpServer::new(move || {
        App::new()
            .app_data(gql_context.clone())
            .app_data(Data::new(new_graphql_schema()))
            // .app_data(boxed_ctx.clone())
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allowed_methods(vec!["POST", "GET", "OPTIONS", "DELETE"])
                    .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
                    .allowed_header(header::CONTENT_TYPE)
                    .supports_credentials()
                    .max_age(3600),
            )
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .service(
                web::scope("/api/v1")
                    .route("/runbooks/check", web::post().to(register_runbook))
                    .route("/runbooks/run", web::post().to(execute_runbook))
                    .route("/runbooks/run/state", web::get().to(execute_runbook))
                    .route("/discovery", web::get().to(discovery)),
            )
            .route("/ping", web::get().to(check_service_health))
            .service(
                web::scope("/gql/v1")
                    .route("/graphql?<request..>", web::get().to(get_graphql))
                    .route("/graphql", web::post().to(post_graphql))
                    .route("/subscriptions", web::get().to(subscriptions)),
            )
    })
    .workers(5)
    .bind(network_binding)?
    .run();

    let handle = server.handle();
    tokio::spawn(server);

    // Declare a pool of threads
    //

    Ok(handle)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RunbookRegistrationRequest {
    hcl_source: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RunbookRegistrationResponse {
    runbook_uuid: Uuid,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StartRunbookExecutionRequest {
    id: String,
    name: String,
    description: String,
    constructs: Vec<ConstructRequest>,
    hcl_source_legacy: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ConstructRequest {
    construct_type: String,
    id: String,
    description: String,
    value: Option<JsonValue>,
    action_id: Option<String>,
    namespace: Option<String>,
    inputs: Option<BTreeMap<String, JsonValue>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunbookExecutionStepState {
    Succeeded,
    Failed,
    Current,
    Next,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RunbookExecutionStep {
    state: RunbookExecutionStepState,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RunbookExecutionStateResponse {
    runbook_uuid: Uuid,
    steps: Vec<RunbookExecutionStep>,
}

pub async fn check_service_health(
    _req: HttpRequest,
    // ctx: Data<Context>,
    // graph_context: Data<GraphContext>,
) -> actix_web::Result<HttpResponse> {
    // info!(ctx.expect_logger(), "{} {}", req.method().as_str(), req.path());

    Ok(HttpResponseBuilder::new(StatusCode::OK).json(true))
}

pub async fn register_runbook(
    _req: HttpRequest,
    // ctx: Data<Context>,
    _payload: Json<RunbookRegistrationRequest>,
    // relayer_context: Data<RelayerContext>,
    // graph_context: Data<GraphContext>,
) -> actix_web::Result<HttpResponse> {
    // info!(ctx.expect_logger(), "{} {}", req.method().as_str(), req.path());

    Ok(HttpResponseBuilder::new(StatusCode::OK).json(true))
}

pub async fn execute_runbook(
    _req: HttpRequest,
    // ctx: Data<Context>,
    payload: Json<StartRunbookExecutionRequest>,
    // relayer_context: Data<RelayerContext>,
    gql_context: Data<RwLock<Option<GraphContext>>>,
) -> actix_web::Result<HttpResponse> {
    // info!(ctx.expect_logger(), "{} {}", req.method().as_str(), req.path());

    let mut reconstructed_source = "".to_string();
    let mut required_addons = HashSet::new();
    for construct in payload.constructs.iter() {
        reconstructed_source.push_str(&construct.construct_type);
        reconstructed_source.push_str(&format!(" \"{}\"", &construct.id));
        if let Some(ref namespace) = construct.namespace {
            required_addons.insert(namespace);
        }
        let command_id = match (&construct.namespace, &construct.action_id) {
            (Some(namespace), Some(id)) => format!("\"{}::{}\"", namespace, id),
            (None, Some(id)) => format!("\"{}\"", id),
            (Some(namespace), None) => format!(" \"{}\"", namespace),
            (None, None) => format!(""),
        };
        reconstructed_source.push_str(&format!(" {} {{\n", &command_id));
        if construct.construct_type.eq("variable") || construct.construct_type.eq("output") {
            reconstructed_source
                .push_str(&format!("  description = \"{}\"\n", construct.description));
            reconstructed_source.push_str(&format!("  editable = true\n"));

            if let Some(ref value) = construct.value {
                match value {
                    JsonValue::Null => {}
                    JsonValue::String(value) if value.starts_with("$") => {
                        reconstructed_source.push_str(&format!("  value = {}\n", &value[1..]));
                    }
                    JsonValue::String(value) => {
                        reconstructed_source.push_str(&format!("  value = \"{}\"\n", value));
                    }
                    JsonValue::Number(value) => {
                        reconstructed_source.push_str(&format!("  value = {}\n", &value));
                    }
                    _ => unreachable!(),
                }
            }
        } else if construct.construct_type.eq("action") {
            if let Some(ref inputs) = construct.inputs {
                for (key, value) in inputs.iter() {
                    match value {
                        JsonValue::Null => {}
                        JsonValue::String(value) if value.eq("null") => {}
                        JsonValue::String(value) if value.starts_with("$") => {
                            reconstructed_source.push_str(&format!(
                                "  {} = {}\n",
                                key,
                                &value[1..]
                            ));
                        }
                        JsonValue::String(value) => {
                            reconstructed_source.push_str(&format!("  {} = \"{}\"\n", key, value));
                        }
                        JsonValue::Number(value) => {
                            reconstructed_source.push_str(&format!("  {} = {}\n", key, &value));
                        }
                        _ => unreachable!(),
                    }
                }
            }
        }
        reconstructed_source.push_str(&format!("}}\n\n"));
    }
    for addon in required_addons.iter() {
        reconstructed_source.push_str(&format!("addon \"{}\" {{\n", addon));
        reconstructed_source.push_str(&format!(" chain_id = 11155111\n"));
        reconstructed_source.push_str(&format!(
            " rpc_api_url = \"https://sepolia.infura.io/v3/a063e95957aa4fd29319b2a53c31d481\"\n"
        ));
        reconstructed_source.push_str(&format!("}}\n\n"));

        reconstructed_source
            .push_str(&format!("signer \"account\" \"{}::web_wallet\" {{\n", addon));
        reconstructed_source.push_str(&format!(" description = \"Account\"\n"));
        reconstructed_source.push_str(&format!("}}\n\n"));
    }
    println!("{}", reconstructed_source);

    let runbook_name = payload.name.clone();
    let runbook_description = Some(payload.description.clone());
    let runbook_source = reconstructed_source;
    let dummy_location =
        FileLocation::from_path_string("/tmp/file.tx").map_err(|e| Box::<dyn StdError>::from(e))?;

    let mut runbook_sources = RunbookSources::new();
    runbook_sources.add_source(runbook_name.clone(), dummy_location, runbook_source);
    let runbook_id = RunbookId { org: None, workspace: None, name: runbook_name.clone() };
    let mut runbook = Runbook::new(runbook_id, runbook_description);

    let runbook_inputs = RunbookTopLevelInputsMap::new();
    let authorization_context = AuthorizationContext::empty();
    runbook
        .build_contexts_from_sources(
            runbook_sources,
            runbook_inputs,
            authorization_context,
            get_addon_by_namespace,
            CloudServiceContext::empty(),
        )
        .await
        .unwrap();

    runbook.enable_full_execution_mode();
    // info!(ctx.expect_logger(), "2");
    let runbook_description = runbook.description.clone();
    let supervisor_addon_data = {
        let flow = runbook.flow_contexts.first().unwrap();
        let mut addons = vec![];
        for addon in flow.execution_context.addon_instances.values() {
            if let Some(addon_defaults) = flow
                .workspace_context
                .addons_defaults
                .get(&(addon.package_id.did(), addon.addon_id.clone()))
            {
                if !addons.iter().any(|a: &SupervisorAddonData| a.addon_name.eq(&addon.addon_id)) {
                    addons.push(SupervisorAddonData::new(&addon.addon_id, addon_defaults));
                }
            }
        }
        addons
    };
    let (block_tx, block_rx) = channel::unbounded::<BlockEvent>();
    let (block_broadcaster, _) = tokio::sync::broadcast::channel(5);
    let (_action_item_updates_tx, _action_item_updates_rx) =
        channel::unbounded::<ActionItemRequest>();
    let (action_item_events_tx, action_item_events_rx) = tokio::sync::broadcast::channel(32);
    let block_store = Arc::new(RwLock::new(BTreeMap::new()));
    let (kill_loops_tx, kill_loops_rx) = channel::bounded(1);
    let (relayer_channel_tx, relayer_channel_rx) = channel::unbounded();

    let moved_block_tx = block_tx.clone();
    let moved_kill_loops_tx = kill_loops_tx.clone();
    // let moved_ctx = ctx.clone();

    let _ = hiro_system_kit::thread_named("Runbook Runloop").spawn(move || {
        let runloop_future =
            start_supervised_runbook_runloop(&mut runbook, moved_block_tx, action_item_events_rx);
        if let Err(diags) = hiro_system_kit::nestable_block_on(runloop_future) {
            for _diag in diags.iter() {
                // error!(moved_ctx.expect_logger(), "Runbook execution failed: {}", diag.message);
            }
            // if let Err(e) = write_runbook_transient_state(&mut runbook, moved_runbook_state) {
            //     println!("{} Failed to write transient runbook state: {}", red!("x"), e);
            // };
        } else {
            // if let Err(e) = write_runbook_state(&mut runbook, moved_runbook_state) {
            //     println!("{} Failed to write runbook state: {}", red!("x"), e);
            // };
        }
        if let Err(_e) = moved_kill_loops_tx.send(true) {
            // std::process::exit(1);
        }
    });

    // start web ui server
    {
        let mut gql_context = gql_context.write().await;
        *gql_context = Some(GqlContext {
            protocol_name: runbook_name.clone(),
            runbook_name: runbook_name.clone(),
            supervisor_addon_data,
            runbook_description,
            block_store: block_store.clone(),
            block_broadcaster: block_broadcaster.clone(),
            action_item_events_tx: action_item_events_tx.clone(),
        });
    }

    let channel_data = Arc::new(RwLock::new(None));
    let _relayer_context = RelayerContext {
        relayer_channel_tx: relayer_channel_tx.clone(),
        channel_data: channel_data.clone(),
    };

    let moved_relayer_channel_tx = relayer_channel_tx.clone();
    let moved_kill_loops_tx = kill_loops_tx.clone();
    let moved_action_item_events_tx = action_item_events_tx.clone();
    let _ = hiro_system_kit::thread_named("Relayer Interaction").spawn(move || {
        let future = start_relayer_event_runloop(
            channel_data,
            relayer_channel_rx,
            moved_relayer_channel_tx,
            moved_action_item_events_tx,
            moved_kill_loops_tx,
        );
        hiro_system_kit::nestable_block_on(future)
    });

    let moved_relayer_channel_tx = relayer_channel_tx.clone();
    let _block_store_handle = tokio::spawn(async move {
        loop {
            if let Ok(mut block_event) = block_rx.try_recv() {
                let mut block_store = block_store.write().await;
                let mut do_propagate_event = true;
                match block_event.clone() {
                    BlockEvent::Action(new_block) => {
                        let len = block_store.len();
                        block_store.insert(len, new_block.clone());
                    }
                    BlockEvent::Clear => {
                        *block_store = BTreeMap::new();
                    }
                    BlockEvent::UpdateActionItems(updates) => {
                        // for action item updates, track if we actually changed anything before propagating the event
                        do_propagate_event = false;
                        let mut filtered_updates = vec![];
                        for update in updates.iter() {
                            for (_, block) in block_store.iter_mut() {
                                let did_update = block.apply_action_item_updates(update.clone());
                                if did_update {
                                    do_propagate_event = true;
                                    filtered_updates.push(update.clone());
                                }
                            }
                        }
                        block_event = BlockEvent::UpdateActionItems(filtered_updates);
                    }
                    BlockEvent::Modal(new_block) => {
                        let len = block_store.len();
                        block_store.insert(len, new_block.clone());
                    }
                    BlockEvent::ProgressBar(new_block) => {
                        let len = block_store.len();
                        block_store.insert(len, new_block.clone());
                    }
                    BlockEvent::UpdateProgressBarStatus(update) => block_store
                        .iter_mut()
                        .filter(|(_, b)| b.uuid == update.progress_bar_uuid)
                        .for_each(|(_, b)| {
                            b.update_progress_bar_status(&update.construct_did, &update.new_status)
                        }),
                    BlockEvent::UpdateProgressBarVisibility(update) => block_store
                        .iter_mut()
                        .filter(|(_, b)| b.uuid == update.progress_bar_uuid)
                        .for_each(|(_, b)| b.visible = update.visible),
                    BlockEvent::RunbookCompleted => {
                        println!("\n{}", green!("Runbook complete!"));
                        break;
                    }
                    BlockEvent::Error(new_block) => {
                        let len = block_store.len();
                        block_store.insert(len, new_block.clone());
                    }
                    BlockEvent::Exit => break,
                }

                if do_propagate_event {
                    let _ = block_broadcaster.send(block_event.clone());
                    let _ = moved_relayer_channel_tx
                        .send(RelayerChannelEvent::ForwardEventToRelayer(block_event.clone()));
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            // println!("waiting for next block event");
        }
    });

    let _ = hiro_system_kit::thread_named("Kill Runloops Thread")
        .spawn(move || {
            let future = async {
                match kill_loops_rx.recv() {
                    Ok(_) => {
                        let _ = block_tx.send(BlockEvent::Exit);
                        let _ = relayer_channel_tx.send(RelayerChannelEvent::Exit);
                    }
                    Err(_) => {}
                };
            };

            hiro_system_kit::nestable_block_on(future)
        })
        .unwrap();

    // info!(ctx.expect_logger(), "Attempt to initialize execution channel");
    Ok(HttpResponseBuilder::new(StatusCode::OK).json(true))
}

async fn discovery() -> impl Responder {
    HttpResponse::Ok()
        .json(DiscoveryResponse { needs_credentials: false, client_type: ClientType::Operator })
}

async fn post_graphql(
    req: HttpRequest,
    payload: web::Payload,
    schema: Data<GraphqlSchema>,
    context: Data<RwLock<Option<GraphContext>>>,
    // ctx: Data<Context>,
) -> Result<HttpResponse, Error> {
    // info!(ctx.expect_logger(), "{} {}", req.method().as_str(), req.path());
    let context = context.write().await;
    let Some(context) = context.as_ref() else {
        return Err(actix_web::error::ErrorServiceUnavailable("Service Unavailable"));
    };
    graphql_handler(&schema, &context, req, payload).await
}

async fn get_graphql(
    req: HttpRequest,
    payload: web::Payload,
    schema: Data<GraphqlSchema>,
    context: Data<RwLock<Option<GraphContext>>>,
    // ctx: Data<Context>,
) -> Result<HttpResponse, Error> {
    // info!(ctx.expect_logger(), "{} {}", req.method().as_str(), req.path());
    let context = context.read().await;
    let Some(context) = context.as_ref() else {
        return Err(actix_web::error::ErrorServiceUnavailable("Service Unavailable"));
    };
    graphql_handler(&schema, &context, req, payload).await
}

async fn subscriptions(
    req: HttpRequest,
    stream: web::Payload,
    schema: Data<GraphqlSchema>,
    context: Data<RwLock<Option<GraphContext>>>,
    // ctx: Data<Context>,
) -> Result<HttpResponse, Error> {
    // info!(ctx.expect_logger(), "{} {}", req.method().as_str(), req.path());
    let context = context.read().await;
    let Some(context) = context.as_ref() else {
        return Err(actix_web::error::ErrorServiceUnavailable("Service Unavailable"));
    };
    let ctx = GraphContext {
        protocol_name: context.protocol_name.clone(),
        runbook_name: context.runbook_name.clone(),
        supervisor_addon_data: context.supervisor_addon_data.clone(),
        runbook_description: context.runbook_description.clone(),
        block_store: context.block_store.clone(),
        block_broadcaster: context.block_broadcaster.clone(),
        action_item_events_tx: context.action_item_events_tx.clone(),
    };
    let config = ConnectionConfig::new(ctx);
    let config = config.with_keep_alive_interval(Duration::from_secs(15));
    subscriptions::ws_handler(req, stream, schema.into_inner(), config).await
}
