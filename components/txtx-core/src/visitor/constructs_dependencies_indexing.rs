use daggy::{Dag, NodeIndex, Walker};
use indexmap::IndexSet;
use petgraph::algo::toposort;
use std::collections::{HashSet, VecDeque};
use txtx_addon_kit::types::{diagnostics::Diagnostic, ConstructDid, PackageDid};

use crate::types::{Runbook, RuntimeContext};

pub fn run_constructs_dependencies_indexing(
    runbook: &mut Runbook,
    runtime_ctx: &mut RuntimeContext,
) -> Result<
    (
        Vec<(ConstructDid, ConstructDid)>,
        Vec<(PackageDid, PackageDid)>,
    ),
    Vec<Diagnostic>,
> {
    let mut runbook_execution_context = runbook.execution_context.clone();
    runbook
        .resolution_context
        .seed_environment_variables(runtime_ctx);

    let mut constructs_edges = vec![];
    let packages_edges = vec![];
    let mut diags = vec![];

    let packages = runbook.resolution_context.packages.clone();

    for (package_did, package) in packages.iter() {
        for construct_did in package.imports_uuids.iter() {
            let construct = runbook_execution_context
                .commands_instances
                .get(construct_did)
                .unwrap();
            for _dep in construct.collect_dependencies().iter() {} // todo
        }
        for construct_did in package.variables_uuids.iter() {
            let construct = runbook_execution_context
                .commands_instances
                .get(construct_did)
                .unwrap();
            for (input, dep) in construct.collect_dependencies().iter() {
                let result = runbook
                    .resolution_context
                    .try_resolve_construct_reference_in_expression(
                        package_did,
                        dep,
                        &runbook_execution_context,
                    );
                if let Ok(Some((resolved_construct_did, _, _))) = result {
                    constructs_edges.push((construct_did.clone(), resolved_construct_did));
                } else {
                    diags.push(diagnosed_error!(
                        "input '{}': unable to resolve '{}'",
                        construct.name,
                        dep
                    ));
                }
            }
        }
        for construct_did in package.modules_uuids.iter() {
            let construct = runbook_execution_context
                .commands_instances
                .get(construct_did)
                .unwrap();
            for (input, dep) in construct.collect_dependencies().iter() {
                let result = runbook
                    .resolution_context
                    .try_resolve_construct_reference_in_expression(
                        package_did,
                        dep,
                        &runbook_execution_context,
                    );
                if let Ok(Some((resolved_construct_did, _, _))) = result {
                    constructs_edges.push((construct_did.clone(), resolved_construct_did));
                } else {
                    diags.push(diagnosed_error!(
                        "module '{}': unable to resolve '{}'",
                        construct.name,
                        dep
                    ));
                }
            }
        }
        for construct_did in package.outputs_uuids.iter() {
            let construct = runbook_execution_context
                .commands_instances
                .get(construct_did)
                .unwrap();
            for (input, dep) in construct.collect_dependencies().iter() {
                let result = runbook
                    .resolution_context
                    .try_resolve_construct_reference_in_expression(
                        package_did,
                        dep,
                        &runbook_execution_context,
                    );
                if let Ok(Some((resolved_construct_did, _, _))) = result {
                    constructs_edges.push((construct_did.clone(), resolved_construct_did));
                } else {
                    diags.push(diagnosed_error!(
                        "output '{}': unable to resolve '{}'",
                        construct.name,
                        dep
                    ));
                }
            }
        }
        let mut wallets = VecDeque::new();
        let mut instantiated_wallets = HashSet::new();
        for construct_did in package.addons_uuids.iter() {
            let command_instance = runbook_execution_context
                .commands_instances
                .get(construct_did)
                .unwrap();
            for (input, dep) in command_instance.collect_dependencies().iter() {
                let result = runbook
                    .resolution_context
                    .try_resolve_construct_reference_in_expression(
                        package_did,
                        dep,
                        &runbook_execution_context,
                    );
                if let Ok(Some((resolved_construct_did, _, _))) = result {
                    if let Some(_) = runbook_execution_context
                        .signing_commands_instances
                        .get(&resolved_construct_did)
                    {
                        wallets.push_front((resolved_construct_did.clone(), true));
                        instantiated_wallets.insert(resolved_construct_did.clone());
                    }
                    constructs_edges.push((construct_did.clone(), resolved_construct_did));
                } else {
                    diags.push(diagnosed_error!(
                        "action '{}': unable to resolve '{}'",
                        command_instance.name,
                        dep
                    ));
                }
            }
        }
        // todo: should we constrain to wallets depending on wallets?
        for construct_did in package.signing_commands_uuids.iter() {
            let wallet_instance = runbook_execution_context
                .signing_commands_instances
                .get(construct_did)
                .unwrap();
            for (input, dep) in wallet_instance.collect_dependencies().iter() {
                let result = runbook
                    .resolution_context
                    .try_resolve_construct_reference_in_expression(
                        package_did,
                        dep,
                        &runbook_execution_context,
                    );
                if let Ok(Some((resolved_construct_did, _, _))) = result {
                    if !instantiated_wallets.contains(&resolved_construct_did) {
                        wallets.push_front((resolved_construct_did.clone(), false))
                    }
                    runbook_execution_context
                        .signing_commands_state
                        .as_mut()
                        .unwrap()
                        .create_new_wallet(
                            &resolved_construct_did,
                            &resolved_construct_did.value().to_string(),
                        );
                    constructs_edges.push((construct_did.clone(), resolved_construct_did));
                } else {
                    diags.push(diagnosed_error!(
                        "wallet '{}': unable to resolve '{}'",
                        wallet_instance.name,
                        dep
                    ));
                }
            }
        }
        // this is the most idiomatic way I could find to get unique values from a hash set
        let mut seen_wallets = HashSet::new();
        wallets.retain(|w| seen_wallets.insert(w.clone()));
        runbook.resolution_context.instantiated_signing_commands = wallets;
    }

    for (src, dst) in constructs_edges.iter() {
        let constructs_graph_nodes = runbook
            .resolution_context
            .constructs_dag_node_lookup
            .clone();

        let src_node_index = constructs_graph_nodes.get(&src).unwrap();
        let dst_node_index = constructs_graph_nodes.get(&dst).unwrap();

        if let Some(edge_to_root) = runbook.resolution_context.constructs_dag.find_edge(
            runbook.resolution_context.graph_root,
            src_node_index.clone(),
        ) {
            runbook
                .resolution_context
                .constructs_dag
                .remove_edge(edge_to_root);
        }
        runbook
            .resolution_context
            .constructs_dag
            .add_edge(dst_node_index.clone(), src_node_index.clone(), 1)
            .unwrap();
    }

    if diags.is_empty() {
        for (construct_did, instantiated) in runbook
            .resolution_context
            .instantiated_signing_commands
            .iter()
        {
            runbook_execution_context
                .order_for_signing_commands_initialization
                .push(construct_did.clone());

            if *instantiated {
                let mut dependencies = vec![];
                let node_index = runbook
                    .resolution_context
                    .constructs_dag_node_lookup
                    .get(construct_did)
                    .expect("construct_did not indexed in graph");
                let descendants = get_descendants_of_node(
                    node_index.clone(),
                    runbook.resolution_context.constructs_dag.clone(),
                );
                for descendant in descendants.into_iter() {
                    let dependent_construct_did = runbook
                        .resolution_context
                        .constructs_dag
                        .node_weight(descendant)
                        .expect("construct_did not indexed in graph");
                    dependencies.push(dependent_construct_did.clone());
                }
                runbook_execution_context
                    .signing_commands_dependencies
                    .insert(construct_did.clone(), dependencies);
            }
        }

        for node in get_sorted_nodes(runbook.resolution_context.constructs_dag.clone()) {
            let construct_did = runbook
                .resolution_context
                .constructs_dag
                .node_weight(node)
                .expect("construct_did not indexed in graph");
            runbook_execution_context
                .order_for_commands_execution
                .push(construct_did.clone());
        }

        for (construct_did, _) in runbook_execution_context.commands_instances.iter() {
            let mut dependencies = vec![];
            let node_index = runbook
                .resolution_context
                .constructs_dag_node_lookup
                .get(construct_did)
                .expect("construct_did not indexed in graph");
            let descendants = get_descendants_of_node(
                node_index.clone(),
                runbook.resolution_context.constructs_dag.clone(),
            );
            for descendant in descendants.into_iter() {
                let dependent_construct_did = runbook
                    .resolution_context
                    .constructs_dag
                    .node_weight(descendant)
                    .expect("construct_did not indexed in graph");
                dependencies.push(dependent_construct_did.clone());
            }
            runbook_execution_context
                .commands_dependencies
                .insert(construct_did.clone(), dependencies);
        }

        runbook.execution_context = runbook_execution_context;
        return Ok((constructs_edges, packages_edges));
    }

    Err(diags)
}

