use std::collections::{BTreeSet, VecDeque};

use crate::{types::Manual, AddonsContext};
use daggy::{petgraph::data::DataMap, Walker};
use txtx_addon_kit::types::ConstructUuid;

pub fn run(manual: &Manual, _addons_ctx: &AddonsContext) -> Result<(), String> {
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
    for node in visited_nodes_to_process.into_iter() {
        let uuid = g.node_weight(node).expect("unable to retrieve construct");
        let construct_uuid = ConstructUuid::Local(uuid.clone());
        let construct = manual
            .constructs
            .get(&construct_uuid)
            .expect("unable to retrieve construct");
        println!("{}", construct.get_construct_uri());
    }

    Ok(())
}
