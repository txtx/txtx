use std::pin::Pin;

use crate::{
    types::block::{GqlBlock, GqlSetActionItemStatus},
    Context,
};
use futures::Stream;
use juniper::{graphql_subscription, FieldError};
use txtx_core::kit::types::frontend::BlockEvent;

pub struct Subscription;

type GqlBlockStream = Pin<Box<dyn Stream<Item = Result<GqlBlock, FieldError>> + Send>>;

type GqlSetActionItemStatusStream =
    Pin<Box<dyn Stream<Item = Result<Vec<GqlSetActionItemStatus>, FieldError>> + Send>>;

type ClearBlockEventStream = Pin<Box<dyn Stream<Item = Result<bool, FieldError>> + Send>>;

#[graphql_subscription(
  context = Context,
)]
impl Subscription {
    async fn append_block_event(context: &Context) -> GqlBlockStream {
        let block_tx = context.block_broadcaster.clone();
        let mut block_rx = block_tx.subscribe();
        let stream = async_stream::stream! {
            loop {
              if let Ok(block_event) = block_rx.recv().await {
                match block_event {
                    BlockEvent::Append(block) => yield Ok(GqlBlock::new(block)),
                    _ => {}
                }

                }
              }

        };
        Box::pin(stream)
    }

    async fn update_action_items_event(context: &Context) -> GqlSetActionItemStatusStream {
        let block_tx = context.block_broadcaster.clone();
        let mut block_rx = block_tx.subscribe();
        let stream = async_stream::stream! {
            loop {
              if let Ok(block_event) = block_rx.recv().await {
                match block_event {
                    BlockEvent::UpdateActionItems(updates) => yield Ok(updates.into_iter().map(GqlSetActionItemStatus::new).collect()),
                    _ => {}
                }
              }
            }
        };
        Box::pin(stream)
    }

    async fn clear_blocks_event(context: &Context) -> ClearBlockEventStream {
        let block_tx = context.block_broadcaster.clone();
        let mut block_rx = block_tx.subscribe();
        let stream = async_stream::stream! {
            loop {
              if let Ok(block_event) = block_rx.recv().await {
                match block_event {
                  BlockEvent::Clear => yield Ok(true),
                    _ => {}
                }
              }
            }
        };
        Box::pin(stream)
    }
}
