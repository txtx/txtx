use crate::types::constructs::Construct;
use crate::types::manual::{GqlManual, ManualDescription};

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

    async fn manual(context: &Context, manual_name: String) -> Option<GqlManual> {
        let Some(data) = context.manuals.get(&manual_name) else {
            return None;
        };
        Some(GqlManual::new(manual_name, data.clone()))
    }

    async fn manuals(context: &Context) -> Vec<ManualDescription> {
        let mut manuals = vec![];
        for (id, manual) in context.manuals.iter() {
            let _metadata = manual.get_metadata_module();
            let construct_uuids = manual.commands_instances.keys().cloned().collect();
            manuals.push(ManualDescription {
                identifier: id.clone(),
                name: Some(id.clone()),
                description: manual.description.clone(),
                construct_uuids,
            })
        }
        manuals
    }
}
