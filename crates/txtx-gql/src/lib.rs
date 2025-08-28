use juniper::RootNode;
use mutation::Mutation;
use query::Query;
use std::{collections::BTreeMap, sync::Arc};
use subscription::Subscription;
use tokio::sync::RwLock;
use txtx_addon_kit::types::frontend::{
    ActionItemResponse, Block, BlockEvent, LogEvent, SupervisorAddonData,
};

pub mod mutation;
pub mod query;
pub mod subscription;
pub mod types;

pub use txtx_addon_kit as kit;

#[derive(Clone, Debug)]
pub struct Context {
    pub protocol_name: String,
    pub runbook_name: String,
    pub supervisor_addon_data: Vec<SupervisorAddonData>,
    pub runbook_description: Option<String>,
    pub block_store: Arc<RwLock<BTreeMap<usize, Block>>>,
    pub log_store: Arc<RwLock<Vec<LogEvent>>>,
    pub block_broadcaster: tokio::sync::broadcast::Sender<BlockEvent>,
    pub log_broadcaster: tokio::sync::broadcast::Sender<LogEvent>,
    pub action_item_events_tx: tokio::sync::broadcast::Sender<ActionItemResponse>,
}

impl juniper::Context for Context {}

pub type GraphqlSchema = RootNode<'static, Query, Mutation, Subscription>;

pub fn new_graphql_schema() -> GraphqlSchema {
    GraphqlSchema::new(Query, Mutation, Subscription)
}
