use crate::Context;
use juniper_codegen::graphql_object;
use serde_json::json;
use txtx_core::eval::{
    get_sorted_nodes, is_child_of_node, prepare_constructs_reevaluation, run_constructs_evaluation,
};
use txtx_core::kit::types::commands::CommandInstanceStateMachineInput;
use txtx_core::types::ConstructUuid;
use uuid::Uuid;

pub struct Mutation;

#[graphql_object(
    context = Context,
)]
impl Mutation {
    fn api_version() -> &'static str {
        "1.0"
    }
}
