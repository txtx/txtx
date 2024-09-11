use flow_context::FlowContext;
use kit::hcl::structure::BlockLabel;
use kit::helpers::hcl::RawHclContent;
use kit::types::commands::CommandExecutionResult;
use kit::types::diagnostics::DiagnosticSpan;
use kit::types::stores::ValueStore;
use kit::types::types::RunbookSupervisionContext;
use kit::types::{diagnostics::Diagnostic, types::Value};
use kit::types::{AuthorizationContext, Did, PackageId, RunbookId};
use kit::Addon;
use std::collections::{HashMap, VecDeque};
use txtx_addon_kit::helpers::fs::FileLocation;

mod diffing_context;
mod execution_context;
mod flow_context;
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
    pub flow_contexts: Vec<FlowContext>,
    /// The supervision context keeps track of the supervision settings the runbook is executing under
    pub supervision_context: RunbookSupervisionContext,
    /// Source files
    pub sources: RunbookSources,
    // The store that will contain _all_ of the environment variables (mainnet,testnet,etc), consolidated with the CLI inputs
    pub top_level_inputs_map: RunbookTopLevelInputsMap,
}

impl Runbook {
    pub fn new(runbook_id: RunbookId, description: Option<String>) -> Self {
        Self {
            runbook_id,
            description,
            flow_contexts: vec![],
            runtime_context: RuntimeContext::new(vec![], AuthorizationContext::empty()),
            sources: RunbookSources::new(),
            supervision_context: RunbookSupervisionContext::new(),
            top_level_inputs_map: RunbookTopLevelInputsMap::new(),
        }
    }

    pub fn runbook_id(&self) -> RunbookId {
        self.runbook_id.clone()
    }

    pub fn enable_full_execution_mode(&mut self) {
        for r in self.flow_contexts.iter_mut() {
            r.execution_context.execution_mode = RunbookExecutionMode::Full
        }
    }

    /// Initializes the flow contexts of the runbook.
    /// This method is called when the runbook is first loaded.
    /// It initializes the flow contexts of the runbook by parsing the source code and evaluating the top-level inputs.
    /// If the runbook has no flow blocks, a default flow context is created based on the currently selected top-level inputs environment.
    pub fn initialize_flow_contexts(
        &self,
        runtime_context: &RuntimeContext,
        runbook_sources: &RunbookSources,
        top_level_inputs_map: &RunbookTopLevelInputsMap,
    ) -> Result<Vec<FlowContext>, Diagnostic> {
        let mut dummy_workspace_context = RunbookWorkspaceContext::new(self.runbook_id.clone());
        let mut dummy_execution_context = RunbookExecutionContext::new();

        let current_top_level_value_store = top_level_inputs_map.current_top_level_inputs();

        for (key, value) in current_top_level_value_store.iter() {
            let construct_did = dummy_workspace_context.index_top_level_input(key, value);

            let mut result = CommandExecutionResult::new();
            result.outputs.insert("value".into(), value.clone());
            dummy_execution_context.commands_execution_results.insert(construct_did, result);
        }

        let mut sources = runbook_sources.to_vec_dequeue();
        let dependencies_execution_results = HashMap::new();

        let mut flow_contexts = vec![];

        let mut package_ids = vec![];

        // we need to call flow_context.workspace_context.index_package and flow_context.graph_context.index_package
        // for each flow_context and each package_id, even if the flow is defined in a different package.
        // we also can't index the flow inputs until we have indexed the packages
        // so first we need to create each of the flows by parsing the hcl and finding the flow blocks
        let mut flow_map = HashMap::new();
        while let Some((location, package_name, raw_content)) = sources.pop_front() {
            let package_id =
                PackageId::from_file(&location, &self.runbook_id, &package_name).map_err(|e| e)?;
            package_ids.push(package_id.clone());

            let mut blocks = raw_content.into_blocks().map_err(|diag| diag.location(&location))?;

            while let Some(block) = blocks.pop_front() {
                match block.ident.value().as_str() {
                    "flow" => {
                        let Some(BlockLabel::String(name)) = block.labels.first() else {
                            continue;
                        };
                        let flow_name = name.to_string();
                        let flow_context = FlowContext::new(
                            &flow_name,
                            &self.runbook_id,
                            &current_top_level_value_store,
                        );
                        flow_map.insert(
                            flow_name,
                            (flow_context, block.body.attributes().cloned().collect()),
                        );
                    }
                    _ => {}
                }
            }
        }

        // if the user didn't specify any flows, we'll create a default one based on the current top-level inputs
        if flow_map.is_empty() {
            let flow_name = top_level_inputs_map.current_top_level_input_name();
            let flow_context =
                FlowContext::new(&flow_name, &self.runbook_id, &current_top_level_value_store);
            flow_map.insert(flow_name, (flow_context, vec![]));
        }

        // next we need to index the packages for each flow and evaluate the flow inputs
        for (_, (flow_context, attributes)) in flow_map.iter_mut() {
            for package_id in package_ids.iter() {
                flow_context.workspace_context.index_package(package_id);
                flow_context.graph_context.index_package(package_id);
                flow_context.index_flow_inputs_from_attributes(
                    attributes,
                    &dependencies_execution_results,
                    package_id,
                    &dummy_workspace_context,
                    &dummy_execution_context,
                    runtime_context,
                )?;
                flow_contexts.push(flow_context.to_owned());
            }
        }

        Ok(flow_contexts)
    }

