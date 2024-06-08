use std::collections::{HashSet, VecDeque};

use txtx_addon_kit::types::{diagnostics::Diagnostic, ConstructUuid, PackageUuid};

use crate::types::{Runbook, RuntimeContext};

pub fn run_constructs_dependencies_indexing(
    runbook: &mut Runbook,
    runtime_ctx: &mut RuntimeContext,
) -> Result<
    (
        Vec<(ConstructUuid, ConstructUuid)>,
        Vec<(PackageUuid, PackageUuid)>,
    ),
    Vec<Diagnostic>,
> {
    runbook.seed_environment_variables(runtime_ctx);

    let mut constructs_edges = vec![];
    let packages_edges = vec![];

    let packages = runbook.packages.clone();

    for (package_uuid, package) in packages.iter() {
        for construct_uuid in package.imports_uuids.iter() {
            let construct = runbook.commands_instances.get(construct_uuid).unwrap();
            for _dep in construct.collect_dependencies().iter() {} // todo
        }
        for construct_uuid in package.variables_uuids.iter() {
            let construct = runbook.commands_instances.get(construct_uuid).unwrap();
            for dep in construct.collect_dependencies().iter() {
                let result = runbook.try_resolve_construct_reference_in_expression(
                    package_uuid,
                    dep,
                    &runtime_ctx,
                );
                if let Ok(Some((resolved_construct_uuid, _))) = result {
                    constructs_edges.push((construct_uuid.clone(), resolved_construct_uuid));
                } else {
                    println!("  -> {} (unable to resolve)", dep,);
                }
            }
        }
        for construct_uuid in package.modules_uuids.iter() {
            let construct = runbook.commands_instances.get(construct_uuid).unwrap();
            for dep in construct.collect_dependencies().iter() {
                let result = runbook.try_resolve_construct_reference_in_expression(
                    package_uuid,
                    dep,
                    &runtime_ctx,
                );
                if let Ok(Some((resolved_construct_uuid, _))) = result {
                    constructs_edges.push((construct_uuid.clone(), resolved_construct_uuid));
                } else {
                    println!("  -> {} (unable to resolve)", dep,);
                }
            }
        }
        for construct_uuid in package.outputs_uuids.iter() {
            let construct = runbook.commands_instances.get(construct_uuid).unwrap();
            for dep in construct.collect_dependencies().iter() {
                let result = runbook.try_resolve_construct_reference_in_expression(
                    package_uuid,
                    dep,
                    &runtime_ctx,
                );
                if let Ok(Some((resolved_construct_uuid, _))) = result {
                    constructs_edges.push((construct_uuid.clone(), resolved_construct_uuid));
                } else {
                    println!("  -> {} (unable to resolve)", dep,);
                }
            }
        }
        let mut wallets = VecDeque::new();
        let mut instantiated_wallets = HashSet::new();
        for construct_uuid in package.addons_uuids.iter() {
            let command_instance = runbook.commands_instances.get(construct_uuid).unwrap();
            for dep in command_instance.collect_dependencies().iter() {
                let result = runbook.try_resolve_construct_reference_in_expression(
                    package_uuid,
                    dep,
                    runtime_ctx,
                );
                if let Ok(Some((resolved_construct_uuid, _))) = result {
                    if let Some(_) = runbook.wallets_instances.get(&resolved_construct_uuid) {
                        wallets.push_front((resolved_construct_uuid.clone(), true));
                        instantiated_wallets.insert(resolved_construct_uuid.clone());
                    }
                    constructs_edges.push((construct_uuid.clone(), resolved_construct_uuid));
                } else {
                    println!("  -> {} (unable to resolve)", dep,);
                }
            }
        }
        // todo: should we constrain to wallets depending on wallets?
        for construct_uuid in package.wallets_uuids.iter() {
            let wallet_instance = runbook.wallets_instances.get(construct_uuid).unwrap();
            for dep in wallet_instance.collect_dependencies().iter() {
                let result = runbook.try_resolve_construct_reference_in_expression(
                    package_uuid,
                    dep,
                    runtime_ctx,
                );
                if let Ok(Some((resolved_construct_uuid, _))) = result {
                    if !instantiated_wallets.contains(&resolved_construct_uuid) {
                        wallets.push_front((resolved_construct_uuid.clone(), false))
                    }
                    runbook.wallets_state.as_mut().unwrap().create_new_wallet(
                        &resolved_construct_uuid,
                        &resolved_construct_uuid.value().to_string(),
                    );
                    constructs_edges.push((construct_uuid.clone(), resolved_construct_uuid));
                } else {
                    println!("  -> {} (unable to resolve)", dep,);
                }
            }
        }
        runbook.instantiated_wallet_instances = wallets;
    }

    for (src, dst) in constructs_edges.iter() {
        let constructs_graph_nodes = runbook.constructs_graph_nodes.clone();

        let src_node_index = constructs_graph_nodes.get(&src.value()).unwrap();
        let dst_node_index = constructs_graph_nodes.get(&dst.value()).unwrap();

        if let Some(edge_to_root) = runbook
            .constructs_graph
            .find_edge(runbook.graph_root, src_node_index.clone())
        {
            runbook.constructs_graph.remove_edge(edge_to_root);
        }
        runbook
            .constructs_graph
            .add_edge(dst_node_index.clone(), src_node_index.clone(), 1)
            .unwrap();
    }

    Ok((constructs_edges, packages_edges))
}
