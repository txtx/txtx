use std::pin::Pin;

use crate::{
    types::block::{
        GqlActionBlock, GqlActionItemRequestUpdate, GqlModalBlock, GqlProgressBarStatusUpdate,
        GqlProgressBarVisibilityUpdate, GqlProgressBlock,
    },
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
type GqlProgressBarStatusUpdateStream =
    Pin<Box<dyn Stream<Item = Result<GqlProgressBarStatusUpdate, FieldError>> + Send>>;
type GqlProgressBarVisibilityUpdateStream =
    Pin<Box<dyn Stream<Item = Result<GqlProgressBarVisibilityUpdate, FieldError>> + Send>>;

type GqlActionItemRequestUpdateStream =
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

    async fn update_progress_bar_status_event(
        context: &Context,
    ) -> GqlProgressBarStatusUpdateStream {
        let block_tx = context.block_broadcaster.clone();
        let mut block_rx = block_tx.subscribe();
        let stream = async_stream::stream! {
            loop {
              if let Ok(block_event) = block_rx.recv().await {
                match block_event {
                    BlockEvent::UpdateProgressBarStatus(update) => yield Ok(GqlProgressBarStatusUpdate::new(update)),
                    _ => {}
                }
              }
            }

        };
        Box::pin(stream)
    }
    async fn update_progress_bar_visibility_event(
        context: &Context,
    ) -> GqlProgressBarVisibilityUpdateStream {
        let block_tx = context.block_broadcaster.clone();
        let mut block_rx = block_tx.subscribe();
        let stream = async_stream::stream! {
            loop {
              if let Ok(block_event) = block_rx.recv().await {
                match block_event {
                  BlockEvent::UpdateProgressBarVisibility(update) => yield Ok(GqlProgressBarVisibilityUpdate::new(update)),
                    _ => {}
                }
              }
            }

        };
        Box::pin(stream)
    }

    async fn update_action_items_event(context: &Context) -> GqlActionItemRequestUpdateStream {
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
