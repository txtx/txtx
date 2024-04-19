use crate::types::constructs::Construct;
use crate::types::runbook::{GqlManual, ManualDescription, ProtocolManifest};

use crate::{Context, ContextData};
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
        let Some(ContextData { manual, .. }) = context.data.get(&manual_name) else {
            return None;
        };
        let uuid = ConstructUuid::from_uuid(&id);
        match manual.read() {
            Ok(manual) => {
                let Some(data) = manual.commands_instances.get(&uuid) else {
                    return None;
                };
                let result = manual.constructs_execution_results.get(&uuid).cloned();
                // Return item
                Some(Construct::new(&uuid, data, result))
            }
            Err(e) => unimplemented!("could not acquire lock: {e}"),
        }
    }

    async fn manual(context: &Context, manual_name: String) -> Option<GqlManual> {
        let Some(ContextData { manual, .. }) = context.data.get(&manual_name) else {
            return None;
        };
        match manual.read() {
            Ok(manual) => Some(GqlManual::new(manual_name, manual.clone())),
            Err(e) => unimplemented!("could not acquire lock: {e}"),
        }
    }

    async fn protocol(context: &Context) -> ProtocolManifest {
        let mut manuals = vec![];
        for (id, ContextData { manual, .. }) in context.data.iter() {
            match manual.read() {
                Ok(manual) => {
                    let _metadata = manual.get_metadata_module();
                    let construct_uuids = manual.commands_instances.keys().cloned().collect();
                    manuals.push(ManualDescription {
                        identifier: id.clone(),
                        name: Some(id.clone()),
                        description: manual.description.clone(),
                        construct_uuids,
                    });
                }
                Err(e) => unimplemented!("could not acquire lock: {e}"),
            }
        }
        ProtocolManifest {
            name: context.protocol_name.clone(),
            manuals,
        }
    }
}
