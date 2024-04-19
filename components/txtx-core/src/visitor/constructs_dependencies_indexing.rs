use std::sync::{Arc, RwLock};

use txtx_addon_kit::types::{ConstructUuid, PackageUuid};

use crate::types::{Runbook, RuntimeContext};

pub fn run_constructs_dependencies_indexing(
    runbook: &Arc<RwLock<Runbook>>,
    runtime_ctx: &Arc<RwLock<RuntimeContext>>,
) -> Result<
    (
        Vec<(ConstructUuid, ConstructUuid)>,
        Vec<(PackageUuid, PackageUuid)>,
    ),
    String,
> {
    let mut constructs_edges = vec![];
    let packages_edges = vec![];

    match runbook.read() {
        Ok(runbook) => {
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
                            constructs_edges
                                .push((construct_uuid.clone(), resolved_construct_uuid));
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
                            constructs_edges
                                .push((construct_uuid.clone(), resolved_construct_uuid));
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
                            constructs_edges
                                .push((construct_uuid.clone(), resolved_construct_uuid));
                        } else {
                            println!("  -> {} (unable to resolve)", dep,);
                        }
                    }
                }
                for construct_uuid in package.addons_uuids.iter() {
                    let command_instance = runbook.commands_instances.get(construct_uuid).unwrap();
                    for dep in command_instance.collect_dependencies().iter() {
                        let result = runbook.try_resolve_construct_reference_in_expression(
                            package_uuid,
                            dep,
                            runtime_ctx,
                        );
                        if let Ok(Some((resolved_construct_uuid, _))) = result {
                            constructs_edges
                                .push((construct_uuid.clone(), resolved_construct_uuid));
                        } else {
                            println!("  -> {} (unable to resolve)", dep,);
                        }
                    }
                }
            }
        }
        Err(e) => unimplemented!("could not acquire lock: {e}"),
    }
    match runbook.write() {
        Ok(mut runbook) => {
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
        }
        Err(e) => unimplemented!("could not acquire lock: {e}"),
    }

    Ok((constructs_edges, packages_edges))
}
