use txtx_addon_kit::types::{ConstructUuid, PackageUuid};

use crate::{types::Manual, AddonsContext};

pub fn run_constructs_dependencies_indexing(
    manual: &mut Manual,
    _addons_ctx: &mut AddonsContext,
) -> Result<
    (
        Vec<(ConstructUuid, ConstructUuid)>,
        Vec<(PackageUuid, PackageUuid)>,
    ),
    String,
> {
    let mut constructs_edges = vec![];
    let mut packages_edges = vec![];

    for (package_uuid, package) in manual.packages.iter() {
        for construct_uuid in package.imports_uuids.iter() {
            let construct = manual.commands_instances.get(construct_uuid).unwrap();
            for dep in construct.collect_dependencies().iter() {}
        }
        for construct_uuid in package.variables_uuids.iter() {
            let construct = manual.commands_instances.get(construct_uuid).unwrap();
            for dep in construct.collect_dependencies().iter() {
                let result =
                    manual.try_resolve_construct_reference_in_expression(package_uuid, dep);
                if let Ok(Some((resolved_construct_uuid, _))) = result {
                    constructs_edges.push((construct_uuid.clone(), resolved_construct_uuid));
                } else {
                    println!("  -> {} (unable to resolve)", dep,);
                }
            }
        }
        for construct_uuid in package.modules_uuids.iter() {
            let construct = manual.commands_instances.get(construct_uuid).unwrap();
            for dep in construct.collect_dependencies().iter() {
                let result =
                    manual.try_resolve_construct_reference_in_expression(package_uuid, dep);
                if let Ok(Some((resolved_construct_uuid, _))) = result {
                    constructs_edges.push((construct_uuid.clone(), resolved_construct_uuid));
                } else {
                    println!("  -> {} (unable to resolve)", dep,);
                }
            }
        }
        for construct_uuid in package.outputs_uuids.iter() {
            let construct = manual.commands_instances.get(construct_uuid).unwrap();
            for dep in construct.collect_dependencies().iter() {
                let result =
                    manual.try_resolve_construct_reference_in_expression(package_uuid, dep);
                if let Ok(Some((resolved_construct_uuid, _))) = result {
                    constructs_edges.push((construct_uuid.clone(), resolved_construct_uuid));
                } else {
                    println!("  -> {} (unable to resolve)", dep,);
                }
            }
        }
    }

    for (src, dst) in constructs_edges.iter() {
        let src_node_index = manual.constructs_graph_nodes.get(&src.value()).unwrap();
        let dst_node_index = manual.constructs_graph_nodes.get(&dst.value()).unwrap();
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

    Ok((constructs_edges, packages_edges))
}
