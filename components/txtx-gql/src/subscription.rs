use std::pin::Pin;

use crate::{
    types::block::{GqlActionBlock, GqlActionItemRequestUpdate, GqlModalBlock, GqlProgressBlock},
    Context,
};
use futures::Stream;
use juniper::{graphql_subscription, FieldError};
use txtx_core::kit::types::frontend::BlockEvent;

pub struct Subscription;

type GqlActionBlockStream = Pin<Box<dyn Stream<Item = Result<GqlActionBlock, FieldError>> + Send>>;
type GqlModalBlockStream = Pin<Box<dyn Stream<Item = Result<GqlModalBlock, FieldError>> + Send>>;
type GqlProgressBlockStream =
    Pin<Box<dyn Stream<Item = Result<GqlProgressBlock, FieldError>> + Send>>;

type GqlSetActionItemStatusStream =
    Pin<Box<dyn Stream<Item = Result<Vec<GqlActionItemRequestUpdate>, FieldError>> + Send>>;

type ClearBlockEventStream = Pin<Box<dyn Stream<Item = Result<bool, FieldError>> + Send>>;

#[graphql_subscription(
  context = Context,
)]
impl Subscription {
    async fn action_block_event(context: &Context) -> GqlActionBlockStream {
        let block_tx = context.block_broadcaster.clone();
        let mut block_rx = block_tx.subscribe();
        let stream = async_stream::stream! {
            loop {
              if let Ok(block_event) = block_rx.recv().await {
                match block_event {
                    BlockEvent::Action(block) => yield Ok(GqlActionBlock::new(block)),
                    _ => {}
                }
              }
            }

        };
        Box::pin(stream)
    }

    async fn modal_block_event(context: &Context) -> GqlModalBlockStream {
        let block_tx = context.block_broadcaster.clone();
        let mut block_rx = block_tx.subscribe();
        let stream = async_stream::stream! {
            loop {
              if let Ok(block_event) = block_rx.recv().await {
                match block_event {
                    BlockEvent::Modal(block) => yield Ok(GqlModalBlock::new(block)),
                    _ => {}
                }
              }
            }

        };
        Box::pin(stream)
    }

    async fn progress_block_event(context: &Context) -> GqlProgressBlockStream {
        let block_tx = context.block_broadcaster.clone();
        let mut block_rx = block_tx.subscribe();
        let stream = async_stream::stream! {
            loop {
              if let Ok(block_event) = block_rx.recv().await {
                match block_event {
                    BlockEvent::ProgressBar(block) => yield Ok(GqlProgressBlock::new(block)),
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
                    BlockEvent::UpdateActionItems(updates) => yield Ok(updates.into_iter().map(GqlActionItemRequestUpdate::new).collect()),
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
