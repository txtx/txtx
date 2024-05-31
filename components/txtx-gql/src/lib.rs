use juniper::RootNode;
use mutation::Mutation;
use query::Query;
use std::{collections::BTreeMap, sync::Arc};
use subscription::Subscription;
use tokio::sync::Mutex;
use txtx_core::kit::{
    channel::Sender,
    types::frontend::{ActionItemResponse, Block, BlockEvent},
};
use uuid::Uuid;

pub mod mutation;
pub mod query;
pub mod subscription;
pub mod types;

pub struct Context {
    pub protocol_name: String,
    pub runbook_name: String,
    pub runbook_description: Option<String>,
    pub block_store: Arc<Mutex<BTreeMap<Uuid, Block>>>,
    pub block_broadcaster: tokio::sync::broadcast::Sender<BlockEvent>,
    pub action_item_events_tx: Sender<ActionItemResponse>,
}

impl juniper::Context for Context {}

pub type GraphqlSchema = RootNode<'static, Query, Mutation, Subscription>;

pub fn new_graphql_schema() -> GraphqlSchema {
    GraphqlSchema::new(Query, Mutation, Subscription)
}
