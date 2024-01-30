use crate::types::variable::Variable;
use crate::Context;
use juniper_codegen::graphql_object;
use uuid::Uuid;

pub struct Mutation;

#[graphql_object(
    context = Context,
)]
impl Mutation {
    fn api_version() -> &'static str {
        "1.0"
    }

    async fn sign_transaction(_context: &Context, _id: Uuid) -> Option<Variable> {
        None
    }
}