    /// Clears all flow contexts stored on the runbook.
    pub async fn build_contexts_from_sources(
        &mut self,
        sources: RunbookSources,
        top_level_inputs_map: RunbookTopLevelInputsMap,
        authorization_context: AuthorizationContext,
        available_addons: Vec<Box<dyn Addon>>,
    ) -> Result<bool, Vec<Diagnostic>> {
        // Re-initialize some shiny new contexts
        self.flow_contexts.clear();
        let mut runtime_context = RuntimeContext::new(available_addons, authorization_context);

        // Index our flow contexts
        let mut flow_contexts = self
            .initialize_flow_contexts(&runtime_context, &sources, &top_level_inputs_map)
            .map_err(|e| vec![e])?;

        // At this point we know if some batching is required
        for flow_context in flow_contexts.iter_mut() {
            // Step 1: identify the addons at play and their globals
            runtime_context.register_addons_from_sources(
                &mut flow_context.workspace_context,
                &self.runbook_id,
                &sources,
                &flow_context.execution_context,
                &top_level_inputs_map.current_environment,
            )?;
            // Step 2: identify and index all the constructs (nodes)
            flow_context.workspace_context.build_from_sources(
                &sources,
                &runtime_context,
                &mut flow_context.graph_context,
                &mut flow_context.execution_context,
                &top_level_inputs_map.current_environment,
            )?;
            // Step 3: simulate inputs evaluation - some more edges could be hidden in there
            flow_context
                .execution_context
                .simulate_inputs_execution(&runtime_context, &flow_context.workspace_context)
                .await;
            // Step 4: let addons build domain aware dependencies
            let domain_specific_dependencies = runtime_context
                .perform_addon_processing(&mut flow_context.execution_context)
                .map_err(|e| vec![e])?;
            // Step 5: identify and index all the relationships between the constructs (edges)
            flow_context.graph_context.build(
                &mut flow_context.execution_context,
                &flow_context.workspace_context,
                domain_specific_dependencies,
            )?;
        }

        // Final step: Update contexts
        self.flow_contexts = flow_contexts;
        self.runtime_context = runtime_context;
        self.sources = sources;
        self.top_level_inputs_map = top_level_inputs_map;
        Ok(true)
    }

    pub fn find_expected_running_context_mut(&mut self, key: &str) -> &mut FlowContext {
        for running_context in self.flow_contexts.iter_mut() {
            if running_context.top_level_inputs.name.eq(key) {
                return running_context;
            }
        }
        unreachable!()
    }

