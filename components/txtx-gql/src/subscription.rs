use std::pin::Pin;

use crate::{types::block::GqlBlockEvent, Context};
use futures::Stream;
use juniper::{graphql_subscription, FieldError};

pub struct Subscription;

type GqlBlockStream = Pin<Box<dyn Stream<Item = Result<GqlBlockEvent, FieldError>> + Send>>;
#[graphql_subscription(
  context = Context,
)]
impl Subscription {
    async fn blocks(context: &Context) -> GqlBlockStream {
        let block_tx = context.block_broadcaster.clone();
        let mut block_rx = block_tx.subscribe();
        let stream = async_stream::stream! {
            loop {
              if let Ok(block_event) = block_rx.recv().await {
                yield Ok(GqlBlockEvent::new(block_event));
              }
            }
        };
        Box::pin(stream)
    }
}
