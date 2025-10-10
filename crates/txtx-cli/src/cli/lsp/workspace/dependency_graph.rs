//! Dependency graph for tracking file relationships.
//!
//! This module provides the [`DependencyGraph`] type for managing dependencies
//! between txtx documents, detecting cycles, and tracking transitive relationships.
//! It maintains bidirectional edges (forward and reverse) for efficient queries
//! in both directions.

use lsp_types::Url;
use std::collections::{HashMap, HashSet};

/// Dependency graph for tracking file relationships.
///
/// Maintains bidirectional dependency edges between documents:
/// - Forward edges: which documents this document depends on
/// - Reverse edges: which documents depend on this document
///
/// Supports cycle detection with caching and transitive dependency queries.
///
/// # Examples
///
/// ```
/// # use txtx_cli::cli::lsp::workspace::DependencyGraph;
/// # use lsp_types::Url;
/// let mut graph = DependencyGraph::new();
/// let a = Url::parse("file:///a.tx").unwrap();
/// let b = Url::parse("file:///b.tx").unwrap();
///
/// graph.add_dependency(a.clone(), b.clone());
/// assert!(graph.get_dependencies(&a).unwrap().contains(&b));
/// assert!(graph.get_dependents(&b).unwrap().contains(&a));
/// ```
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// Forward edges: document -> documents it depends on.
    depends_on: HashMap<Url, HashSet<Url>>,
    /// Reverse edges: document -> documents that depend on it.
    dependents: HashMap<Url, HashSet<Url>>,
    /// Cycle detection cache.
    has_cycle: Option<bool>,
    /// Nodes involved in cycle (if any).
    cycle_nodes: Vec<Url>,
}

impl DependencyGraph {
    /// Creates a new empty dependency graph.
    pub fn new() -> Self {
        Self {
            depends_on: HashMap::new(),
            dependents: HashMap::new(),
            has_cycle: None,
            cycle_nodes: Vec::new(),
        }
    }

    /// Adds a dependency relationship.
    ///
    /// Creates an edge indicating that `dependent` depends on `depends_on`.
    /// Automatically maintains both forward and reverse edges for efficient
    /// bidirectional queries. Invalidates the cycle detection cache.
    ///
    /// # Arguments
    ///
    /// * `dependent` - The document that has the dependency
    /// * `depends_on` - The document being depended upon
    pub fn add_dependency(&mut self, dependent: Url, depends_on: Url) {
        // Add forward edge
        self.depends_on
            .entry(dependent.clone())
            .or_insert_with(HashSet::new)
            .insert(depends_on.clone());

        // Add reverse edge
        self.dependents
            .entry(depends_on)
            .or_insert_with(HashSet::new)
            .insert(dependent);

        // Invalidate cycle cache
        self.invalidate_cache();
    }

    /// Removes a specific dependency relationship.
    ///
    /// Removes both the forward and reverse edges. Cleans up empty sets
    /// to avoid memory leaks. Invalidates the cycle detection cache.
    ///
    /// # Arguments
    ///
    /// * `dependent` - The document that has the dependency
    /// * `depends_on` - The document being depended upon
    pub fn remove_dependency(&mut self, dependent: &Url, depends_on: &Url) {
        Self::remove_from_map(&mut self.depends_on, dependent, depends_on);
        Self::remove_from_map(&mut self.dependents, depends_on, dependent);
        self.invalidate_cache();
    }

    /// Helper to remove a value from a `HashMap<K, HashSet<V>>`.
    ///
    /// Removes the value from the set, and removes the key entirely if the
    /// set becomes empty. This prevents memory leaks from empty collections.
    fn remove_from_map<K, V>(map: &mut HashMap<K, HashSet<V>>, key: &K, value: &V)
    where
        K: Eq + std::hash::Hash,
        V: Eq + std::hash::Hash,
    {
        if let Some(set) = map.get_mut(key) {
            set.remove(value);
            if set.is_empty() {
                map.remove(key);
            }
        }
    }

    /// Removes all dependencies for a document.
    ///
    /// Called when a document is closed. Cleans up both forward edges
    /// (where `uri` depends on other documents) and reverse edges (where
    /// other documents depend on `uri`). Invalidates the cycle detection cache.
    ///
    /// # Arguments
    ///
    /// * `uri` - The document being removed
    pub fn remove_document(&mut self, uri: &Url) {
        // Remove all forward edges where uri is dependent
        if let Some(dependencies) = self.depends_on.remove(uri) {
            for dependency in dependencies {
                Self::remove_from_map(&mut self.dependents, &dependency, uri);
            }
        }

        // Remove all reverse edges where uri is a dependency
        if let Some(dependents) = self.dependents.remove(uri) {
            for dependent in dependents {
                Self::remove_from_map(&mut self.depends_on, &dependent, uri);
            }
        }

        self.invalidate_cache();
    }

