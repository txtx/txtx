use crate::Context;
use juniper_codegen::graphql_object;
use txtx_core::kit::types::frontend::ActionItemResponse;

pub struct Mutation;

#[graphql_object(
    context = Context,
)]
impl Mutation {
    fn api_version() -> &'static str {
        "1.0"
    }

    fn update_action_item(context: &Context, event: String) -> Result<String, String> {
        println!("received mutation");
        let event: ActionItemResponse = serde_json::from_str(&event).map_err(|e| e.to_string())?;
        let _ = context.action_item_events_tx.send(event);
        Ok("Ok".to_string())
    }
}