/// Gets all descendants of `node` within `graph`.
pub fn get_descendants_of_node(
    node: NodeIndex,
    graph: Dag<ConstructDid, u32, u32>,
) -> IndexSet<NodeIndex> {
    let mut descendant_nodes = VecDeque::new();
    descendant_nodes.push_front(node);
    let mut descendants = IndexSet::new();
    while let Some(node) = descendant_nodes.pop_front() {
        for (_, child) in graph.children(node).iter(&graph) {
            descendant_nodes.push_back(child);
            descendants.insert(child);
        }
    }
    descendants
}

/// Gets all descendants of `node` within `graph` and returns them, topologically sorted.
/// Legacy, dead code
#[allow(dead_code)]
pub fn get_sorted_descendants_of_node(
    node: NodeIndex,
    graph: Dag<ConstructDid, u32, u32>,
) -> IndexSet<NodeIndex> {
    let sorted = toposort(&graph, None)
        .unwrap()
        .into_iter()
        .collect::<IndexSet<NodeIndex>>();

    let start_node_descendants = get_descendants_of_node(node, graph);
    let mut sorted_descendants = IndexSet::new();

    for this_node in sorted.into_iter() {
        let is_descendant = start_node_descendants.iter().any(|d| d == &this_node);
        let is_start_node = this_node == node;
        if is_descendant || is_start_node {
            sorted_descendants.insert(this_node);
        }
    }
    sorted_descendants
}

/// Returns a topologically sorted set of all nodes in the graph.
pub fn get_sorted_nodes(graph: Dag<ConstructDid, u32, u32>) -> IndexSet<NodeIndex> {
    toposort(&graph, None)
        .unwrap()
        .into_iter()
        .collect::<IndexSet<NodeIndex>>()
}