    pub async fn update_inputs_selector(
        &mut self,
        selector: Option<String>,
        force: bool,
    ) -> Result<bool, Vec<Diagnostic>> {
        // Ensure that the value of the selector is changing
        if !force && selector.eq(&self.top_level_inputs_map.current_environment) {
            return Ok(false);
        }

        // Ensure that the selector exists
        if let Some(ref entry) = selector {
            if !self.top_level_inputs_map.environments.contains(entry) {
                return Err(vec![Diagnostic::error_from_string(format!(
                    "input '{}' unknown from inputs map",
                    entry
                ))]);
            }
        }
        // Rebuild contexts
        let mut inputs_map = self.top_level_inputs_map.clone();
        inputs_map.current_environment = selector;
        let available_addons = self.runtime_context.collect_available_addons();
        let authorization_context: AuthorizationContext =
            self.runtime_context.authorization_context.clone();
        self.build_contexts_from_sources(
            self.sources.clone(),
            inputs_map,
            authorization_context,
            available_addons,
        )
        .await
    }

    pub fn get_inputs_selectors(&self) -> Vec<String> {
        self.top_level_inputs_map.environments.clone()
    }

    pub fn get_active_inputs_selector(&self) -> Option<String> {
        self.top_level_inputs_map.current_environment.clone()
    }
}

#[derive(Clone, Debug)]
pub struct RunbookTopLevelInputsMap {
    pub current_environment: Option<String>,
    pub environments: Vec<String>,
    pub values: HashMap<Option<String>, Vec<(String, Value)>>,
}

pub const DEFAULT_TOP_LEVEL_INPUTS_NAME: &str = "default";

impl RunbookTopLevelInputsMap {
    pub fn new() -> Self {
        Self { current_environment: None, environments: vec![], values: HashMap::new() }
    }

    pub fn current_top_level_input_name(&self) -> String {
        self.current_environment
            .clone()
            .unwrap_or_else(|| DEFAULT_TOP_LEVEL_INPUTS_NAME.to_string())
    }

    pub fn current_top_level_inputs(&self) -> ValueStore {
        let empty_vec = vec![];
        let name = self.current_top_level_input_name();
        let raw_inputs = self.values.get(&self.current_environment).unwrap_or(&empty_vec);
        let current_map = ValueStore::new(&name, &Did::zero()).with_inputs_from_vec(raw_inputs);
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
    pub tree: HashMap<FileLocation, (String, RawHclContent)>,
}

impl RunbookSources {
    pub fn new() -> Self {
        Self { tree: HashMap::new() }
    }

    pub fn add_source(&mut self, name: String, location: FileLocation, content: String) {
        self.tree.insert(location, (name, RawHclContent::from_string(content)));
    }

    pub fn to_vec_dequeue(&self) -> VecDeque<(FileLocation, String, RawHclContent)> {
        self.tree
            .iter()
            .map(|(file_location, (package_name, raw_content))| {
                (file_location.clone(), package_name.clone(), raw_content.clone())
            })
            .collect()
    }
}

pub fn get_source_context_for_diagnostic(
    diag: &Diagnostic,
    runbook_sources: &RunbookSources,
) -> Option<DiagnosticSpan> {
    let Some(construct_location) = &diag.location else {
        return None;
    };
    let Some(span_range) = &diag.span_range() else {
        return None;
    };

    let Some((_, (_, raw_content))) =
        runbook_sources.tree.iter().find(|(location, _)| location.eq(&construct_location))
    else {
        return None;
    };
    let raw_content_string = raw_content.to_string();
    let mut lines = 1;
    let mut cols = 1;
    let mut span = DiagnosticSpan::new();

    let mut chars = raw_content_string.chars().enumerate().peekable();
    while let Some((i, ch)) = chars.next() {
        if i == span_range.start {
            span.line_start = lines;
            span.column_start = cols;
        }
        if i == span_range.end {
            span.line_end = lines;
            span.column_end = cols;
        }
        match ch {
            '\n' => {
                lines += 1;
                cols = 1;
            }
            '\r' => {
                // check for \r\n
                if let Some((_, '\n')) = chars.peek() {
                    // Skip the next character
                    chars.next();
                    lines += 1;
                    cols = 1;
                } else {
                    cols += 1;
                }
            }
            _ => {
                cols += 1;
            }
        }
    }
    Some(span)
}
