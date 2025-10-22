use txtx_addon_kit::hcl::structure::Attribute;
use txtx_addon_kit::indexmap::IndexMap;
use txtx_addon_kit::types::commands::{CommandExecutionResult, DependencyExecutionResultCache};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::{diagnostics::Diagnostic, types::Value};
use txtx_addon_kit::types::{Did, PackageId, RunbookId};

use crate::eval::{self, ExpressionEvaluationStatus};

use super::{
    RunbookExecutionContext, RunbookExecutionMode, RunbookGraphContext, RunbookWorkspaceContext,
    RuntimeContext,
};

#[derive(Clone, Debug)]
pub struct FlowContext {
    /// The name of the flow
    pub name: String,
    /// The description of the flow
    pub description: Option<String>,
    /// The resolution context contains all the data related to source code analysis and DAG construction
    pub graph_context: RunbookGraphContext,
    /// The execution context contains all the data related to the execution of the runbook
    pub execution_context: RunbookExecutionContext,
    /// The workspace context keeps track of packages and constructs reachable
    pub workspace_context: RunbookWorkspaceContext,
    /// The set of environment variables used during the execution
    pub top_level_inputs: ValueStore,
    /// The evaluated inputs to this flow
    pub evaluated_inputs: ValueStore,
}

impl FlowContext {
    pub fn new(name: &str, runbook_id: &RunbookId, top_level_inputs: &ValueStore) -> Self {
        let workspace_context = RunbookWorkspaceContext::new(runbook_id.clone());
        let graph_context = RunbookGraphContext::new();
        let execution_context = RunbookExecutionContext::new();
        let mut running_context = Self {
            name: name.to_string(),
            description: None,
            workspace_context,
            graph_context,
            execution_context,
            top_level_inputs: top_level_inputs.clone(),
            evaluated_inputs: ValueStore::new(name, &Did::zero()),
        };
        running_context.index_top_level_inputs(top_level_inputs);
        running_context
    }

    pub fn is_enabled(&self) -> bool {
        !self.execution_context.execution_mode.eq(&RunbookExecutionMode::Ignored)
    }

    /// Each key/value pair in the provided [ValueStore] is indexed in the [FlowContext]'s [RunbookWorkspaceContext],
    /// [RunbookGraphContext], and [RunbookExecutionContext]' `command_execution_results`.
    pub fn index_top_level_inputs(&mut self, inputs_set: &ValueStore) {
        for (key, value) in inputs_set.iter() {
            let construct_did = self.workspace_context.index_top_level_input(key, value);
            self.graph_context.index_top_level_input(&construct_did);
            let mut result = CommandExecutionResult::new();
            result.outputs.insert("value".into(), value.clone());
            self.execution_context.commands_execution_results.insert(construct_did, result);
        }
    }

    pub fn index_flow_inputs_from_attributes(
        &mut self,
        attributes: &Vec<Attribute>,
        dependencies_execution_results: &DependencyExecutionResultCache,
        package_id: &PackageId,
        workspace_context: &RunbookWorkspaceContext,
        execution_context: &RunbookExecutionContext,
        runtime_context: &RuntimeContext,
    ) -> Result<(), Diagnostic> {
        for attr in attributes.into_iter() {
            let res = eval::eval_expression(
                &attr.value,
                &dependencies_execution_results,
                &package_id,
                &workspace_context,
                &execution_context,
                &runtime_context,
            )
            .map_err(|e| e)?;

            match res {
                ExpressionEvaluationStatus::CompleteOk(value) => {
                    if attr.key.to_string().eq("description") {
                        self.description = Some(value.to_string());
                    } else {
                        self.index_flow_input(&attr.key, value, &package_id);
                    }
                }
                ExpressionEvaluationStatus::DependencyNotComputed => {
                    return Err(Diagnostic::error_from_string(format!(
                        "flow '{}': unable to evaluate input {}",
                        self.name,
                        attr.key.to_string()
                    )))
                }
                ExpressionEvaluationStatus::CompleteErr(e) => {
                    return Err(Diagnostic::error_from_string(format!(
                        "flow '{}': unable to evaluate input {}: {}",
                        self.name,
                        attr.key.to_string(),
                        e.message
                    )))
                }
            }
        }
        Ok(())
    }

    pub fn index_flow_input(&mut self, key: &str, value: Value, package_id: &PackageId) {
        let construct_id =
            self.workspace_context.index_flow_input(key, package_id, &mut self.graph_context);
        self.evaluated_inputs.insert(key.to_string(), value.clone());
        // self.graph_context.index_top_level_input(&construct_did);
        let mut result = CommandExecutionResult::new();
        result.outputs.insert("value".into(), value);

        // result.outputs.insert(key.into(), value);
        self.execution_context.commands_execution_results.insert(construct_id.did(), result);
    }

    pub fn sorted_evaluated_inputs_fingerprints(&self) -> IndexMap<String, Did> {
        let mut inputs_store = self.evaluated_inputs.inputs.store.clone();
        inputs_store.sort_keys();
        inputs_store.into_iter().map(|(k, v)| (k, v.compute_fingerprint())).collect()
    }
}
