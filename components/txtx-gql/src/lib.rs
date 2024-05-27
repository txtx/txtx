use juniper::{EmptySubscription, RootNode};
use mutation::Mutation;
use query::Query;
use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};
use txtx_core::kit::{
    channel::Sender,
    types::frontend::{ActionItemEvent, Block},
};
use uuid::Uuid;

pub mod mutation;
pub mod query;
pub mod types;

pub struct Context {
    pub protocol_name: String,
    pub block_store: Arc<RwLock<BTreeMap<Uuid, Block>>>,
    pub action_item_events_tx: Sender<ActionItemEvent>,
}

impl juniper::Context for Context {}

pub type NestorGraphqlSchema =
    RootNode<'static, query::Query, mutation::Mutation, EmptySubscription<Context>>;

pub fn new_graphql_schema() -> NestorGraphqlSchema {
    NestorGraphqlSchema::new(Query, Mutation, EmptySubscription::new())
}
