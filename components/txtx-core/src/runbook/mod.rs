use kit::types::commands::CommandExecutionResult;
use kit::types::types::RunbookSupervisionContext;
use kit::types::{diagnostics::Diagnostic, types::Value};
use kit::types::{AuthorizationContext, Did, RunbookId, ValueStore};
use kit::Addon;
use std::collections::HashMap;
use txtx_addon_kit::helpers::fs::FileLocation;

mod diffing_context;
mod execution_context;
mod graph_context;
mod runtime_context;
mod workspace_context;

pub use diffing_context::ConsolidatedChanges;
pub use diffing_context::{RunbookExecutionSnapshot, RunbookSnapshotContext, SynthesizedChange};
pub use execution_context::{RunbookExecutionContext, RunbookExecutionMode};
pub use graph_context::RunbookGraphContext;
pub use runtime_context::{AddonConstructFactory, RuntimeContext};
pub use workspace_context::RunbookWorkspaceContext;

#[derive(Debug)]
pub struct Runbook {
    /// Id of the Runbook
    pub runbook_id: RunbookId,
    /// Description of the Runbook
    pub description: Option<String>,
    /// The runtime context keeps track of all the functions, commands, and signing commands in scope during execution
    pub runtime_context: RuntimeContext,
    /// Running contexts
    pub running_contexts: Vec<RunningContext>,
    /// The supervision context keeps track of the supervision settings the runbook is executing under
    pub supervision_context: RunbookSupervisionContext,
    /// Source files
    pub sources: RunbookSources,
    pub inputs_map: RunbookInputsMap,
}

impl Runbook {
    pub fn new(runbook_id: RunbookId, description: Option<String>) -> Self {
        Self {
            runbook_id,
            description,
            running_contexts: vec![],
            runtime_context: RuntimeContext::new(vec![], AuthorizationContext::empty()),
            sources: RunbookSources::new(),
            supervision_context: RunbookSupervisionContext::new(),
            inputs_map: RunbookInputsMap::new(),
        }
    }

    pub fn runbook_id(&self) -> RunbookId {
        self.runbook_id.clone()
    }

    pub fn enable_full_execution_mode(&mut self) {
        for r in self.running_contexts.iter_mut() {
            r.execution_context.execution_mode = RunbookExecutionMode::Full
        }
    }

    pub fn build_contexts_from_sources(
        &mut self,
        sources: RunbookSources,
        inputs_map: RunbookInputsMap,
        authorization_context: AuthorizationContext,
        available_addons: Vec<Box<dyn Addon>>,
    ) -> Result<bool, Vec<Diagnostic>> {
        // Re-initialize some shiny new contexts
        self.running_contexts.clear();
        let mut runtime_context = RuntimeContext::new(available_addons, authorization_context);

        runtime_context.load_all_addons(&self.runbook_id, &sources)?;

        let inputs_sets = runtime_context.collect_environment_variables(
            &self.runbook_id,
            &inputs_map,
            &sources,
        )?;

        // At this point we know if some batching is required
        for inputs_set in inputs_sets.iter() {
            // We're initializing some new contexts
            let mut running_context = RunningContext::new(&self.runbook_id, inputs_set);

            // Step 1: identify the addons at play and their globals
            runtime_context.build_from_sources(
                &mut running_context.workspace_context,
                &self.runbook_id,
                &inputs_sets,
                &sources,
                &running_context.execution_context,
            )?;
            // Step 2: identify and index all the constructs (nodes)
            running_context.workspace_context.build_from_sources(
                &sources,
                &runtime_context,
                &mut running_context.graph_context,
                &mut running_context.execution_context,
            )?;
            // Step 3: simulate inputs evaluation - some more edges could be hidden in there
            running_context
                .execution_context
                .simulate_inputs_execution(&runtime_context, &running_context.workspace_context);
            // Step 4: let addons build domain aware dependencies
            let domain_specific_dependencies = runtime_context
                .collect_domain_specific_dependencies(&running_context.execution_context)
                .map_err(|e| vec![e])?;
            // Step 5: identify and index all the relationships between the constructs (edges)
            running_context.graph_context.build(
                &mut running_context.execution_context,
                &running_context.workspace_context,
                domain_specific_dependencies,
            )?;

            self.running_contexts.push(running_context)
        }

        // Final step: Update contexts
        self.runtime_context = runtime_context;
        self.sources = sources;
        self.inputs_map = inputs_map;
        Ok(true)
    }

