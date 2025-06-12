use daggy::Walker;
use daggy::{Dag, NodeIndex};
use kit::types::commands::ConstructInstance;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};
use std::collections::{HashMap, HashSet};
use txtx_addon_kit::hcl::Span;
use txtx_addon_kit::indexmap::IndexSet;
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::types::Did;
use txtx_addon_kit::types::PackageDid;
use txtx_addon_kit::types::PackageId;

use super::{RunbookExecutionContext, RunbookWorkspaceContext};

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
    /// Keep track of the signing commands (signer) instantiated (ordered)
    pub instantiated_signers: VecDeque<(ConstructDid, bool)>,
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
            instantiated_signers: VecDeque::new(),
            graph_root,
        }
    }

    pub fn build(
        &mut self,
        execution_context: &mut RunbookExecutionContext,
        workspace_context: &RunbookWorkspaceContext,
        domain_specific_dependencies: HashMap<ConstructDid, Vec<ConstructDid>>,
    ) -> Result<(), Vec<Diagnostic>> {
        let mut constructs_edges = vec![];

        let mut diags = vec![];

        let packages = workspace_context.packages.clone();

        for (package_id, package) in packages.iter() {
            // add variable constructs to graph
            for construct_did in package.variables_dids.iter() {
                let command_instance =
                    execution_context.commands_instances.get(construct_did).unwrap();
                let construct_id = workspace_context.constructs.get(construct_did).unwrap();

                for (_input, dep) in
                    command_instance.get_expressions_referencing_commands_from_inputs().iter()
                {
                    let result = workspace_context
                        .try_resolve_construct_reference_in_expression(package_id, dep);
                    if let Ok(Some((resolved_construct_did, _, _))) = result {
                        constructs_edges.push((construct_did.clone(), resolved_construct_did));
                    } else {
                        diags.push(
                            diagnosed_error!(
                                "unable to resolve '{}' in input '{}'",
                                dep.to_string().trim(),
                                command_instance.name,
                            )
                            .location(&construct_id.construct_location)
                            .set_span_range(command_instance.block.span()),
                        );
                    }
                }
            }
            // add module constructs to graph
            for construct_did in package.modules_dids.iter() {
                let command_instance =
                    execution_context.commands_instances.get(construct_did).unwrap();
                let construct_id = workspace_context.constructs.get(construct_did).unwrap();

                for (_input, dep) in
                    command_instance.get_expressions_referencing_commands_from_inputs().iter()
                {
                    let result = workspace_context
                        .try_resolve_construct_reference_in_expression(package_id, dep);
                    if let Ok(Some((resolved_construct_did, _, _))) = result {
                        constructs_edges.push((construct_did.clone(), resolved_construct_did));
                    } else {
                        diags.push(
                            diagnosed_error!(
                                "unable to resolve '{}' in module '{}'",
                                dep.to_string().trim(),
                                command_instance.name,
                            )
                            .location(&construct_id.construct_location)
                            .set_span_range(command_instance.block.span()),
                        );
                    }
                }
            }
            // add output constructs to graph
            for construct_did in package.outputs_dids.iter() {
                let command_instance =
                    execution_context.commands_instances.get(construct_did).unwrap();
                let construct_id = workspace_context.constructs.get(construct_did).unwrap();

                for (_input, dep) in
                    command_instance.get_expressions_referencing_commands_from_inputs().iter()
                {
                    let result = workspace_context
                        .try_resolve_construct_reference_in_expression(package_id, dep);
                    if let Ok(Some((resolved_construct_did, _, _))) = result {
                        constructs_edges.push((construct_did.clone(), resolved_construct_did));
                    } else {
                        diags.push(
                            diagnosed_error!(
                                "unable to resolve '{}' in output '{}'",
                                dep.to_string().trim(),
                                command_instance.name,
                            )
                            .location(&construct_id.construct_location)
                            .set_span_range(command_instance.block.span()),
                        );
                    }
                }
            }
            let mut signers = VecDeque::new();
            let mut instantiated_signers = HashSet::new();
            // add command constructs to graph
            for construct_did in package.commands_dids.iter() {
                let command_instance =
                    execution_context.commands_instances.get(construct_did).unwrap();

                let construct_id = workspace_context.constructs.get(construct_did).unwrap();

                if let Some(dependencies) = domain_specific_dependencies.get(construct_did) {
                    for dependent_construct_did in dependencies {
                        constructs_edges
                            .push((construct_did.clone(), dependent_construct_did.clone()));
                    }
                }

                for (_input, dep) in
                    command_instance.get_expressions_referencing_commands_from_inputs().iter()
                {
                    let result = workspace_context
                        .try_resolve_construct_reference_in_expression(package_id, dep);
                    if let Ok(Some((resolved_construct_did, _, _))) = result {
                        if let Some(_) =
                            execution_context.signers_instances.get(&resolved_construct_did)
                        {
                            signers.push_front((resolved_construct_did.clone(), true));
                            instantiated_signers.insert(resolved_construct_did.clone());
                        }
                        constructs_edges.push((construct_did.clone(), resolved_construct_did));
                    } else {
                        diags.push(
                            diagnosed_error!(
                                "unable to resolve '{}' in action '{}'",
                                dep.to_string().trim(),
                                command_instance.name,
                            )
                            .location(&construct_id.construct_location)
                            .set_span_range(command_instance.block.span()),
                        );
                    }
                }
            }

            // add embedded runbook constructs to graph
            for construct_did in package.embedded_runbooks_dids.iter() {
                let embedded_runbook_instance =
                    execution_context.embedded_runbooks.get(construct_did).unwrap();

                let construct_id = workspace_context.constructs.get(construct_did).unwrap();

                if let Some(dependencies) = domain_specific_dependencies.get(construct_did) {
                    for dependent_construct_did in dependencies {
                        constructs_edges
                            .push((construct_did.clone(), dependent_construct_did.clone()));
                    }
                }

                for (_input, dep) in embedded_runbook_instance
                    .get_expressions_referencing_commands_from_inputs()
                    .iter()
                {
                    let result = workspace_context
                        .try_resolve_construct_reference_in_expression(package_id, dep);
                    if let Ok(Some((resolved_construct_did, _, _))) = result {
                        if let Some(_) =
                            execution_context.signers_instances.get(&resolved_construct_did)
                        {
                            signers.push_front((resolved_construct_did.clone(), true));
                            instantiated_signers.insert(resolved_construct_did.clone());
                        }
                        constructs_edges.push((construct_did.clone(), resolved_construct_did));
                    } else {
                        diags.push(
                            diagnosed_error!(
                                "unable to resolve '{}' in embedded runbook '{}'",
                                dep.to_string().trim(),
                                embedded_runbook_instance.name,
                            )
                            .location(&construct_id.construct_location)
                            .set_span_range(embedded_runbook_instance.block.span()),
                        );
                    }
                }
            }
            // todo: should we constrain to signers depending on signers?
            // add signer constructs to graph
            for construct_did in package.signers_dids.iter() {
                let signer_instance =
                    execution_context.signers_instances.get(construct_did).unwrap();
                let construct_id = workspace_context.constructs.get(construct_did).unwrap();

                for (_input, dep) in
                    signer_instance.get_expressions_referencing_commands_from_inputs().iter()
                {
                    let result = workspace_context
                        .try_resolve_construct_reference_in_expression(package_id, dep);
                    if let Ok(Some((resolved_construct_did, _, _))) = result {
                        if !instantiated_signers.contains(&resolved_construct_did) {
                            signers.push_front((resolved_construct_did.clone(), false))
                        }
                        execution_context.signers_state.as_mut().unwrap().create_new_signer(
                            &resolved_construct_did,
                            &resolved_construct_did.value().to_string(),
                        );
                        constructs_edges.push((construct_did.clone(), resolved_construct_did));
                    } else {
                        diags.push(
                            diagnosed_error!(
                                "unable to resolve '{}' in signer '{}'",
                                dep.to_string().trim(),
                                signer_instance.name,
                            )
                            .location(&construct_id.construct_location)
                            .set_span_range(signer_instance.block.span()),
                        );
                    }
                }
            }
            // this is the most idiomatic way I could find to get unique values from a hash set
            let mut seen_signers = HashSet::new();
            signers.retain(|w| seen_signers.insert(w.clone()));
            self.instantiated_signers = signers;
        }

        for (src, dst) in constructs_edges.iter() {
            let constructs_graph_nodes = self.constructs_dag_node_lookup.clone();

            let src_node_index = constructs_graph_nodes.get(&src).unwrap();
            let dst_node_index = constructs_graph_nodes.get(&dst).unwrap();

            if let Some(edge_to_root) =
                self.constructs_dag.find_edge(self.graph_root, src_node_index.clone())
            {
                self.constructs_dag.remove_edge(edge_to_root);
            }
            if dst_node_index == src_node_index {
                continue;
            }
            if let Err(_e) =
                self.constructs_dag.add_edge(dst_node_index.clone(), src_node_index.clone(), 1)
            {
                diags.push(diagnosed_error!("Cycling dependency"));
            }
        }

        if !diags.is_empty() {
            return Err(diags);
        }

        for (signer_did, instantiated) in self.instantiated_signers.iter() {
            execution_context.order_for_signers_initialization.push(signer_did.clone());
            // For each signing command instantiated
            if *instantiated {
                // We retrieve the downstream dependencies (signed commands)
                let unordered_signed_commands =
                    self.get_downstream_dependencies_for_construct_did(signer_did, false);
                let mut ordered_signed_commands = vec![];

                for signed_dependency in unordered_signed_commands.iter() {
                    // For each signed commands, we retrieve the upstream dependencies, but:
                    // - we ignore the signing commands
                    // - we pop the last root synthetic ConstructDid
                    let mut upstream_dependencies = self
                        .get_upstream_dependencies_for_construct_did(signed_dependency)
                        .into_iter()
                        .filter(|c| {
                            !self
                                .instantiated_signers
                                .iter()
                                .any(|(signer_did, _)| c.eq(signer_did))
                        })
                        .collect::<Vec<_>>();
                    upstream_dependencies.remove(upstream_dependencies.len() - 1);

                    ordered_signed_commands
                        .push((signed_dependency.clone(), upstream_dependencies.len()));

                    execution_context
                        .signed_commands_upstream_dependencies
                        .insert(signed_dependency.clone(), upstream_dependencies);
                    execution_context.signed_commands.insert(signed_dependency.clone());
                }

                ordered_signed_commands.sort_by(|(a_id, a_len), (b_id, b_len)| {
                    if a_len.eq(b_len) {
                        a_id.cmp(b_id)
                    } else {
                        a_len.cmp(&b_len)
                    }
                });

                execution_context.signers_downstream_dependencies.push((
                    signer_did.clone(),
                    ordered_signed_commands
                        .into_iter()
                        .map(|(construct_id, _)| construct_id)
                        .collect(),
                ));
            }
        }

        for construct_did in self.get_sorted_constructs() {
            execution_context.order_for_commands_execution.push(construct_did.clone());
        }

        for construct_did in execution_context
            .commands_instances
            .keys()
            .chain(execution_context.embedded_runbooks.keys())
        {
            let dependencies =
                self.get_downstream_dependencies_for_construct_did(construct_did, true);
            execution_context.commands_dependencies.insert(construct_did.clone(), dependencies);
        }
        Ok(())
    }

    pub fn index_package(&mut self, package_id: &PackageId) {
        self.packages_dag.add_child(self.graph_root, 0, package_id.did());
    }

    pub fn index_construct(&mut self, construct_did: &ConstructDid) {
        let (_, node_index) =
            self.constructs_dag.add_child(self.graph_root.clone(), 100, construct_did.clone());
        self.constructs_dag_node_lookup.insert(construct_did.clone(), node_index);
    }

    /// Adds the provided [ConstructDid] to the `constructs_dag` of the [RunbookGraphContext].
    /// Then, the construct's [NodeIndex] is added to the `constructs_dag_node_lookup`.
    pub fn index_top_level_input(&mut self, construct_did: &ConstructDid) {
        let (_, node_index) =
            self.constructs_dag.add_child(self.graph_root.clone(), 100, construct_did.clone());
        self.constructs_dag_node_lookup.insert(construct_did.clone(), node_index);
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
            for (_, child) in self.constructs_dag.children(node).iter(&self.constructs_dag) {
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
        let nodes = stable_kahn_toposort(&self.constructs_dag);
        self.resolve_constructs_dids(nodes)
    }

    pub fn resolve_constructs_dids(&self, nodes: IndexSet<NodeIndex>) -> Vec<ConstructDid> {
        let mut construct_dids = vec![];
        for node in nodes {
            let construct_did =
                self.constructs_dag.node_weight(node).expect("construct_did not indexed in graph");

            construct_dids.push(construct_did.clone());
        }
        construct_dids
    }
}

