use crate::types::constructs::Construct;
use crate::types::runbook::{GqlRunbook, ProtocolManifest, RunbookDescription};

use crate::Context;
use juniper_codegen::graphql_object;
use txtx_core::types::ConstructUuid;
use uuid::Uuid;

pub struct Query;

#[graphql_object(
    context = Context,
)]
impl Query {
    fn api_version() -> &'static str {
        "1.0"
    }
}
