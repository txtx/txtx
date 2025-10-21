use diffing_context::ConsolidatedPlanChanges;
use flow_context::FlowContext;
use kit::indexmap::IndexMap;
use kit::types::cloud_interface::CloudServiceContext;
use kit::types::frontend::ActionItemRequestType;
use kit::types::types::AddonJsonConverter;
use kit::types::{ConstructDid, RunbookInstanceContext};
use serde_json::{json, Value as JsonValue};
use std::collections::{HashMap, HashSet, VecDeque};
use txtx_addon_kit::hcl::structure::BlockLabel;
use txtx_addon_kit::hcl::Span;
use txtx_addon_kit::helpers::fs::FileLocation;
use txtx_addon_kit::helpers::hcl::RawHclContent;
use txtx_addon_kit::types::commands::{CommandExecutionResult, DependencyExecutionResultCache};
use txtx_addon_kit::types::diagnostics::DiagnosticSpan;
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::RunbookSupervisionContext;
use txtx_addon_kit::types::{diagnostics::Diagnostic, types::Value};
use txtx_addon_kit::types::{AuthorizationContext, Did, PackageId, RunbookId};
use txtx_addon_kit::Addon;

mod diffing_context;
pub mod embedded_runbook;
mod execution_context;
pub mod flow_context;
mod graph_context;
mod runtime_context;
mod workspace_context;

pub use diffing_context::ConsolidatedChanges;
pub use diffing_context::{RunbookExecutionSnapshot, RunbookSnapshotContext, SynthesizedChange};
pub use execution_context::{RunbookExecutionContext, RunbookExecutionMode};
pub use graph_context::RunbookGraphContext;
pub use runtime_context::{AddonConstructFactory, RuntimeContext};
pub use workspace_context::RunbookWorkspaceContext;

use crate::manifest::{RunbookStateLocation, RunbookTransientStateLocation};

#[derive(Debug)]
pub struct Runbook {
    /// Id of the Runbook
    pub runbook_id: RunbookId,
    /// Description of the Runbook
    pub description: Option<String>,
    /// The runtime context keeps track of all the functions, commands, and signing commands in scope during execution
    pub runtime_context: RuntimeContext,
    /// Running contexts
    flow_contexts: Vec<FlowContext>,
    /// The supervision context keeps track of the supervision settings the runbook is executing under
    pub supervision_context: RunbookSupervisionContext,
    /// Source files
    pub sources: RunbookSources,
    // The store that will contain _all_ of the environment variables (mainnet,testnet,etc), consolidated with the CLI inputs
    pub top_level_inputs_map: RunbookTopLevelInputsMap,
}

impl Runbook {
    fn get_no_addons_by_namespace(_namepace: &str) -> Option<Box<dyn Addon>> {
        None
    }
    pub fn new(runbook_id: RunbookId, description: Option<String>) -> Self {
        Self {
            runbook_id,
            description,
            flow_contexts: vec![],
            runtime_context: RuntimeContext::new(
                AuthorizationContext::empty(),
                Runbook::get_no_addons_by_namespace,
                CloudServiceContext::empty(),
            ),
            sources: RunbookSources::new(),
            supervision_context: RunbookSupervisionContext::new(),
            top_level_inputs_map: RunbookTopLevelInputsMap::new(),
        }
    }

    pub fn runbook_id(&self) -> RunbookId {
        self.runbook_id.clone()
    }

    pub fn to_instance_context(&self) -> RunbookInstanceContext {
        RunbookInstanceContext {
            runbook_id: self.runbook_id.clone(),
            workspace_location: self
                .runtime_context
                .authorization_context
                .workspace_location
                .clone(),
            environment_selector: self.top_level_inputs_map.current_environment.clone(),
        }
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
        let dependencies_execution_results = DependencyExecutionResultCache::new();

        let mut flow_contexts = vec![];

        let mut package_ids = vec![];

        // we need to call flow_context.workspace_context.index_package and flow_context.graph_context.index_package
        // for each flow_context and each package_id, even if the flow is defined in a different package.
        // we also can't index the flow inputs until we have indexed the packages
        // so first we need to create each of the flows by parsing the hcl and finding the flow blocks
        let mut flow_map = vec![];
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
                        flow_map.push((flow_context, block.body.attributes().cloned().collect()));
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
            flow_map.push((flow_context, vec![]));
        }

