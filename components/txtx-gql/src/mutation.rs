use crate::{Context, ContextData};
use juniper_codegen::graphql_object;
use serde_json::json;
use txtx_core::eval::run_constructs_evaluation;
use txtx_core::types::ConstructUuid;
use uuid::Uuid;

pub struct Mutation;

#[graphql_object(
    context = Context,
)]
impl Mutation {
    fn api_version() -> &'static str {
        "1.0"
    }

    fn update_command_input<'ctx>(
        context: &'ctx Context,
        manual_name: String,
        command_uuid: Uuid,
        input_name: String,
        value: String,
    ) -> Result<String, String> {
        println!("mutation!!! value: {}", value);
        let ContextData {
            manual,
            runtime_context,
        } = context
            .data
            .get(&manual_name)
            .ok_or(format!("could not fine manual {manual_name}"))?;

        let command_uuid = ConstructUuid::Local(command_uuid);
        let command_graph_node = match manual.write() {
            Ok(mut manual) => match manual.commands_instances.get(&command_uuid) {
                Some(command_instance) => {
                    let moved_command_instance = command_instance.clone();
                    match manual
                        .command_inputs_evaluation_results
                        .get_mut(&command_uuid)
                    {
                        Some(input_evaluation_results) => {
                            moved_command_instance.update_input_evaluation_results_from_user_input(
                                input_evaluation_results,
                                input_name,
                                value,
                            );
                            manual
                                .constructs_graph_nodes
                                .get(&command_uuid.value())
                                .cloned()
                        }
                        None => None,
                    }
                }
                None => None,
            },
            Err(e) => unimplemented!("could not acquire lock: {e}"),
        };
        match command_graph_node {
            Some(command_graph_node) => {
                match run_constructs_evaluation(
                    &manual,
                    runtime_context,
                    Some(command_graph_node),
                    context.eval_tx.clone(),
                ) {
                    Ok(()) => println!("successfully reevaluated constructs after mutation"),
                    Err(e) => println!("error reevaluating constructs after mutation: {:?}", e),
                }
            }
            None => {} // no evaluation needed if this construct is somehow not part of the graph
        }

        let result = match manual.read() {
            Ok(manual) => {
                let mut result = vec![];
                for (construct_uuid, command_instance) in manual.commands_instances.iter() {
                    let constructs_execution_results =
                        manual.constructs_execution_results.get(&construct_uuid);
                    let command_inputs_evaluation_results = manual
                        .command_inputs_evaluation_results
                        .get(&construct_uuid);
                    result.push(json!({
                        "constructUuid": construct_uuid,
                        "commandInstance": command_instance,
                        "commandInputsEvaluationResult": command_inputs_evaluation_results,
                        "constructsExecutionResult": constructs_execution_results
                    }));
                }
                result
            }
            Err(e) => unimplemented!("could not acquire lock: {e}"),
        };

        serde_json::to_string(&result).map_err(|e| format!("failed to serialize manual data {e}"))
    }
}
