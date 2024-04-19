use crate::types::constructs::Construct;
use crate::types::runbook::{GqlRunbook, ProtocolManifest, RunbookDescription};

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

    async fn construct(context: &Context, runbook_name: String, id: Uuid) -> Option<Construct> {
        let Some(ContextData { runbook, .. }) = context.data.get(&runbook_name) else {
            return None;
        };
        let uuid = ConstructUuid::from_uuid(&id);
        match runbook.read() {
            Ok(runbook) => {
                let Some(data) = runbook.commands_instances.get(&uuid) else {
                    return None;
                };
                let result = runbook.constructs_execution_results.get(&uuid).cloned();
                // Return item
                Some(Construct::new(&uuid, data, result))
            }
            Err(e) => unimplemented!("could not acquire lock: {e}"),
        }
    }

    async fn runbook(context: &Context, runbook_name: String) -> Option<GqlRunbook> {
        let Some(ContextData { runbook, .. }) = context.data.get(&runbook_name) else {
            return None;
        };
        match runbook.read() {
            Ok(runbook) => Some(GqlRunbook::new(runbook_name, runbook.clone())),
            Err(e) => unimplemented!("could not acquire lock: {e}"),
        }
    }

    async fn protocol(context: &Context) -> ProtocolManifest {
        let mut runbooks = vec![];
        for (id, ContextData { runbook, .. }) in context.data.iter() {
            match runbook.read() {
                Ok(runbook) => {
                    let _metadata = runbook.get_metadata_module();
                    let construct_uuids = runbook.commands_instances.keys().cloned().collect();
                    runbooks.push(RunbookDescription {
                        identifier: id.clone(),
                        name: Some(id.clone()),
                        description: runbook.description.clone(),
                        construct_uuids,
                    });
                }
                Err(e) => unimplemented!("could not acquire lock: {e}"),
            }
        }
        ProtocolManifest {
            name: context.protocol_name.clone(),
            runbooks,
        }
    }
}