    /// Gets all documents that depend on this document.
    ///
    /// Returns direct dependents only (not transitive). For transitive
    /// dependents, use [`get_affected_documents`](Self::get_affected_documents).
    ///
    /// # Arguments
    ///
    /// * `uri` - The document to query
    ///
    /// # Returns
    ///
    /// `Some` with the set of dependents, or `None` if no documents depend on this one.
    pub fn get_dependents(&self, uri: &Url) -> Option<&HashSet<Url>> {
        self.dependents.get(uri)
    }

    /// Gets all documents that this document depends on.
    ///
    /// Returns direct dependencies only (not transitive).
    ///
    /// # Arguments
    ///
    /// * `uri` - The document to query
    ///
    /// # Returns
    ///
    /// `Some` with the set of dependencies, or `None` if this document has no dependencies.
    pub fn get_dependencies(&self, uri: &Url) -> Option<&HashSet<Url>> {
        self.depends_on.get(uri)
    }

    /// Gets all documents affected by a change to `uri`.
    ///
    /// Recursively collects all transitive dependents. For example, if A depends
    /// on B and B depends on C, then changing C affects both B and A.
    ///
    /// # Arguments
    ///
    /// * `uri` - The document that changed
    ///
    /// # Returns
    ///
    /// A set containing all documents that transitively depend on `uri`.
    pub fn get_affected_documents(&self, uri: &Url) -> HashSet<Url> {
        let mut affected = HashSet::new();
        self.collect_dependents(uri, &mut affected);
        affected
    }

    /// Recursively collects all dependents.
    ///
    /// Uses depth-first traversal with cycle detection (via the `affected` set)
    /// to avoid infinite loops.
    fn collect_dependents(&self, uri: &Url, affected: &mut HashSet<Url>) {
        if let Some(deps) = self.dependents.get(uri) {
            for dep in deps {
                if affected.insert(dep.clone()) {
                    // Only recurse if we haven't seen this dependent before
                    self.collect_dependents(dep, affected);
                }
            }
        }
    }

    /// Detects cycles in the dependency graph using DFS.
    ///
    /// Returns the nodes involved in the cycle if one is found. Results are
    /// cached until the graph is modified. Uses depth-first search with a
    /// recursion stack to detect back edges.
    ///
    /// # Returns
    ///
    /// `Some` with a vector of URLs forming the cycle, or `None` if the graph is acyclic.
    ///
    /// # Examples
    ///
    /// ```
    /// # use txtx_cli::cli::lsp::workspace::DependencyGraph;
    /// # use lsp_types::Url;
    /// let mut graph = DependencyGraph::new();
    /// let a = Url::parse("file:///a.tx").unwrap();
    /// let b = Url::parse("file:///b.tx").unwrap();
    ///
    /// graph.add_dependency(a.clone(), b.clone());
    /// graph.add_dependency(b.clone(), a.clone());
    ///
    /// let cycle = graph.detect_cycles();
    /// assert!(cycle.is_some());
    /// ```
    pub fn detect_cycles(&mut self) -> Option<Vec<Url>> {
        // Return cached result if available
        if let Some(has_cycle) = self.has_cycle {
            return if has_cycle {
                Some(self.cycle_nodes.clone())
            } else {
                None
            };
        }

        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for node in self.depends_on.keys() {
            if !visited.contains(node) {
                if self.dfs_cycle(node, &mut visited, &mut rec_stack, &mut path) {
                    self.has_cycle = Some(true);
                    self.cycle_nodes = path.clone();
                    return Some(path);
                }
            }
        }

        self.has_cycle = Some(false);
        self.cycle_nodes.clear();
        None
    }

