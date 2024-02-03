use crate::types::manual::ManualDescription;
use crate::types::module::Module;
use crate::types::output::Output;
use crate::types::variable::Variable;

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

    async fn variable(context: &Context, manual_name: String, id: Uuid) -> Option<Variable> {
        let uuid = ConstructUuid::from_uuid(&id);
        let Some(data) = context
            .manuals
            .get(&manual_name)?
            .constructs
            .get(&uuid)
            .and_then(|c| c.as_variable())
        else {
            return None;
        };
        // Return item
        Some(Variable::new(&uuid, data))
    }

    async fn variables(
        context: &Context,
        manual_name: String,
        package_id: Option<Uuid>,
    ) -> Option<Vec<Variable>> {
        let mut variables_uuids = vec![];
        let manual = context.manuals.get(&manual_name)?;
        for (id, package) in manual.packages.iter() {
            let package = match package_id {
                Some(package_id) if package_id.eq(&id.value()) => package,
                None => package,
                _ => continue,
            };
            for variable_uuid in package.variables_uuids.iter() {
                variables_uuids.push(variable_uuid);
            }
        }
        // Return collection
        let mut variables = vec![];
        for uuid in variables_uuids.iter() {
            let Some(data) = context
                .manuals
                .get(&manual_name)?
                .constructs
                .get(&uuid)
                .and_then(|c| c.as_variable())
            else {
                continue;
            };

            variables.push(Variable::new(&uuid, data));
        }
        Some(variables)
    }

    async fn module(context: &Context, manual_name: String, id: Uuid) -> Option<Module> {
        let uuid = ConstructUuid::from_uuid(&id);
        let Some(data) = context
            .manuals
            .get(&manual_name)?
            .constructs
            .get(&uuid)
            .and_then(|c| c.as_module())
        else {
            return None;
        };
        // Return item
        Some(Module::new(&uuid, data))
    }

    async fn modules(
        context: &Context,
        manual_name: String,
        package_id: Option<Uuid>,
    ) -> Option<Vec<Module>> {
        let mut modules_uuids = vec![];
        let manual = context.manuals.get(&manual_name)?;
        for (id, package) in manual.packages.iter() {
            let package = match package_id {
                Some(package_id) if package_id.eq(&id.value()) => package,
                None => package,
                _ => continue,
            };
            for module_uuid in package.modules_uuids.iter() {
                modules_uuids.push(module_uuid);
            }
        }
        // Return collection
        let mut modules = vec![];
        for uuid in modules_uuids.iter() {
            let Some(data) = context
                .manuals
                .get(&manual_name)?
                .constructs
                .get(&uuid)
                .and_then(|c| c.as_module())
            else {
                continue;
            };

            modules.push(Module::new(&uuid, data));
        }
        Some(modules)
    }

    async fn output(context: &Context, manual_name: String, id: Uuid) -> Option<Output> {
        let uuid = ConstructUuid::from_uuid(&id);
        let Some(data) = context
            .manuals
            .get(&manual_name)?
            .constructs
            .get(&uuid)
            .and_then(|c| c.as_output())
        else {
            return None;
        };
        // Return item
        Some(Output::new(&uuid, data))
    }

    async fn outputs(
        context: &Context,
        manual_name: String,
        package_id: Option<Uuid>,
    ) -> Option<Vec<Output>> {
        let mut outputs_uuids = vec![];
        let manual = context.manuals.get(&manual_name)?;
        for (id, package) in manual.packages.iter() {
            let package = match package_id {
                Some(package_id) if package_id.eq(&id.value()) => package,
                None => package,
                _ => continue,
            };
            for output_uuid in package.outputs_uuids.iter() {
                outputs_uuids.push(output_uuid);
            }
        }
        // Return collection
        let mut outputs = vec![];
        for uuid in outputs_uuids.iter() {
            let Some(data) = context
                .manuals
                .get(&manual_name)?
                .constructs
                .get(&uuid)
                .and_then(|c| c.as_output())
            else {
                continue;
            };

            outputs.push(Output::new(&uuid, data));
        }
        Some(outputs)
    }

    async fn manuals(context: &Context) -> Vec<ManualDescription> {
        let mut manuals = vec![];
        for (id, manual) in context.manuals.iter() {
            let metadata = manual.get_metadata_module();
            manuals.push(ManualDescription {
                identifier: id.clone(),
                name: metadata
                    .and_then(|m| m.name.as_ref())
                    .and_then(|n| Some(n.to_string())),
                description: metadata
                    .and_then(|m| m.description.as_ref())
                    .and_then(|n| Some(n.to_string())),
            })
        }
        manuals
    }
}
