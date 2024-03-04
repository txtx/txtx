use std::sync::RwLock;

use txtx_addon_kit::types::{ConstructUuid, PackageUuid};

use crate::types::{Manual, RuntimeContext};

pub fn run_constructs_dependencies_indexing(
    manual: &RwLock<Manual>,
    runtime_ctx: &RwLock<RuntimeContext>,
) -> Result<
    (
        Vec<(ConstructUuid, ConstructUuid)>,
        Vec<(PackageUuid, PackageUuid)>,
    ),
    String,
> {
    let mut constructs_edges = vec![];
    let packages_edges = vec![];

    match manual.read() {
        Ok(manual) => {
            let packages = manual.packages.clone();

            for (package_uuid, package) in packages.iter() {
                for construct_uuid in package.imports_uuids.iter() {
                    let construct = manual.commands_instances.get(construct_uuid).unwrap();
                    for dep in construct.collect_dependencies().iter() {}
                }
                for construct_uuid in package.variables_uuids.iter() {
                    let construct = manual.commands_instances.get(construct_uuid).unwrap();
                    for dep in construct.collect_dependencies().iter() {
                        let result = manual.try_resolve_construct_reference_in_expression(
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
                    let construct = manual.commands_instances.get(construct_uuid).unwrap();
                    for dep in construct.collect_dependencies().iter() {
                        let result = manual.try_resolve_construct_reference_in_expression(
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
                    let construct = manual.commands_instances.get(construct_uuid).unwrap();
                    for dep in construct.collect_dependencies().iter() {
                        let result = manual.try_resolve_construct_reference_in_expression(
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
            }
        }
        Err(e) => unimplemented!("could not acquire lock: {e}"),
    }
    match manual.write() {
        Ok(mut manual) => {
            for (src, dst) in constructs_edges.iter() {
                let constructs_graph_nodes = manual.constructs_graph_nodes.clone();

                let src_node_index = constructs_graph_nodes.get(&src.value()).unwrap();
                let dst_node_index = constructs_graph_nodes.get(&dst.value()).unwrap();

                if let Some(edge_to_root) = manual
                    .constructs_graph
                    .find_edge(manual.graph_root, src_node_index.clone())
                {
                    manual.constructs_graph.remove_edge(edge_to_root);
                }
                manual
                    .constructs_graph
                    .add_edge(dst_node_index.clone(), src_node_index.clone(), 1)
                    .unwrap();
            }
        }
        Err(e) => unimplemented!("could not acquire lock: {e}"),
    }

    Ok((constructs_edges, packages_edges))
}
