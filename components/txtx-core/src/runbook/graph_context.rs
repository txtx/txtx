use daggy::Walker;
use daggy::{Dag, NodeIndex};
use indexmap::IndexSet;
use kit::types::ConstructDid;
use kit::types::ConstructId;
use kit::types::Did;
use kit::types::PackageDid;
use kit::types::PackageId;
use petgraph::algo::toposort;
use std::collections::HashMap;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct Construct {
    /// Id of the Construct
    pub construct_id: ConstructId,
}

#[derive(Debug, Clone)]
pub struct RunbookGraphContext {
    /// Direct Acyclic Graph keeping track of the dependencies between packages
    pub packages_dag: Dag<PackageDid, u32, u32>,
    /// Lookup: Retrieve the DAG node id of a given package did
    pub packages_dag_node_lookup: HashMap<PackageDid, NodeIndex<u32>>,
    /// Direct Acyclic Graph keeping track of the dependencies between constructs
    pub constructs_dag: Dag<ConstructDid, u32, u32>,
    /// Lookup: Retrieve the DAG node id of a given construct did
    pub constructs_dag_node_lookup: HashMap<ConstructDid, NodeIndex<u32>>,
    /// Keep track of the signing commands (wallet) instantiated (ordered)
    pub instantiated_signing_commands: VecDeque<(ConstructDid, bool)>,
    /// Keep track of the root DAGs (temporary - to be removed)
    pub graph_root: NodeIndex<u32>,
}

impl RunbookGraphContext {
    pub fn new() -> Self {
        // Initialize DAGs
        let mut packages_dag = Dag::new();
        let _ = packages_dag.add_node(PackageDid(Did::zero()));
        let mut constructs_dag = Dag::new();
        let graph_root = constructs_dag.add_node(ConstructDid(Did::zero()));

        Self {
            packages_dag,
            packages_dag_node_lookup: HashMap::new(),
            constructs_dag,
            constructs_dag_node_lookup: HashMap::new(),
            instantiated_signing_commands: VecDeque::new(),
            graph_root,
        }
    }

    pub fn index_package(&mut self, package_id: &PackageId) {
        self.packages_dag
            .add_child(self.graph_root, 0, package_id.did());
    }

    pub fn index_construct(&mut self, construct_did: &ConstructDid) {
        let (_, node_index) =
            self.constructs_dag
                .add_child(self.graph_root.clone(), 100, construct_did.clone());
        self.constructs_dag_node_lookup
            .insert(construct_did.clone(), node_index);
    }

    pub fn index_environment_variable(&mut self, construct_did: &ConstructDid) {
        let (_, node_index) =
            self.constructs_dag
                .add_child(self.graph_root.clone(), 100, construct_did.clone());
        self.constructs_dag_node_lookup
            .insert(construct_did.clone(), node_index);
    }

    /// Gets all descendants of `node` within `graph`.
    pub fn get_nodes_descending_from_node(&self, node: NodeIndex) -> IndexSet<NodeIndex> {
        let mut descendant_nodes = VecDeque::new();
        descendant_nodes.push_front(node);
        let mut descendants = IndexSet::new();
        while let Some(node) = descendant_nodes.pop_front() {
            for (_, child) in self
                .constructs_dag
                .children(node)
                .iter(&self.constructs_dag)
            {
                descendant_nodes.push_back(child);
                descendants.insert(child);
            }
        }
        descendants
    }

    /// Gets all descendants of `node` within `graph`.
    pub fn get_downstream_dependencies_for_construct_did(
        &self,
        construct_did: &ConstructDid,
    ) -> Vec<ConstructDid> {
        let node_index = self
            .constructs_dag_node_lookup
            .get(construct_did)
            .expect("construct_did not indexed in graph");
        let nodes = self.get_nodes_descending_from_node(node_index.clone());
        self.resolve_constructs_dids(nodes)
    }

    /// Gets all descendants of `node` within `graph` and returns them, topologically sorted.
    /// Legacy, dead code
    #[allow(dead_code)]
    pub fn get_sorted_descendants_of_node(&self, node: NodeIndex) -> Vec<ConstructDid> {
        let sorted = toposort(&self.constructs_dag, None)
            .unwrap()
            .into_iter()
            .collect::<IndexSet<NodeIndex>>();

        let start_node_descendants = self.get_nodes_descending_from_node(node);
        let mut sorted_descendants = IndexSet::new();

        for this_node in sorted.into_iter() {
            let is_descendant = start_node_descendants.iter().any(|d| d == &this_node);
            let is_start_node = this_node == node;
            if is_descendant || is_start_node {
                sorted_descendants.insert(this_node);
            }
        }
        self.resolve_constructs_dids(sorted_descendants)
    }

    /// Gets all ascendants of `node` within `graph`.
    pub fn get_nodes_ascending_from_node(&self, node: NodeIndex) -> IndexSet<NodeIndex> {
        let mut ascendants_nodes = VecDeque::new();
        ascendants_nodes.push_front(node);
        let mut ascendants = IndexSet::new();
        while let Some(node) = ascendants_nodes.pop_front() {
            for (_, parent) in self.constructs_dag.parents(node).iter(&self.constructs_dag) {
                ascendants_nodes.push_back(parent);
                ascendants.insert(parent);
            }
        }
        ascendants
    }

    /// Gets all ascendants of `node` within `graph`.
    pub fn get_upstream_dependencies_for_construct_did(
        &self,
        construct_did: &ConstructDid,
    ) -> Vec<ConstructDid> {
        let node_index = self
            .constructs_dag_node_lookup
            .get(construct_did)
            .expect("construct_did not indexed in graph");
        let nodes = self.get_nodes_ascending_from_node(node_index.clone());
        self.resolve_constructs_dids(nodes)
    }

    /// Returns a topologically sorted set of all nodes in the graph.
    pub fn get_sorted_constructs(&self) -> Vec<ConstructDid> {
        let nodes = toposort(&self.constructs_dag, None)
            .unwrap()
            .into_iter()
            .collect::<IndexSet<NodeIndex>>();
        self.resolve_constructs_dids(nodes)
    }

    pub fn resolve_constructs_dids(&self, nodes: IndexSet<NodeIndex>) -> Vec<ConstructDid> {
        let mut construct_dids = vec![];
        for node in nodes {
            let construct_did = self
                .constructs_dag
                .node_weight(node)
                .expect("construct_did not indexed in graph");

            construct_dids.push(construct_did.clone());
        }
        construct_dids
    }
}
