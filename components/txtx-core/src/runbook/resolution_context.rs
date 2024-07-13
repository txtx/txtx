use daggy::{Dag, NodeIndex};
use kit::types::ConstructDid;
use kit::types::ConstructId;
use kit::types::Did;
use kit::types::PackageDid;
use kit::types::PackageId;
use std::collections::HashMap;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct Construct {
    /// Id of the Construct
    pub construct_id: ConstructId,
}

#[derive(Debug, Clone)]
pub struct RunbookResolutionContext {
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

impl RunbookResolutionContext {
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
}
