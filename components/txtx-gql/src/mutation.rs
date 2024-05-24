use crate::{Context, ContextData};
use juniper_codegen::graphql_object;
use serde_json::json;
use txtx_core::eval::{
    get_sorted_nodes, is_child_of_node, prepare_constructs_reevaluation, run_constructs_evaluation,
};
use txtx_core::kit::types::commands::CommandInstanceStateMachineInput;
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
        runbook_name: String,
        command_uuid: Uuid,
        input_name: String,
        value: String,
    ) -> Result<String, String> {
        let ContextData {
            runbook,
            runtime_context,
        } = context
            .data
            .get(&runbook_name)
            .ok_or(format!("could not find runbook {runbook_name}"))?;

        let command_uuid = ConstructUuid::Local(command_uuid);
        let command_graph_node = match runbook.write() {
            Ok(mut runbook) => {
                let graph_node = match runbook.commands_instances.get(&command_uuid) {
                    Some(command_instance) => {
                        let moved_command_instance = command_instance.clone();
                        match runbook
                            .command_inputs_evaluation_results
                            .get_mut(&command_uuid)
                        {
                            Some(input_evaluation_results) => {
                                moved_command_instance
                                    .update_input_evaluation_results_from_user_input(
                                        input_evaluation_results,
                                        input_name,
                                        value,
                                    );
                                runbook
                                    .constructs_graph_nodes
                                    .get(&command_uuid.value())
                                    .cloned()
                            }
                            None => None,
                        }
                    }
                    None => None,
                };
                match runbook.commands_instances.get_mut(&command_uuid) {
                    Some(command_instance) => match command_instance.state.lock() {
                        Ok(mut state_machine) => {
                            state_machine
                                .consume(&CommandInstanceStateMachineInput::ReEvaluate)
                                .unwrap();
                        }
                        Err(_) => unimplemented!(),
                    },
                    None => {}
                };
                graph_node
            }
            Err(e) => unimplemented!("could not acquire lock: {e}"),
        };
        match command_graph_node {
            Some(command_graph_node) => {
                prepare_constructs_reevaluation(&runbook, command_graph_node);
                match run_constructs_evaluation(
                    &runbook,
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

        let result = match runbook.read() {
            Ok(runbook) => {
                let mut result = vec![];
                let ordered_nodes = get_sorted_nodes(runbook.constructs_graph.clone());
                let graph = runbook.constructs_graph.clone();

                for (i, node) in ordered_nodes.into_iter().enumerate() {
                    let uuid = graph
                        .node_weight(node)
                        .expect("unable to retrieve construct");
                    let construct_uuid = ConstructUuid::Local(uuid.clone());

                    let Some(command_instance) = runbook.commands_instances.get(&construct_uuid)
                    else {
                        continue;
                    };

                    let is_child_of_root = is_child_of_node(runbook.graph_root, node, &graph);

                    let constructs_execution_results =
                        match runbook.constructs_execution_results.get(&construct_uuid) {
                            None => None,
                            Some(result) => match result {
                                Ok(result) => Some(serde_json::to_value(result).map_err(|e| {
                                    format!("failed to serialize runbook data {e}")
                                })?),
                                Err(e) => Some(json!({"error": e})),
                            },
                        };
                    let command_inputs_evaluation_results = runbook
                        .command_inputs_evaluation_results
                        .get(&construct_uuid);
                    result.push(json!({
                        "readonly": !is_child_of_root,
                        "index": i,
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

        serde_json::to_string(&result).map_err(|e| format!("failed to serialize runbook data {e}"))
    }
}
