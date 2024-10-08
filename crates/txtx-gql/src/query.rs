use crate::{
    types::{
        block::{GqlActionBlock, GqlErrorBlock, GqlModalBlock, GqlProgressBlock},
        runbook::RunbookMetadata,
    },
    Context,
};
use juniper_codegen::graphql_object;
use txtx_addon_kit::types::frontend::Panel;

pub struct Query;

#[graphql_object(
    context = Context,
)]
impl Query {
    fn api_version() -> &'static str {
        "1.0"
    }

    async fn action_blocks(context: &Context) -> Vec<GqlActionBlock> {
        let block_store = context.block_store.read().await;
        block_store
            .values()
            .cloned()
            .filter(|b| if let Panel::ActionPanel(_) = b.panel { true } else { false })
            .map(GqlActionBlock::new)
            .collect()
    }

    async fn modal_blocks(context: &Context) -> Vec<GqlModalBlock> {
        let block_store = context.block_store.read().await;
        block_store
            .values()
            .cloned()
            .filter(|b| if let Panel::ModalPanel(_) = b.panel { true } else { false })
            .map(GqlModalBlock::new)
            .collect()
    }

    async fn error_blocks(context: &Context) -> Vec<GqlErrorBlock> {
        let block_store = context.block_store.read().await;
        block_store
            .values()
            .cloned()
            .filter(|b| if let Panel::ErrorPanel(_) = b.panel { true } else { false })
            .map(GqlErrorBlock::new)
            .collect()
    }

    async fn progress_blocks(context: &Context) -> Vec<GqlProgressBlock> {
        let block_store = context.block_store.read().await;
        block_store
            .values()
            .cloned()
            .filter(|b| if let Panel::ProgressBar(_) = b.panel { true } else { false })
            .map(GqlProgressBlock::new)
            .collect()
    }

    fn runbook(context: &Context) -> RunbookMetadata {
        RunbookMetadata::new(
            &context.runbook_name,
            &context.registered_addons,
            &context.runbook_description,
        )
    }
}