/// Stable topological sort using Kahn's algorithm
/// This implementation prioritizes the original order of nodes in the graph
fn stable_kahn_toposort(dag: &Dag<ConstructDid, u32>) -> IndexSet<NodeIndex> {
    let graph = dag.graph();
    // Map nodes to their original positions for stable sorting
    let index_map: HashMap<NodeIndex, usize> =
        graph.clone().node_indices().enumerate().map(|(i, node)| (node, i)).collect();

    // Track the in-degree of each node
    let mut in_degree: HashMap<NodeIndex, usize> = HashMap::new();
    let mut queue: BinaryHeap<Reverse<(usize, NodeIndex)>> = BinaryHeap::new();

    // Initialize in-degrees and enqueue nodes with zero in-degree
    for node in graph.node_indices() {
        let degree = graph.edges_directed(node, petgraph::Incoming).count();
        in_degree.insert(node, degree);
        if degree == 0 {
            // Insert node into queue with priority based on original order
            queue.push(Reverse((index_map[&node], node)));
        }
    }

    let mut sorted = Vec::new();

    // Process nodes in topological order, prioritizing original order for equal dependencies
    while let Some(Reverse((_, node))) = queue.pop() {
        // Add the node to the sorted output
        sorted.push(node);

        // For each outgoing edge from this node, decrement the in-degree of the destination node
        for neighbor in graph.neighbors_directed(node, petgraph::Outgoing) {
            let degree = in_degree.get_mut(&neighbor).unwrap();
            *degree -= 1;

            if *degree == 0 {
                // Enqueue the neighbor when its in-degree becomes zero, maintain original order priority
                queue.push(Reverse((index_map[&neighbor], neighbor)));
            }
        }
    }

    if sorted.len() == graph.node_count() {
        sorted.into_iter().collect::<IndexSet<_>>()
    } else {
        panic!("Graph has cycles!");
    }
}

