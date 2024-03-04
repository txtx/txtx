use juniper::{EmptySubscription, RootNode};
use mutation::Mutation;
use query::Query;
use std::{collections::HashMap, sync::RwLock};
use txtx_core::types::{Manual, RuntimeContext};

pub mod mutation;
pub mod query;
pub mod types;

pub struct Context {
    pub data: HashMap<String, ContextData>,
}

pub struct ContextData {
    pub manual: RwLock<Manual>,
    pub runtime_context: RwLock<RuntimeContext>,
}

impl juniper::Context for Context {}

pub type NestorGraphqlSchema =
    RootNode<'static, query::Query, mutation::Mutation, EmptySubscription<Context>>;

pub fn new_graphql_schema() -> NestorGraphqlSchema {
    NestorGraphqlSchema::new(Query, Mutation, EmptySubscription::new())
}