    /// DFS-based cycle detection helper.
    ///
    /// Uses the recursion stack to detect back edges, which indicate cycles.
    /// The `path` accumulates nodes as we traverse, and is unwound on backtracking.
    fn dfs_cycle(
        &self,
        node: &Url,
        visited: &mut HashSet<Url>,
        rec_stack: &mut HashSet<Url>,
        path: &mut Vec<Url>,
    ) -> bool {
        visited.insert(node.clone());
        rec_stack.insert(node.clone());
        path.push(node.clone());

        if let Some(neighbors) = self.depends_on.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if self.dfs_cycle(neighbor, visited, rec_stack, path) {
                        return true;
                    }
                } else if rec_stack.contains(neighbor) {
                    // Found a cycle - add the closing node to show the cycle
                    path.push(neighbor.clone());
                    return true;
                }
            }
        }

        rec_stack.remove(node);
        path.pop();
        false
    }

    /// Invalidates the cycle detection cache.
    ///
    /// Called whenever the graph is modified. Forces the next `detect_cycles`
    /// call to perform a full cycle detection.
    fn invalidate_cache(&mut self) {
        self.has_cycle = None;
        self.cycle_nodes.clear();
    }

    /// Gets the total number of documents in the graph.
    ///
    /// Counts unique documents that appear in either forward or reverse edges.
    pub fn document_count(&self) -> usize {
        self.depends_on
            .keys()
            .chain(self.dependents.keys())
            .collect::<HashSet<_>>()
            .len()
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::lsp::tests::test_utils::url;

    #[test]
    fn test_add_dependency() {
        let mut graph = DependencyGraph::new();
        let a = url("a.tx");
        let b = url("b.tx");

        graph.add_dependency(a.clone(), b.clone());

        // Check forward edge
        assert!(graph.depends_on.get(&a).unwrap().contains(&b));

        // Check reverse edge
        assert!(graph.dependents.get(&b).unwrap().contains(&a));
    }

    #[test]
    fn test_remove_dependency() {
        let mut graph = DependencyGraph::new();
        let a = url("a.tx");
        let b = url("b.tx");

        graph.add_dependency(a.clone(), b.clone());
        graph.remove_dependency(&a, &b);

        assert!(graph.depends_on.get(&a).is_none());
        assert!(graph.dependents.get(&b).is_none());
    }

    #[test]
    fn test_get_affected_documents() {
        let mut graph = DependencyGraph::new();
        let manifest = url("txtx.yml");
        let a = url("a.tx");
        let b = url("b.tx");
        let c = url("c.tx");

        // a, b, c all depend on manifest
        graph.add_dependency(a.clone(), manifest.clone());
        graph.add_dependency(b.clone(), manifest.clone());
        graph.add_dependency(c.clone(), manifest.clone());

        let affected = graph.get_affected_documents(&manifest);
        assert_eq!(affected.len(), 3);
        assert!(affected.contains(&a));
        assert!(affected.contains(&b));
        assert!(affected.contains(&c));
    }

    #[test]
    fn test_cycle_detection_no_cycle() {
        let mut graph = DependencyGraph::new();
        let a = url("a.tx");
        let b = url("b.tx");
        let c = url("c.tx");

        // Linear: a -> b -> c
        graph.add_dependency(a, b.clone());
        graph.add_dependency(b, c);

        assert!(graph.detect_cycles().is_none());
    }

    #[test]
    fn test_cycle_detection_simple_cycle() {
        let mut graph = DependencyGraph::new();
        let a = url("a.tx");
        let b = url("b.tx");

        // Cycle: a -> b -> a
        graph.add_dependency(a.clone(), b.clone());
        graph.add_dependency(b.clone(), a.clone());

        let cycle = graph.detect_cycles();
        assert!(cycle.is_some());
        let cycle_nodes = cycle.unwrap();
        assert!(cycle_nodes.contains(&a));
        assert!(cycle_nodes.contains(&b));
    }

    #[test]
    fn test_cycle_detection_complex_cycle() {
        let mut graph = DependencyGraph::new();
        let a = url("a.tx");
        let b = url("b.tx");
        let c = url("c.tx");

        // Cycle: a -> b -> c -> a
        graph.add_dependency(a.clone(), b.clone());
        graph.add_dependency(b.clone(), c.clone());
        graph.add_dependency(c.clone(), a.clone());

        let cycle = graph.detect_cycles();
        assert!(cycle.is_some());
        let cycle_nodes = cycle.unwrap();
        assert!(cycle_nodes.contains(&a));
        assert!(cycle_nodes.contains(&b));
        assert!(cycle_nodes.contains(&c));
    }

    #[test]
    fn test_cycle_detection_cache() {
        let mut graph = DependencyGraph::new();
        let a = url("a.tx");
        let b = url("b.tx");

        graph.add_dependency(a.clone(), b.clone());

        // First detection
        assert!(graph.detect_cycles().is_none());
        assert_eq!(graph.has_cycle, Some(false));

        // Second detection should use cache
        assert!(graph.detect_cycles().is_none());

        // Adding cycle should invalidate cache
        graph.add_dependency(b.clone(), a.clone());
        assert_eq!(graph.has_cycle, None);

        // Detection should find cycle
        assert!(graph.detect_cycles().is_some());
        assert_eq!(graph.has_cycle, Some(true));
    }

    #[test]
    fn test_transitive_dependents() {
        let mut graph = DependencyGraph::new();
        let manifest = url("txtx.yml");
        let a = url("a.tx");
        let b = url("b.tx");
        let c = url("c.tx");

        // manifest <- a <- b <- c
        graph.add_dependency(a.clone(), manifest.clone());
        graph.add_dependency(b.clone(), a.clone());
        graph.add_dependency(c.clone(), b.clone());

        // Changing manifest affects all
        let affected = graph.get_affected_documents(&manifest);
        assert_eq!(affected.len(), 3);

        // Changing a affects b and c
        let affected = graph.get_affected_documents(&a);
        assert_eq!(affected.len(), 2);
        assert!(affected.contains(&b));
        assert!(affected.contains(&c));
    }

    #[test]
    fn test_remove_document() {
        let mut graph = DependencyGraph::new();
        let a = url("a.tx");
        let b = url("b.tx");
        let c = url("c.tx");

        graph.add_dependency(a.clone(), b.clone());
        graph.add_dependency(b.clone(), c.clone());

        // Remove b
        graph.remove_document(&b);

        // a should have no dependencies
        assert!(graph.get_dependencies(&a).is_none());

        // c should have no dependents
        assert!(graph.get_dependents(&c).is_none());
    }

    #[test]
    fn test_remove_document_cleans_up_empty_sets() {
        let mut graph = DependencyGraph::new();
        let a = url("a.tx");
        let b = url("b.tx");

        // Create: a -> b
        graph.add_dependency(a.clone(), b.clone());

        // Verify setup
        assert!(graph.depends_on.contains_key(&a));
        assert!(graph.dependents.contains_key(&b));

        // Remove b (the dependency)
        graph.remove_document(&b);

        // Critical: The empty set in depends_on for 'a' should be removed
        // This is the bug the refactoring fixed - the original code would leave
        // an empty HashSet in depends_on[a] after removing b
        assert!(
            !graph.depends_on.contains_key(&a),
            "Empty dependency set should be cleaned up from depends_on"
        );
        assert!(
            !graph.dependents.contains_key(&b),
            "Entry for removed document should not exist in dependents"
        );

        // Verify the graph is truly empty
        assert_eq!(graph.document_count(), 0, "Graph should have no documents");
    }

    #[test]
    fn test_remove_document_with_multiple_edges_cleans_properly() {
        let mut graph = DependencyGraph::new();
        let a = url("a.tx");
        let b = url("b.tx");
        let c = url("c.tx");
        let d = url("d.tx");

        // Create diamond: a -> b, a -> c, b -> d, c -> d
        graph.add_dependency(a.clone(), b.clone());
        graph.add_dependency(a.clone(), c.clone());
        graph.add_dependency(b.clone(), d.clone());
        graph.add_dependency(c.clone(), d.clone());

        // Remove d - should clean up empty sets in b and c
        graph.remove_document(&d);

        // b and c should still exist but have no dependencies
        assert!(
            graph.depends_on.get(&b).is_none() || graph.depends_on.get(&b).unwrap().is_empty(),
            "b should have no dependencies after d is removed"
        );
        assert!(
            graph.depends_on.get(&c).is_none() || graph.depends_on.get(&c).unwrap().is_empty(),
            "c should have no dependencies after d is removed"
        );

        // Now remove b - should clean up empty set in a's dependencies
        graph.remove_document(&b);

        // a should still have c as dependency
        let a_deps = graph.get_dependencies(&a).expect("a should still have dependencies");
        assert_eq!(a_deps.len(), 1);
        assert!(a_deps.contains(&c));

        // Remove c - should clean up a's last dependency
        graph.remove_document(&c);

        // a should have no dependencies now (empty set cleaned up)
        assert!(
            graph.get_dependencies(&a).is_none(),
            "a should have no dependencies entry after all dependencies removed"
        );
    }
}