#[cfg(test)]
mod tests {

    use txtx_test_utils::test_harness::build_runbook_from_fixture;

    use test_case::test_case;

    use crate::tests::get_addon_by_namespace;

    #[tokio::test]
    async fn it_rejects_circular_dependency_runbooks() {
        let fixture = include_str!("../tests/fixtures/circular.tx");
        let Err(e) =
            build_runbook_from_fixture("circular.tx", fixture, get_addon_by_namespace).await
        else {
            panic!("Missing expected error on circular dependency");
        };
        assert_eq!(e.get(0).unwrap().message, format!("Cycling dependency"));
    }

    #[test_case(include_str!("../tests/fixtures/ab_c.tx"), vec!["a", "b", "c"])]
    #[test_case(include_str!("../tests/fixtures/sorting/1.tx"), vec!["a", "b", "c", "d", "e"]; "multiple 0-index nodes")]
    #[test_case(include_str!("../tests/fixtures/sorting/2.tx"), vec!["e", "d", "c", "b", "a"]; "multiple 0-index nodes sanity check")]
    #[test_case(include_str!("../tests/fixtures/sorting/3.tx"), vec!["a", "b", "c"]; "3 nodes partially ordered")]
    #[test_case(include_str!("../tests/fixtures/sorting/4.tx"), vec!["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"]; "10 nodes reverse topological order")]
    #[test_case(include_str!("../tests/fixtures/sorting/5.tx"), vec!["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"]; "10 nodes topological order")]
    #[test_case(include_str!("../tests/fixtures/sorting/6.tx"), vec!["url", "get", "get_status", "get_status_out", "post", "post_status", "post_status_out"]; "mixed constructs")]
    #[tokio::test]
    async fn it_sorts_graph_and_preserves_declared_order(
        fixture: &str,
        construct_names: Vec<&str>,
    ) {
        let runbook =
            build_runbook_from_fixture("test.tx", fixture, get_addon_by_namespace).await.unwrap();
        let execution_context = runbook.flow_contexts[0].execution_context.clone();
        let order_for_execution = execution_context.order_for_commands_execution;
        let commands_instances = execution_context.commands_instances;
        assert_eq!(order_for_execution.len(), construct_names.len() + 1);
        assert!(commands_instances.get(&order_for_execution[0]).is_none()); // root id
        for (i, name) in construct_names.iter().enumerate() {
            assert_eq!(commands_instances.get(&order_for_execution[i + 1]).unwrap().name, *name);
        }
    }
}
