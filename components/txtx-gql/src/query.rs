use crate::types::constructs::Construct;
use crate::types::manual::ManualDescription;

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

    async fn construct(context: &Context, manual_name: String, id: Uuid) -> Option<Construct> {
        let uuid = ConstructUuid::from_uuid(&id);
        let Some(manual) = context.manuals.get(&manual_name) else {
            return None;
        };
        let Some(data) = manual.commands_instances.get(&uuid) else {
            return None;
        };
        let result = if let Some(result) = manual.constructs_execution_results.get(&uuid) {
            Some(result.clone())
        } else {
            None
        };

        // Return item
        Some(Construct::new(&uuid, data, result))
    }
    }

    async fn manuals(context: &Context) -> Vec<ManualDescription> {
        let mut manuals = vec![];
        for (id, manual) in context.manuals.iter() {
            let metadata = manual.get_metadata_module();
            manuals.push(ManualDescription {
                identifier: id.clone(),
                name: metadata.and_then(|m| Some(m.name.to_string())),
                description: None,
            })
        }
        manuals
    }
}
