use kit::types::commands::CommandExecutionResult;
use kit::types::types::RunbookSupervisionContext;
use kit::types::RunbookId;
use kit::types::{diagnostics::Diagnostic, types::Value};
use std::collections::HashMap;
use txtx_addon_kit::helpers::fs::FileLocation;

mod diffing_context;
mod execution_context;
mod graph_context;
mod runtime_context;
mod workspace_context;

pub use diffing_context::{RunbookExecutionSnapshot, RunbookSnapshotContext};
pub use execution_context::RunbookExecutionContext;
pub use graph_context::RunbookGraphContext;
pub use runtime_context::RuntimeContext;
pub use workspace_context::RunbookWorkspaceContext;

#[derive(Debug)]
pub struct Runbook {
    /// The resolution context contains all the data related to source code analysis and DAG construction
    pub graph_context: RunbookGraphContext,
    /// The execution context contains all the data related to the execution of the runbook
    pub execution_context: RunbookExecutionContext,
    /// The workspace context keeps track of packages and constructs reachable
    pub workspace_context: RunbookWorkspaceContext,
    /// The runtime context keeps track of all the functions, commands, and signing commands in scope during execution
    pub runtime_context: RuntimeContext,
    /// The supervision context keeps track of the supervision settings the runbook is executing under
    pub supervision_context: RunbookSupervisionContext,
    /// Source files
    pub sources: RunbookSources,
    /// Runbook inputs
    pub inputs_map: RunbookInputsMap,
}

impl Runbook {
    pub fn new(runbook_id: RunbookId, description: Option<String>) -> Self {
        Self {
            workspace_context: RunbookWorkspaceContext::new(runbook_id, description),
            graph_context: RunbookGraphContext::new(),
            execution_context: RunbookExecutionContext::new(),
            runtime_context: RuntimeContext::new(),
            sources: RunbookSources::new(),
            inputs_map: RunbookInputsMap::new(),
            supervision_context: RunbookSupervisionContext::new(),
        }
    }

    pub fn runbook_id(&self) -> RunbookId {
        self.workspace_context.runbook_id.clone()
    }

    pub fn build_contexts_from_sources(
        &mut self,
        sources: RunbookSources,
        inputs_map: RunbookInputsMap,
    ) -> Result<bool, Vec<Diagnostic>> {
        // Re-initialize some shiny new contexts
        let mut runtime_context = RuntimeContext::new();
        let mut workspace_context = RunbookWorkspaceContext::new(
            self.workspace_context.runbook_id.clone(),
            self.workspace_context.description.clone(),
        );
        let mut graph_context = RunbookGraphContext::new();
        let mut execution_context = RunbookExecutionContext::new();

        // Step 0: inject runbook inputs (environment variables, etc)
        inputs_map.seed_contexts(
            &mut workspace_context,
            &mut graph_context,
            &mut execution_context,
        );
        // Step 1: identify the addons at play and their globals
        runtime_context.build_from_sources(
            &sources,
            &workspace_context.runbook_id,
            &workspace_context,
            &execution_context,
        )?;
        // Step 2: identify and index all the constructs (nodes)
        workspace_context.build_from_sources(
            &sources,
            &mut runtime_context,
            &mut graph_context,
            &mut execution_context,
        )?;
        // Step 3: identify and index all the relationships between the constructs (edges)
        graph_context.build(&mut execution_context, &workspace_context)?;

        // Final step: Update contexts
        self.runtime_context = runtime_context;
        self.workspace_context = workspace_context;
        self.graph_context = graph_context;
        self.execution_context = execution_context;
        self.sources = sources;
        self.inputs_map = inputs_map;

        Ok(true)
    }

    pub fn update_inputs_selector(
        &mut self,
        selector: Option<String>,
        force: bool,
    ) -> Result<bool, Vec<Diagnostic>> {
        // Ensure that the value of the selector is changing
        if !force && selector.eq(&self.inputs_map.current) {
            return Ok(false);
        }

        // Ensure that the selector exists
        if let Some(ref entry) = selector {
            if !self.inputs_map.environments.contains(entry) {
                return Err(vec![diagnosed_error!(
                    "input '{}' unknown from inputs map",
                    entry
                )]);
            }
        }

        // Rebuild contexts
        let mut inputs_map = self.inputs_map.clone();
        inputs_map.current = selector;
        self.build_contexts_from_sources(self.sources.clone(), inputs_map)
    }

    pub fn get_inputs_selectors(&self) -> Vec<String> {
        self.inputs_map.environments.clone()
    }

    pub fn get_active_inputs_selector(&self) -> Option<String> {
        self.inputs_map.current.clone()
    }

    pub fn seed_runbook_inputs(&mut self, runbook_inputs: &HashMap<String, Value>) {}
}

#[derive(Clone, Debug)]
pub struct RunbookInputsMap {
    pub current: Option<String>,
    pub environments: Vec<String>,
    pub values: HashMap<Option<String>, Vec<(String, Value)>>,
}

impl RunbookInputsMap {
    pub fn new() -> Self {
        Self {
            current: None,
            environments: vec![],
            values: HashMap::new(),
        }
    }

    pub fn seed_contexts(
        &self,
        workspace_context: &mut RunbookWorkspaceContext,
        graph_context: &mut RunbookGraphContext,
        execution_context: &mut RunbookExecutionContext,
    ) {
        for (key, value) in self.current_map().iter() {
            let construct_did = workspace_context.index_environment_variable(key, value);
            graph_context.index_environment_variable(&construct_did);
            let mut result = CommandExecutionResult::new();
            result.outputs.insert("value".into(), value.clone());
            execution_context
                .commands_execution_results
                .insert(construct_did, result);
        }
    }

    pub fn current_map(&self) -> HashMap<String, Value> {
        let empty_vec = vec![];
        let raw_inputs = self.values.get(&self.current).unwrap_or(&empty_vec);
        let mut current_map = HashMap::new();
        for (k, v) in raw_inputs.iter() {
            current_map.insert(k.clone(), v.clone());
        }
        current_map
    }
}

#[derive(Clone, Debug)]
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
