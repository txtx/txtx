use juniper::{EmptySubscription, RootNode};
use mutation::Mutation;
use query::Query;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use txtx_core::{
    channel::{Receiver, Sender},
    types::{
        frontend::{Block, ChecklistAction, ChecklistActionEvent},
        Runbook, RuntimeContext,
    },
};

pub mod mutation;
pub mod query;
pub mod types;

pub struct Context {
    pub protocol_name: String,
    pub data: HashMap<String, ContextData>,
    pub block_rx: Receiver<Block>,
    pub checklist_action_updates_rx: Receiver<ChecklistAction>,
    pub checklist_action_events_tx: Sender<ChecklistActionEvent>,
}

pub struct ContextData {
    pub runbook: Arc<RwLock<Runbook>>,
    pub runtime_context: Arc<RwLock<RuntimeContext>>,
}

impl juniper::Context for Context {}

pub type NestorGraphqlSchema =
    RootNode<'static, query::Query, mutation::Mutation, EmptySubscription<Context>>;

pub fn new_graphql_schema() -> NestorGraphqlSchema {
    NestorGraphqlSchema::new(Query, Mutation, EmptySubscription::new())
}
