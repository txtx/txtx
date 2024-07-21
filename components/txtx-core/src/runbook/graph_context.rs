use daggy::Walker;
use daggy::{Dag, NodeIndex};
use kit::indexmap::IndexSet;
use kit::types::diagnostics::Diagnostic;
use kit::types::ConstructDid;
use kit::types::ConstructId;
use kit::types::Did;
use kit::types::PackageDid;
use kit::types::PackageId;
use petgraph::algo::toposort;
use std::collections::VecDeque;
use std::collections::{HashMap, HashSet};

use super::{RunbookExecutionContext, RunbookWorkspaceContext};

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

    pub fn build(
        &mut self,
        execution_context: &mut RunbookExecutionContext,
        workspace_context: &RunbookWorkspaceContext,
    ) -> Result<(), Vec<Diagnostic>> {
        // let environment_variables = &runtime_context.get_active_environment_variables();
        // runbook.index_environment_variables(environment_variables);

        let mut constructs_edges = vec![];
        // let packages_edges = vec![];
        let mut diags = vec![];

        let packages = workspace_context.packages.clone();

        for (package_did, package) in packages.iter() {
            for construct_did in package.imports_dids.iter() {
                let construct = execution_context
                    .commands_instances
                    .get(construct_did)
                    .unwrap();
                for _dep in construct.collect_dependencies().iter() {} // todo
            }
            for construct_did in package.variables_dids.iter() {
                let construct = execution_context
                    .commands_instances
                    .get(construct_did)
                    .unwrap();
                for (_input, dep) in construct.collect_dependencies().iter() {
                    let result = workspace_context
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
                let construct = execution_context
                    .commands_instances
                    .get(construct_did)
                    .unwrap();
                for (_input, dep) in construct.collect_dependencies().iter() {
                    let result = workspace_context
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
                let construct = execution_context
                    .commands_instances
                    .get(construct_did)
                    .unwrap();
                for (_input, dep) in construct.collect_dependencies().iter() {
                    let result = workspace_context
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
                let command_instance = execution_context
                    .commands_instances
                    .get(construct_did)
                    .unwrap();
                for (_input, dep) in command_instance.collect_dependencies().iter() {
                    let result = workspace_context
                        .try_resolve_construct_reference_in_expression(package_did, dep);
                    if let Ok(Some((resolved_construct_did, _, _))) = result {
                        if let Some(_) = execution_context
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
                let wallet_instance = execution_context
                    .signing_commands_instances
                    .get(construct_did)
                    .unwrap();
                for (_input, dep) in wallet_instance.collect_dependencies().iter() {
                    let result = workspace_context
                        .try_resolve_construct_reference_in_expression(package_did, dep);
                    if let Ok(Some((resolved_construct_did, _, _))) = result {
                        if !instantiated_wallets.contains(&resolved_construct_did) {
                            wallets.push_front((resolved_construct_did.clone(), false))
                        }
                        execution_context
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
            self.instantiated_signing_commands = wallets;
        }

        for (src, dst) in constructs_edges.iter() {
            let constructs_graph_nodes = self.constructs_dag_node_lookup.clone();

            let src_node_index = constructs_graph_nodes.get(&src).unwrap();
            let dst_node_index = constructs_graph_nodes.get(&dst).unwrap();

            if let Some(edge_to_root) = self
                .constructs_dag
                .find_edge(self.graph_root, src_node_index.clone())
            {
                self.constructs_dag.remove_edge(edge_to_root);
            }
            self.constructs_dag
                .add_edge(dst_node_index.clone(), src_node_index.clone(), 1)
                .unwrap();
        }

        if !diags.is_empty() {
            return Err(diags);
        }

        for (construct_did, instantiated) in self.instantiated_signing_commands.iter() {
            execution_context
                .order_for_signing_commands_initialization
                .push(construct_did.clone());
            // For each signing command instantiated
            if *instantiated {
                // We retrieve the downstream dependencies (signed commands)
                let unordered_signed_commands =
                    self.get_downstream_dependencies_for_construct_did(construct_did, false);
                let mut ordered_signed_commands = vec![];

                for signed_dependency in unordered_signed_commands.iter() {
                    // For each signed commands, we retrieve the upstream dependencies, but:
                    // - we ignore the signing commands
                    // - we pop the last root synthetic ConstructDid
                    let mut upstream_dependencies = self
                        .get_upstream_dependencies_for_construct_did(signed_dependency)
                        .into_iter()
                        .filter(|c| !c.eq(&construct_did))
                        .collect::<Vec<_>>();
                    upstream_dependencies.remove(upstream_dependencies.len() - 1);

                    ordered_signed_commands
                        .push((signed_dependency.clone(), upstream_dependencies.len()));

                    execution_context
                        .signed_commands_upstream_dependencies
                        .insert(signed_dependency.clone(), upstream_dependencies);
                    execution_context
                        .signed_commands
                        .insert(signed_dependency.clone());
                }
                ordered_signed_commands.sort_by(|(a_id, a_len), (b_id, b_len)| {
                    if a_len.eq(b_len) {
                        a_id.cmp(b_id)
                    } else {
                        a_len.cmp(&b_len)
                    }
                });

                execution_context
                    .signing_commands_downstream_dependencies
                    .push((
                        construct_did.clone(),
                        ordered_signed_commands
                            .into_iter()
                            .map(|(construct_id, _)| construct_id)
                            .collect(),
                    ));
            }
        }

        for construct_did in self.get_sorted_constructs() {
            execution_context
                .order_for_commands_execution
                .push(construct_did.clone());
        }

        for (construct_did, _) in execution_context.commands_instances.iter() {
            let dependencies =
                self.get_downstream_dependencies_for_construct_did(construct_did, true);
            execution_context
                .commands_dependencies
                .insert(construct_did.clone(), dependencies);
        }
        Ok(())
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
    pub fn get_nodes_descending_from_node(
        &self,
        node: NodeIndex,
        recursive: bool,
    ) -> IndexSet<NodeIndex> {
        let mut descendant_nodes = VecDeque::new();
        descendant_nodes.push_front(node);
        let mut descendants = IndexSet::new();
        while let Some(node) = descendant_nodes.pop_front() {
            for (_, child) in self
                .constructs_dag
                .children(node)
                .iter(&self.constructs_dag)
            {
                if recursive {
                    descendant_nodes.push_back(child);
                }
                descendants.insert(child);
            }
        }
        descendants
    }

    /// Gets all descendants of `node` within `graph`.
    pub fn get_downstream_dependencies_for_construct_did(
        &self,
        construct_did: &ConstructDid,
        recursive: bool,
    ) -> Vec<ConstructDid> {
        let node_index = self
            .constructs_dag_node_lookup
            .get(construct_did)
            .expect("construct_did not indexed in graph");
        let nodes = self.get_nodes_descending_from_node(node_index.clone(), recursive);
        self.resolve_constructs_dids(nodes)
    }

    /// Gets all descendants of `node` within `graph` and returns them, topologically sorted.
    /// Legacy, dead code
    #[allow(dead_code)]
    pub fn get_sorted_descendants_of_node(
        &self,
        node: NodeIndex,
        recursive: bool,
    ) -> Vec<ConstructDid> {
        let sorted = toposort(&self.constructs_dag, None)
            .unwrap()
            .into_iter()
            .collect::<IndexSet<NodeIndex>>();

        let start_node_descendants = self.get_nodes_descending_from_node(node, recursive);
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
