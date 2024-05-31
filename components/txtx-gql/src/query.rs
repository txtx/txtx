use crate::{
    types::{block::GqlBlock, runbook::RunbookDescription},
    Context,
};
use juniper_codegen::graphql_object;

pub struct Query;

#[graphql_object(
    context = Context,
)]
impl Query {
    fn api_version() -> &'static str {
        "1.0"
    }

    async fn blocks(context: &Context) -> Vec<GqlBlock> {
        let block_store = context.block_store.lock().await;
        block_store.values().cloned().map(GqlBlock::new).collect()
    }

    fn runbook(context: &Context) -> RunbookDescription {
        RunbookDescription {
            name: context.runbook_name.clone(),
            description: context.runbook_description.clone(),
        }
    }
}
