use crate::{types::Manual, AddonsContext};
use daggy::Walker;
use txtx_addon_kit::types::ConstructUuid;

pub fn run(manual: &Manual, _addons_ctx: &AddonsContext) -> Result<(), String> {
    println!("Executing graph");
    let root = manual.graph_root;
    println!("{:?}", manual.constructs_graph);
    // let mut walker = manual.constructs_graph.children(root).iter(&manual.constructs_graph);

    let mut walker = manual
        .constructs_graph
        .recursive_walk(root, |g, n| g.children(n).iter(g).find(|&(e, n)| true));

    while let Some((_edge, node)) = walker.walk_next(&manual.constructs_graph) {
        let uuid = manual
            .constructs_graph
            .node_weight(node)
            .expect("unable to retrieve construct uuid");
        let construct_uuid = ConstructUuid::Local(*uuid);
        if let Some(construct) = manual.constructs.get(&construct_uuid) {
            println!("- {}", construct.get_construct_uri(),);
        }
    }

    Ok(())
}
