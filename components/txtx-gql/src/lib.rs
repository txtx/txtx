use std::collections::HashMap;

use juniper::{EmptySubscription, RootNode};
use mutation::Mutation;
use query::Query;
use txtx_vm::types::Manual;

pub mod mutation;
pub mod query;
pub mod types;

pub struct Context {
    pub manuals: HashMap<String, Manual>,
}

impl juniper::Context for Context {}

pub type NestorGraphqlSchema =
    RootNode<'static, query::Query, mutation::Mutation, EmptySubscription<Context>>;

pub fn new_graphql_schema() -> NestorGraphqlSchema {
    NestorGraphqlSchema::new(Query, Mutation, EmptySubscription::new())
}
