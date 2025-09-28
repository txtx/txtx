use std::collections::{HashMap, HashSet};

/// A graph structure for tracking dependencies and detecting cycles
///
/// This is used to detect circular dependencies in:
/// - Variable definitions (e.g., `var1` depends on `var2` which depends on `var1`)
/// - Action dependencies (e.g., `action1` uses output from `action2` which uses output from `action1`)
///
/// The graph uses depth-first search (DFS) to detect all cycles and report them with
/// precise source locations for debugging.
#[derive(Debug, Clone, Default)]
pub struct DependencyGraph {
    /// Node name -> list of nodes it depends on
    pub(crate) deps: HashMap<String, Vec<String>>,
    /// Node name -> span location for error reporting
    pub(crate) spans: HashMap<String, std::ops::Range<usize>>,
}

impl DependencyGraph {
    /// Create a new empty dependency graph
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a node to the graph, initializing its dependency list if needed
    ///
    /// The span is used for error reporting when a cycle is detected involving this node.
    pub fn add_node(&mut self, name: impl Into<String>, span: Option<std::ops::Range<usize>>) {
        let name = name.into();
        self.deps.entry(name.clone()).or_default();
        if let Some(span) = span {
            self.spans.insert(name, span);
        }
    }

    /// Add a dependency edge from `from` to `to`
    ///
    /// This indicates that `from` depends on `to`. For example, if variable `x` uses
    /// variable `y` in its definition, we add an edge from `x` to `y`.
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn add_edge(&mut self, from: &str, to: impl Into<String>) {
        let to_str = to.into();
        #[cfg(debug_assertions)]
        {
            eprintln!("DEBUG [DependencyGraph]: Adding edge '{}' -> '{}' (called from: {:?})",
                     from, to_str, std::panic::Location::caller());
        }
        if let Some(deps) = self.deps.get_mut(from) {
            deps.push(to_str);
        } else {
            #[cfg(debug_assertions)]
            eprintln!("DEBUG [DependencyGraph]: Warning - node '{}' not found in graph", from);
        }
    }

    /// Find all cycles in the graph using depth-first search
    ///
    /// Returns a vector of cycles, where each cycle is represented as a vector of node names
    /// forming the circular dependency chain. For example: `["var1", "var2", "var3", "var1"]`
    pub fn find_all_cycles(&self) -> Vec<Vec<String>> {
        #[cfg(debug_assertions)]
        eprintln!("DEBUG [DependencyGraph]: Searching for cycles in graph with {} nodes", self.deps.len());
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for node in self.deps.keys() {
            if !visited.contains(node.as_str()) {
                self.dfs_cycles(
                    node,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                    &mut cycles,
                );
            }
        }

        cycles
    }

    /// Extract a cycle from the current path
    ///
    /// When a node in the recursion stack is encountered again, it indicates a cycle.
    /// This method extracts the cycle portion from the current path.
    fn extract_cycle(&self, path: &[String], cycle_start: &str) -> Option<Vec<String>> {
        path.iter()
            .position(|n| n == cycle_start)
            .map(|start| {
                let mut cycle = path[start..].to_vec();
                cycle.push(cycle_start.to_string());
                #[cfg(debug_assertions)]
                eprintln!("DEBUG [DependencyGraph]: Found cycle: {}", cycle.join(" -> "));
                cycle
            })
    }

    /// Process a single neighbor during DFS cycle detection
    ///
    /// Checks if the neighbor creates a cycle (already in recursion stack) or
    /// needs to be explored further (not yet visited).
    fn process_neighbor(
        &self,
        neighbor: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        if rec_stack.contains(neighbor) {
            // Found a cycle
            if let Some(cycle) = self.extract_cycle(path, neighbor) {
                cycles.push(cycle);
            }
        } else if !visited.contains(neighbor) {
            // Continue DFS on unvisited neighbor
            self.dfs_cycles(neighbor, visited, rec_stack, path, cycles);
        }
    }

    /// Depth-first search to find cycles starting from a given node
    ///
    /// Uses the standard DFS cycle detection algorithm with a recursion stack
    /// to track the current path and identify back edges that form cycles.
    fn dfs_cycles(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        // Mark node as visited and add to recursion stack
        visited.insert(node.to_owned());
        rec_stack.insert(node.to_owned());
        path.push(node.to_owned());

        // Process all neighbors
        if let Some(neighbors) = self.deps.get(node) {
            for neighbor in neighbors {
                self.process_neighbor(neighbor, visited, rec_stack, path, cycles);
            }
        }

        // Cleanup before returning
        rec_stack.remove(node);
        path.pop();
    }

    /// Get the span for a node if it exists
    pub fn get_span(&self, node: &str) -> Option<&std::ops::Range<usize>> {
        self.spans.get(node)
    }
}