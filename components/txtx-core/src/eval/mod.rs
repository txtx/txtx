use std::collections::{BTreeSet, HashMap, VecDeque};

use crate::{types::Manual, AddonsContext};
use daggy::Walker;
use txtx_addon_kit::types::{commands::CommandExecutionResult, ConstructUuid};

pub fn run_constructs_evaluation(
    manual: &Manual,
    addons_ctx: &AddonsContext,
) -> Result<(), String> {
    let root = manual.graph_root;
    let g = &manual.constructs_graph;

    let mut nodes_to_visit = VecDeque::new();
    let mut visited_nodes_to_process = BTreeSet::new();

    nodes_to_visit.push_front(root);
    while let Some(node) = nodes_to_visit.pop_front() {
        // All the parents must have been visited first
        for (_, parent) in g.parents(node).iter(&g) {
            if !visited_nodes_to_process.contains(&parent) {
                nodes_to_visit.push_back(node)
            }
        }
        // Enqueue all the children
        for (_, child) in g.children(node).iter(&g) {
            nodes_to_visit.push_back(child);
        }
        // Mark node as visited
        visited_nodes_to_process.insert(node);
    }

    visited_nodes_to_process.remove(&root);

    let mut constructs_execution_results: HashMap<ConstructUuid, CommandExecutionResult> =
        HashMap::new();
    for node in visited_nodes_to_process.into_iter() {
        let uuid = g.node_weight(node).expect("unable to retrieve construct");
        let construct_uuid = ConstructUuid::Local(uuid.clone());
        let command_instance = manual
            .commands_instances
            .get(&construct_uuid)
            .expect("unable to retrieve construct");

        let mut dependencies_execution_results: HashMap<ConstructUuid, &CommandExecutionResult> =
            HashMap::new();

        // Retrieve the construct_uuid of the inputs
        // Collect the outputs
        let references_expressions = command_instance
            .get_references_expressions_from_inputs()
            .unwrap();
        let (package_uuid, _) = manual.constructs_locations.get(&construct_uuid).unwrap();
        for expr in references_expressions.into_iter() {
            let res = manual
                .try_resolve_construct_reference_in_expression(package_uuid, &expr)
                .unwrap();
            if let Some(dependency) = res {
                let evaluation_result_opt = constructs_execution_results.get(&dependency);
                if let Some(evaluation_result) = evaluation_result_opt {
                    dependencies_execution_results.insert(dependency, evaluation_result);
                }
            }
        }

        let evaluated_inputs =
            command_instance.perform_inputs_evaluation(&dependencies_execution_results);
        let execution_result = command_instance.perform_execution(&evaluated_inputs);
        constructs_execution_results.insert(construct_uuid, execution_result);
    }
    Ok(())
}