    pub fn find_expected_running_context_mut(&mut self, key: &str) -> &mut RunningContext {
        for running_context in self.running_contexts.iter_mut() {
            if running_context.inputs_set.name.eq(key) {
                return running_context;
            }
        }
        unreachable!()
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
        let available_addons = self.runtime_context.collect_available_addons();
        let authorization_context = self.runtime_context.authorization_context.clone();
        self.build_contexts_from_sources(
            self.sources.clone(),
            inputs_map,
            authorization_context,
            available_addons,
        )
    }

    pub fn get_inputs_selectors(&self) -> Vec<String> {
        self.inputs_map.environments.clone()
    }

    pub fn get_active_inputs_selector(&self) -> Option<String> {
        self.inputs_map.current.clone()
    }
}

#[derive(Clone, Debug)]
pub struct RunningContext {
    /// The resolution context contains all the data related to source code analysis and DAG construction
    pub graph_context: RunbookGraphContext,
    /// The execution context contains all the data related to the execution of the runbook
    pub execution_context: RunbookExecutionContext,
    /// The workspace context keeps track of packages and constructs reachable
    pub workspace_context: RunbookWorkspaceContext,
    /// The set of environment variables used during the execution
    pub inputs_set: ValueStore,
}

impl RunningContext {
    pub fn new(runbook_id: &RunbookId, inputs_set: &ValueStore) -> Self {
        let workspace_context = RunbookWorkspaceContext::new(runbook_id.clone());
        let graph_context = RunbookGraphContext::new();
        let execution_context = RunbookExecutionContext::new();
        let mut running_context = Self {
            workspace_context,
            graph_context,
            execution_context,
            inputs_set: inputs_set.clone(),
        };
        running_context.index_environment_variables(inputs_set);
        running_context
    }

    pub fn is_enabled(&self) -> bool {
        !self
            .execution_context
            .execution_mode
            .eq(&RunbookExecutionMode::Ignored)
    }

    pub fn index_environment_variables(&mut self, inputs_set: &ValueStore) {
        for (key, value) in inputs_set.iter() {
            let construct_did = self
                .workspace_context
                .index_environment_variable(key, value);
            self.graph_context
                .index_environment_variable(&construct_did);
            let mut result = CommandExecutionResult::new();
            result.outputs.insert("value".into(), value.clone());
            self.execution_context
                .commands_execution_results
                .insert(construct_did, result);
        }
    }
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

    pub fn current_inputs_set(&self) -> ValueStore {
        let empty_vec = vec![];
        let raw_inputs = self.values.get(&self.current).unwrap_or(&empty_vec);
        let mut current_map = ValueStore::new("env", &Did::zero());
        for (k, v) in raw_inputs.iter() {
            current_map.insert(k, v.clone());
        }
        current_map
    }

    pub fn override_values_with_cli_inputs(
        &mut self,
        inputs: &Vec<String>,
        buffer_stdin: Option<String>,
    ) -> Result<(), String> {
        for input in inputs.iter() {
            let Some((input_name, input_value)) = input.split_once("=") else {
                return Err(format!(
                    "expected --input argument to be formatted as '{}', got '{}'",
                    "key=value", input
                ));
            };
            let input_value = match (input_value.eq("←"), &buffer_stdin) {
                (true, Some(v)) => v.to_string(),
                _ => input_value.to_string(),
            };
            let new_value = Value::parse_and_default_to_string(&input_value);
            for (_, values) in self.values.iter_mut() {
                let mut found = false;
                for (k, old_value) in values.iter_mut() {
                    if k.eq(&input_name) {
                        *old_value = new_value.clone();
                        found = true;
                    }
                }
                if !found {
                    values.push((input_name.to_string(), new_value.clone()));
                }
            }
        }
        Ok(())
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
