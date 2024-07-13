use kit::types::diagnostics::Diagnostic;
use kit::types::RunbookId;
use std::collections::HashMap;
use txtx_addon_kit::helpers::fs::FileLocation;

mod execution_context;
mod resolution_context;

pub use execution_context::RunbookExecutionContext;
pub use resolution_context::RunbookResolutionContext;

pub struct RunbookSources {
    /// Map of files required to construct the runbook
    pub tree: HashMap<FileLocation, (String, String)>,
}

impl RunbookSources {
    pub fn new() -> Self {
        Self {
            tree: HashMap::new(),
        }
    }

    pub fn add_source(&mut self, name: String, location: FileLocation, content: String) {
        self.tree.insert(location, (name, content));
    }
}

#[derive(Debug, Clone)]
pub struct Runbook {
    /// Id of the Runbook
    pub runbook_id: RunbookId,
    /// Description of the Runbook
    pub description: Option<String>,
    /// The resolution context contains all the data related to source code analysis and DAG construction
    pub resolution_context: RunbookResolutionContext,
    /// The execution context contains all the data related to the execution of the runbook
    pub execution_context: RunbookExecutionContext,
    /// Diagnostics collected over time, until hitting a fatal error
    pub diagnostics: Vec<Diagnostic>,
}

impl Runbook {
    pub fn new(runbook_id: RunbookId, description: Option<String>) -> Self {
        Self {
            runbook_id,
            description,
            resolution_context: RunbookResolutionContext::new(),
            execution_context: RunbookExecutionContext::new(),
            diagnostics: vec![],
        }
    }

    // add attribute "tainting_change"

    // -> Order the nodes
    // -> Compute canonical id for each node
    //      -> traverse all the inputs, same order, only considering the one with "tainting_change" set to true
    // The edges should be slightly differently created: only if a tainting_change is at stake. 5
    // -> Compute canonical id for each edge (?)
    //      ->
}
