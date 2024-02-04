use crate::{types::{ConstructUuid, Manual, PackageUuid}, AddonsContext};

pub fn run_edge_indexer(
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
            let construct = manual.constructs.get(construct_uuid).unwrap();
            for dep in construct.collect_dependencies().iter() {
                println!("  -> {}", dep);
            }
        }
        for construct_uuid in package.variables_uuids.iter() {
            let construct = manual.constructs.get(construct_uuid).unwrap();
            for dep in construct.collect_dependencies().iter() {
                let result = manual.resolve_construct_reference(package_uuid, dep);
                if let Ok(Some(resolved_construct_uuid)) = result {
                    println!(
                        "  -> {} resolving to {}",
                        dep,
                        resolved_construct_uuid.value()
                    );
                } else {
                    println!("  -> {} (unable to resolve)", dep,);
                }
            }
        }
        for construct_uuid in package.modules_uuids.iter() {
            let construct = manual.constructs.get(construct_uuid).unwrap();
            for dep in construct.collect_dependencies().iter() {
                let result = manual.resolve_construct_reference(package_uuid, dep);
                if let Ok(Some(resolved_construct_uuid)) = result {
                    println!(
                        "  -> {} resolving to {}",
                        dep,
                        resolved_construct_uuid.value()
                    );
                } else {
                    println!("  -> {} (unable to resolve)", dep,);
                }
            }
        }
        for construct_uuid in package.outputs_uuids.iter() {
            let construct = manual.constructs.get(construct_uuid).unwrap();
            for dep in construct.collect_dependencies().iter() {
                let result = manual.resolve_construct_reference(package_uuid, dep);
                if let Ok(Some(resolved_construct_uuid)) = result {
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
        manual.constructs_graph.add_edge(src_node_index.clone(), dst_node_index.clone(), 1).unwrap();
    }

    Ok((constructs_edges, packages_edges))
}
