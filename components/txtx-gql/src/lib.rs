use futures::lock::Mutex;
use juniper::{EmptySubscription, RootNode};
use mutation::Mutation;
use query::Query;
use std::{
    collections::HashMap,
    sync::{mpsc::Sender, Arc, RwLock},
};
use txtx_core::{
    kit::types::commands::EvalEvent,
    types::{Manual, RuntimeContext},
};

pub mod mutation;
pub mod query;
pub mod types;

pub struct Context {
    pub data: HashMap<String, ContextData>,
    pub eval_tx: Sender<EvalEvent>,
}

pub struct ContextData {
    pub manual: Arc<RwLock<Manual>>,
    pub runtime_context: Arc<RwLock<RuntimeContext>>,
}

impl juniper::Context for Context {}

pub type NestorGraphqlSchema =
    RootNode<'static, query::Query, mutation::Mutation, EmptySubscription<Context>>;

pub fn new_graphql_schema() -> NestorGraphqlSchema {
    NestorGraphqlSchema::new(Query, Mutation, EmptySubscription::new())
}