        // next we need to index the packages for each flow and evaluate the flow inputs
        for (flow_context, attributes) in flow_map.iter_mut() {
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
            }
            flow_contexts.push(flow_context.to_owned());
        }

        Ok(flow_contexts)
    }

    /// Clears all flow contexts stored on the runbook.
    pub async fn build_contexts_from_sources(
        &mut self,
        sources: RunbookSources,
        top_level_inputs_map: RunbookTopLevelInputsMap,
        authorization_context: AuthorizationContext,
        get_addon_by_namespace: fn(&str) -> Option<Box<dyn Addon>>,
        cloud_service_context: CloudServiceContext,
    ) -> Result<bool, Vec<Diagnostic>> {
        // Re-initialize some shiny new contexts
        self.flow_contexts.clear();
        let mut runtime_context = RuntimeContext::new(
            authorization_context,
            get_addon_by_namespace,
            cloud_service_context,
        );

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
            flow_context
                .workspace_context
                .build_from_sources(
                    &sources,
                    &mut runtime_context,
                    &mut flow_context.graph_context,
                    &mut flow_context.execution_context,
                    &top_level_inputs_map.current_environment,
                )
                .await?;

            // Step 3: simulate inputs evaluation - some more edges could be hidden in there
            flow_context
                .execution_context
                .simulate_inputs_execution(&runtime_context, &flow_context.workspace_context)
                .await
                .map_err(|diag| {
                    vec![diag
                        .clone()
                        .set_diagnostic_span(get_source_context_for_diagnostic(&diag, &sources))]
                })?;
            // Step 4: let addons build domain aware dependencies
            let domain_specific_dependencies = runtime_context
                .perform_addon_processing(&mut flow_context.execution_context)
                .map_err(|(diag, construct_did)| {
                    let construct_id =
                        &flow_context.workspace_context.expect_construct_id(&construct_did);
                    let command_instance = flow_context
                        .execution_context
                        .commands_instances
                        .get(&construct_did)
                        .unwrap();
                    let diag = diag
                        .location(&construct_id.construct_location)
                        .set_span_range(command_instance.block.span());
                    vec![diag
                        .clone()
                        .set_diagnostic_span(get_source_context_for_diagnostic(&diag, &sources))]
                })?;
            // Step 5: identify and index all the relationships between the constructs (edges)
            flow_context
                .graph_context
                .build(
                    &mut flow_context.execution_context,
                    &flow_context.workspace_context,
                    domain_specific_dependencies,
                )
                .map_err(|diags| {
                    diags
                        .into_iter()
                        .map(|diag| {
                            diag.clone().set_diagnostic_span(get_source_context_for_diagnostic(
                                &diag, &sources,
                            ))
                        })
                        .collect::<Vec<_>>()
                })?;
        }

        // Final step: Update contexts
        self.flow_contexts = flow_contexts;
        self.runtime_context = runtime_context;
        self.sources = sources;
        self.top_level_inputs_map = top_level_inputs_map;
        Ok(true)
    }

    pub fn find_expected_flow_context_mut(&mut self, key: &str) -> &mut FlowContext {
        for flow_context in self.flow_contexts.iter_mut() {
            if flow_context.name.eq(key) {
                return flow_context;
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
        let authorization_context: AuthorizationContext =
            self.runtime_context.authorization_context.clone();
        let cloud_service_context: CloudServiceContext =
            self.runtime_context.cloud_service_context.clone();

        self.build_contexts_from_sources(
            self.sources.clone(),
            inputs_map,
            authorization_context,
            self.runtime_context.addons_context.get_addon_by_namespace,
            cloud_service_context,
        )
        .await
    }

    pub fn get_inputs_selectors(&self) -> Vec<String> {
        self.top_level_inputs_map.environments.clone()
    }

    pub fn get_active_inputs_selector(&self) -> Option<String> {
        self.top_level_inputs_map.current_environment.clone()
    }

    pub fn backup_execution_contexts(&self) -> HashMap<String, RunbookExecutionContext> {
        let mut execution_context_backups = HashMap::new();
        for flow_context in self.flow_contexts.iter() {
            let execution_context_backup = flow_context.execution_context.clone();
            execution_context_backups.insert(flow_context.name.clone(), execution_context_backup);
        }
        execution_context_backups
    }

    pub async fn simulate_and_snapshot_flows(
        &mut self,
        old_snapshot: &RunbookExecutionSnapshot,
    ) -> Result<RunbookExecutionSnapshot, String> {
        let ctx = RunbookSnapshotContext::new();

        for flow_context in self.flow_contexts.iter_mut() {
            let frontier = HashSet::new();
            let _res = flow_context
                .execution_context
                .simulate_execution(
                    &self.runtime_context,
                    &flow_context.workspace_context,
                    &self.supervision_context,
                    &frontier,
                )
                .await;

            let Some(flow_snapshot) = old_snapshot.flows.get(&flow_context.name) else {
                continue;
            };

            // since our simulation results are limited, apply the old snapshot on top of the gaps in
            // our simulated execution context
            flow_context
                .execution_context
                .apply_snapshot_to_execution_context(flow_snapshot, &flow_context.workspace_context)
                .map_err(|e| e.message)?;
        }

        let new = ctx
            .snapshot_runbook_execution(
                &self.runbook_id,
                &self.flow_contexts,
                None,
                &self.top_level_inputs_map,
            )
            .map_err(|e| e.message)?;
        Ok(new)
    }

    pub fn prepare_flows_for_new_plans(
        &mut self,
        new_plans_to_add: &Vec<String>,
        execution_context_backups: HashMap<String, RunbookExecutionContext>,
    ) {
        for flow_context_key in new_plans_to_add.iter() {
            let flow_context = self.find_expected_flow_context_mut(&flow_context_key);
            flow_context.execution_context.execution_mode = RunbookExecutionMode::Full;
            let pristine_execution_context =
                execution_context_backups.get(flow_context_key).unwrap();
            flow_context.execution_context = pristine_execution_context.clone();
        }
    }

    pub fn prepared_flows_for_updated_plans(
        &mut self,
        plans_to_update: &IndexMap<String, ConsolidatedPlanChanges>,
    ) -> (
        IndexMap<String, Vec<(String, Option<String>)>>,
        IndexMap<String, Vec<(String, Option<String>)>>,
    ) {
        let mut actions_to_re_execute = IndexMap::new();
        let mut actions_to_execute = IndexMap::new();

        for (flow_context_key, changes) in plans_to_update.iter() {
            let critical_edits = changes
                .constructs_to_update
                .iter()
                .filter(|c| !c.description.is_empty() && c.critical)
                .collect::<Vec<_>>();

            let additions = changes.new_constructs_to_add.iter().collect::<Vec<_>>();
            let mut unexecuted =
                changes.constructs_to_run.iter().map(|(e, _)| e.clone()).collect::<Vec<_>>();

            let flow_context = self.find_expected_flow_context_mut(&flow_context_key);

            if critical_edits.is_empty() && additions.is_empty() && unexecuted.is_empty() {
                flow_context.execution_context.execution_mode = RunbookExecutionMode::Ignored;
                continue;
            }

            let mut added_construct_dids: Vec<ConstructDid> =
                additions.into_iter().map(|(construct_did, _)| construct_did.clone()).collect();

            let mut descendants_of_critically_changed_commands = critical_edits
                .iter()
                .filter_map(|c| {
                    if let Some(construct_did) = &c.construct_did {
                        let mut segment = vec![];
                        segment.push(construct_did.clone());
                        let mut deps = flow_context
                            .graph_context
                            .get_downstream_dependencies_for_construct_did(&construct_did, true);
                        segment.append(&mut deps);
                        Some(segment)
                    } else {
                        None
                    }
                })
                .flatten()
                .filter(|d| !added_construct_dids.contains(d))
                .collect::<Vec<_>>();
            descendants_of_critically_changed_commands.sort();
            descendants_of_critically_changed_commands.dedup();

            let actions: Vec<(String, Option<String>)> = descendants_of_critically_changed_commands
                .iter()
                .map(|construct_did| {
                    let documentation = flow_context
                        .execution_context
                        .commands_inputs_evaluation_results
                        .get(construct_did)
                        .and_then(|r| r.inputs.get_string("description"))
                        .and_then(|d| Some(d.to_string()));
                    let command = flow_context
                        .execution_context
                        .commands_instances
                        .get(construct_did)
                        .unwrap();
                    (command.name.to_string(), documentation)
                })
                .collect();
            actions_to_re_execute.insert(flow_context_key.clone(), actions);

            let added_actions: Vec<(String, Option<String>)> = added_construct_dids
                .iter()
                .map(|construct_did| {
                    let documentation = flow_context
                        .execution_context
                        .commands_inputs_evaluation_results
                        .get(construct_did)
                        .and_then(|r| r.inputs.get_string("description"))
                        .and_then(|d| Some(d.to_string()));
                    let command = flow_context
                        .execution_context
                        .commands_instances
                        .get(construct_did)
                        .unwrap();
                    (command.name.to_string(), documentation)
                })
                .collect();
            actions_to_execute.insert(flow_context_key.clone(), added_actions);

            let mut great_filter = descendants_of_critically_changed_commands;
            great_filter.append(&mut added_construct_dids);
            great_filter.append(&mut unexecuted);

            for construct_did in great_filter.iter() {
                let _ =
                    flow_context.execution_context.commands_execution_results.remove(construct_did);
            }

            flow_context.execution_context.order_for_commands_execution = flow_context
                .execution_context
                .order_for_commands_execution
                .clone()
                .into_iter()
                .filter(|c| great_filter.contains(&c))
                .collect();

            flow_context.execution_context.execution_mode =
                RunbookExecutionMode::Partial(great_filter);
        }

        (actions_to_re_execute, actions_to_execute)
    }

    pub fn write_runbook_state(
        &self,
        runbook_state_location: Option<RunbookStateLocation>,
    ) -> Result<Option<FileLocation>, String> {
        if let Some(state_file_location) = runbook_state_location {
            let previous_snapshot = match state_file_location.load_execution_snapshot(
                true,
                &self.runbook_id.name,
                &self.top_level_inputs_map.current_top_level_input_name(),
            ) {
                Ok(snapshot) => Some(snapshot),
                Err(_e) => None,
            };

            let state_file_location = state_file_location.get_location_for_ctx(
                &self.runbook_id.name,
                Some(&self.top_level_inputs_map.current_top_level_input_name()),
            );
            if let Some(RunbookTransientStateLocation(lock_file)) =
                RunbookTransientStateLocation::from_state_file_location(&state_file_location)
            {
                let _ = std::fs::remove_file(&lock_file.to_string());
            }

            let diff = RunbookSnapshotContext::new();
            let snapshot = diff
                .snapshot_runbook_execution(
                    &self.runbook_id,
                    &self.flow_contexts,
                    previous_snapshot,
                    &self.top_level_inputs_map,
                )
                .map_err(|e| e.message)?;
            state_file_location
                .write_content(serde_json::to_string_pretty(&snapshot).unwrap().as_bytes())
                .expect("unable to save state");
            Ok(Some(state_file_location))
        } else {
            Ok(None)
        }
    }

    pub fn mark_failed_and_write_transient_state(
        &mut self,
        runbook_state_location: Option<RunbookStateLocation>,
    ) -> Result<Option<FileLocation>, String> {
        for running_context in self.flow_contexts.iter_mut() {
            running_context.execution_context.execution_mode = RunbookExecutionMode::FullFailed;
        }

        if let Some(runbook_state_location) = runbook_state_location {
            let previous_snapshot = match runbook_state_location.load_execution_snapshot(
                false,
                &self.runbook_id.name,
                &self.top_level_inputs_map.current_top_level_input_name(),
            ) {
                Ok(snapshot) => Some(snapshot),
                Err(_e) => None,
            };

            let lock_file = RunbookTransientStateLocation::get_location_from_state_file_location(
                &runbook_state_location.get_location_for_ctx(
                    &self.runbook_id.name,
                    Some(&self.top_level_inputs_map.current_top_level_input_name()),
                ),
            );
            let diff = RunbookSnapshotContext::new();
            let snapshot = diff
                .snapshot_runbook_execution(
                    &self.runbook_id,
                    &self.flow_contexts,
                    previous_snapshot,
                    &self.top_level_inputs_map,
                )
                .map_err(|e| e.message)?;
            lock_file
                .write_content(serde_json::to_string_pretty(&snapshot).unwrap().as_bytes())
                .map_err(|e| format!("unable to save state ({})", e.to_string()))?;
            Ok(Some(lock_file))
        } else {
            Ok(None)
        }
    }

    pub fn collect_formatted_outputs(&self) -> RunbookOutputs {
        let mut runbook_outputs = RunbookOutputs::new();
        for flow_context in self.flow_contexts.iter() {
            let grouped_actions_items = flow_context
                .execution_context
                .collect_outputs_constructs_results(&self.runtime_context.authorization_context);
            for (_, items) in grouped_actions_items.iter() {
                for item in items.iter() {
                    if let ActionItemRequestType::DisplayOutput(ref output) = item.action_type {
                        runbook_outputs.add_output(
                            &flow_context.name,
                            &output.name,
                            &output.value,
                            &output.description,
                        );
                    }
                }
            }
        }
        runbook_outputs
    }
}

#[derive(Clone, Debug)]
pub struct RunbookOutputs {
    outputs: IndexMap<String, IndexMap<String, (Value, Option<String>)>>,
}
impl RunbookOutputs {
    pub fn new() -> Self {
        Self { outputs: IndexMap::new() }
    }

    pub fn add_output(
        &mut self,
        flow_name: &str,
        output_name: &str,
        output_value: &Value,
        output_description: &Option<String>,
    ) {
        let flow_outputs = self.outputs.entry(flow_name.to_string()).or_insert_with(IndexMap::new);
        flow_outputs
            .insert(output_name.to_string(), (output_value.clone(), output_description.clone()));
    }

    /// Organizes the outputs in a format suitable to be displayed using the `AsciiTable` crate.
    pub fn get_output_row_data(
        &self,
        filter: &Option<String>,
    ) -> IndexMap<String, Vec<Vec<String>>> {
        let mut output_row_data = IndexMap::new();
        for (flow_name, flow_outputs) in self.outputs.iter() {
            let mut flow_output_row =
                vec![vec!["name".to_string(), "value".to_string(), "description".to_string()]];
            for (output_name, (output_value, output_description)) in flow_outputs.iter() {
                if let Some(ref filter) = filter {
                    if !output_name.contains(filter) {
                        continue;
                    }
                }

                let mut row = vec![];
                row.push(output_name.to_string());
                row.push(output_value.to_string());
                row.push(output_description.clone().unwrap_or_else(|| "".to_string()));
                flow_output_row.push(row);
            }
            output_row_data.insert(flow_name.to_string(), flow_output_row);
        }
        output_row_data
    }

    pub fn to_json(&self, addon_converters: &Vec<AddonJsonConverter>) -> JsonValue {
        let mut json = json!({});
        let only_one_flow = self.outputs.len() == 1;
        for (flow_name, flow_outputs) in self.outputs.iter() {
            let mut flow_json = json!({});
            for (output_name, (output_value, output_description)) in flow_outputs.iter() {
                let mut output_json = json!({});
                output_json["value"] = output_value.to_json(Some(&addon_converters));
                if let Some(ref output_description) = output_description {
                    output_json["description"] = output_description.clone().into();
                }
                flow_json[output_name] = output_json;
            }
            if only_one_flow {
                return flow_json;
            }
            json[flow_name] = flow_json;
        }
        json
    }

    pub fn is_empty(&self) -> bool {
        if self.outputs.is_empty() {
            return true;
        }
        let mut empty = true;
        for (_, outputs) in self.outputs.iter() {
            if !outputs.is_empty() {
                empty = false;
            }
        }
        empty
    }
}

#[derive(Clone, Debug)]
pub struct RunbookTopLevelInputsMap {
    current_environment: Option<String>,
    environments: Vec<String>,
    values: HashMap<Option<String>, Vec<(String, Value)>>,
}

pub const DEFAULT_TOP_LEVEL_INPUTS_NAME: &str = "default";
pub const GLOBAL_TOP_LEVEL_INPUTS_NAME: &str = "global";

impl RunbookTopLevelInputsMap {
    pub fn new() -> Self {
        Self { current_environment: None, environments: vec![], values: HashMap::new() }
    }
    pub fn from_environment_map(
        selector: &Option<String>,
        environments_map: &IndexMap<String, IndexMap<String, String>>,
    ) -> Self {
        let mut environments = vec![];
        let mut values = HashMap::from_iter([(None, vec![])]);

        let mut global_values = vec![];
        if let Some(global_env_vars) = environments_map.get(GLOBAL_TOP_LEVEL_INPUTS_NAME) {
            for (key, value) in global_env_vars.iter() {
                global_values.push((key.to_string(), Value::parse_and_default_to_string(value)));
            }
        };

        for (selector, inputs) in environments_map.iter() {
            if selector.eq(GLOBAL_TOP_LEVEL_INPUTS_NAME) {
                continue; // Skip global inputs, their values are added to all environments but should not be listed as an environment
            }
            let mut env_values = vec![];
            // Add global values to all environments
            for (key, value) in global_values.iter() {
                env_values.push((key.to_string(), value.clone()));
            }
            // _Then_ add the environment specific values, overwriting the global ones in the case of collisions
            for (key, value) in inputs.iter() {
                env_values.push((key.to_string(), Value::parse_and_default_to_string(value)));
            }
            environments.push(selector.to_string());
            values.insert(Some(selector.to_string()), env_values);
        }

        Self {
            current_environment: selector.clone().or(environments.get(0).map(|v| v.to_string())),
            environments,
            values,
        }
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
