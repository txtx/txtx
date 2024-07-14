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
    let environment_variables = &runtime_ctx.get_active_environment_variables();
    runbook.index_environment_variables(environment_variables);

    let mut constructs_edges = vec![];
    let packages_edges = vec![];
    let mut diags = vec![];

    let packages = runbook.workspace_context.packages.clone();

    for (package_did, package) in packages.iter() {
        for construct_did in package.imports_dids.iter() {
            let construct = runbook
                .execution_context
                .commands_instances
                .get(construct_did)
                .unwrap();
            for _dep in construct.collect_dependencies().iter() {} // todo
        }
        for construct_did in package.variables_dids.iter() {
            let construct = runbook
                .execution_context
                .commands_instances
                .get(construct_did)
                .unwrap();
            for (_input, dep) in construct.collect_dependencies().iter() {
                let result = runbook
                    .workspace_context
                    .try_resolve_construct_reference_in_expression(package_did, dep);
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
        for construct_did in package.modules_dids.iter() {
            let construct = runbook
                .execution_context
                .commands_instances
                .get(construct_did)
                .unwrap();
            for (_input, dep) in construct.collect_dependencies().iter() {
                let result = runbook
                    .workspace_context
                    .try_resolve_construct_reference_in_expression(package_did, dep);
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
        for construct_did in package.outputs_dids.iter() {
            let construct = runbook
                .execution_context
                .commands_instances
                .get(construct_did)
                .unwrap();
            for (_input, dep) in construct.collect_dependencies().iter() {
                let result = runbook
                    .workspace_context
                    .try_resolve_construct_reference_in_expression(package_did, dep);
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
        for construct_did in package.addons_dids.iter() {
            let command_instance = runbook
                .execution_context
                .commands_instances
                .get(construct_did)
                .unwrap();
            for (_input, dep) in command_instance.collect_dependencies().iter() {
                let result = runbook
                    .workspace_context
                    .try_resolve_construct_reference_in_expression(package_did, dep);
                if let Ok(Some((resolved_construct_did, _, _))) = result {
                    if let Some(_) = runbook
                        .execution_context
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
        for construct_did in package.signing_commands_dids.iter() {
            let wallet_instance = runbook
                .execution_context
                .signing_commands_instances
                .get(construct_did)
                .unwrap();
            for (_input, dep) in wallet_instance.collect_dependencies().iter() {
                let result = runbook
                    .workspace_context
                    .try_resolve_construct_reference_in_expression(package_did, dep);
                if let Ok(Some((resolved_construct_did, _, _))) = result {
                    if !instantiated_wallets.contains(&resolved_construct_did) {
                        wallets.push_front((resolved_construct_did.clone(), false))
                    }
                    runbook
                        .execution_context
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
        runbook.graph_context.instantiated_signing_commands = wallets;
    }

    for (src, dst) in constructs_edges.iter() {
        let constructs_graph_nodes = runbook
            .graph_context
            .constructs_dag_node_lookup
            .clone();

        let src_node_index = constructs_graph_nodes.get(&src).unwrap();
        let dst_node_index = constructs_graph_nodes.get(&dst).unwrap();

        if let Some(edge_to_root) = runbook.graph_context.constructs_dag.find_edge(
            runbook.graph_context.graph_root,
            src_node_index.clone(),
        ) {
            runbook
                .graph_context
                .constructs_dag
                .remove_edge(edge_to_root);
        }
        runbook
            .graph_context
            .constructs_dag
            .add_edge(dst_node_index.clone(), src_node_index.clone(), 1)
            .unwrap();
    }

    if diags.is_empty() {
        for (construct_did, instantiated) in runbook
            .graph_context
            .instantiated_signing_commands
            .iter()
        {
            runbook
                .execution_context
                .order_for_signing_commands_initialization
                .push(construct_did.clone());

            if *instantiated {
                let mut dependencies = vec![];
                let node_index = runbook
                    .graph_context
                    .constructs_dag_node_lookup
                    .get(construct_did)
                    .expect("construct_did not indexed in graph");
                for dependent_construct_did in runbook
                    .graph_context
                    .get_constructs_ids_descending_from_node(node_index.clone())
                    .into_iter()
                {
                    dependencies.push(dependent_construct_did.clone());
                }
                runbook
                    .execution_context
                    .signing_commands_dependencies
                    .insert(construct_did.clone(), dependencies);
            }
        }

        for construct_did in runbook.graph_context.get_sorted_constructs() {
            runbook
                .execution_context
                .order_for_commands_execution
                .push(construct_did.clone());
        }

        for (construct_did, _) in runbook.execution_context.commands_instances.iter() {
            let mut dependencies = vec![];
            let node_index = runbook
                .graph_context
                .constructs_dag_node_lookup
                .get(construct_did)
                .expect("construct_did not indexed in graph");
            for dependent_construct_did in runbook
                .graph_context
                .get_constructs_ids_descending_from_node(node_index.clone())
                .into_iter()
            {
                dependencies.push(dependent_construct_did.clone());
            }
            runbook
                .execution_context
                .commands_dependencies
                .insert(construct_did.clone(), dependencies);
        }
        return Ok((constructs_edges, packages_edges));
    }

    Err(diags)
}
