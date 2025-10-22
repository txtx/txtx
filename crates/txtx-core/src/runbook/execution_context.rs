use kit::constants::{ActionItemKey, DocumentationKey};
use kit::types::commands::ConstructInstance;
use kit::types::frontend::DisplayErrorLogRequest;
use kit::types::AuthorizationContext;
use std::collections::HashMap;
use std::collections::HashSet;
use txtx_addon_kit::channel::unbounded;
use txtx_addon_kit::channel::Sender;
use txtx_addon_kit::hcl::Span;
use txtx_addon_kit::indexmap::IndexMap;
use txtx_addon_kit::types::commands::add_ctx_to_diag;
use txtx_addon_kit::types::commands::add_ctx_to_embedded_runbook_diag;
use txtx_addon_kit::types::commands::CommandInputsEvaluationResult;
use txtx_addon_kit::types::commands::DependencyExecutionResultCache;
use txtx_addon_kit::types::commands::{CommandExecutionResult, CommandInstance};
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::types::embedded_runbooks::EmbeddedRunbookInstance;
use txtx_addon_kit::types::frontend::ActionItemRequest;
use txtx_addon_kit::types::frontend::ActionItemRequestType;
use txtx_addon_kit::types::frontend::BlockEvent;
use txtx_addon_kit::types::frontend::DisplayOutputRequest;
use txtx_addon_kit::types::signers::SignerInstance;
use txtx_addon_kit::types::signers::SignersState;
use txtx_addon_kit::types::stores::AddonDefaults;
use txtx_addon_kit::types::types::ObjectType;
use txtx_addon_kit::types::types::RunbookSupervisionContext;
use txtx_addon_kit::types::types::Value;
use txtx_addon_kit::types::AddonInstance;
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::types::EvaluatableInput;
use txtx_addon_kit::uuid::Uuid;

use crate::eval::perform_inputs_evaluation;
use crate::eval::CommandInputEvaluationStatus;
use crate::eval::EvaluationPassResult;
use crate::eval::LoopEvaluationResult;

use super::diffing_context::RunbookFlowSnapshot;
use super::diffing_context::ValuePostEvaluation;
use super::RunbookWorkspaceContext;
use super::RuntimeContext;

#[derive(Debug, Clone)]
pub struct RunbookExecutionContext {
    /// Map of addon instances (addon "evm" { ... })
    pub addon_instances: HashMap<ConstructDid, AddonInstance>,
    /// Map of embedded runbooks
    pub embedded_runbooks: HashMap<ConstructDid, EmbeddedRunbookInstance>,
    /// Map of executable commands (input, output, action)
    pub commands_instances: HashMap<ConstructDid, CommandInstance>,
    /// Map of signing commands (signer)
    pub signers_instances: HashMap<ConstructDid, SignerInstance>,
    /// State of the signing commands states (stateful)
    pub signers_state: Option<SignersState>,
    /// Results of commands executions
    pub commands_execution_results: HashMap<ConstructDid, CommandExecutionResult>,
    /// Results of commands inputs evaluation
    pub commands_inputs_evaluation_results: HashMap<ConstructDid, CommandInputsEvaluationResult>,
    /// Constructs depending on a given Construct.
    pub commands_dependencies: HashMap<ConstructDid, Vec<ConstructDid>>,
    /// Constructs depending on a given Construct performing signing.
    pub signers_downstream_dependencies: Vec<(ConstructDid, Vec<ConstructDid>)>,
    /// Constructs depending on a given Construct being signed.
    pub signed_commands_upstream_dependencies: HashMap<ConstructDid, Vec<ConstructDid>>,
    /// Constructs depending on a given Construct being signed.
    pub signed_commands: HashSet<ConstructDid>,
    /// Commands execution order.
    pub order_for_commands_execution: Vec<ConstructDid>,
    /// Signing commands initialization order.
    pub order_for_signers_initialization: Vec<ConstructDid>,
    /// Wether or not this running context is enabled
    pub execution_mode: RunbookExecutionMode,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunbookExecutionMode {
    Ignored,
    Partial(Vec<ConstructDid>),
    Full,
    FullFailed,
}

impl RunbookExecutionContext {
    pub fn new() -> Self {
        Self {
            addon_instances: HashMap::new(),
            embedded_runbooks: HashMap::new(),
            commands_instances: HashMap::new(),
            signers_instances: HashMap::new(),
            signers_state: Some(SignersState::new()),
            commands_execution_results: HashMap::new(),
            commands_inputs_evaluation_results: HashMap::new(),
            commands_dependencies: HashMap::new(),
            signers_downstream_dependencies: vec![],
            signed_commands_upstream_dependencies: HashMap::new(),
            signed_commands: HashSet::new(),
            order_for_commands_execution: vec![],
            order_for_signers_initialization: vec![],
            execution_mode: RunbookExecutionMode::Ignored,
        }
    }

    pub fn is_signer_instantiated(&self, construct_did: &ConstructDid) -> bool {
        for (c, deps) in self.signers_downstream_dependencies.iter() {
            if c.eq(&construct_did) && deps.len() > 0 {
                return true;
            }
        }
        false
    }

    pub fn collect_outputs_constructs_results(
        &self,
        auth_context: &AuthorizationContext,
    ) -> IndexMap<String, Vec<ActionItemRequest>> {
        let mut action_items = IndexMap::new();

        for construct_did in self.order_for_commands_execution.iter() {
            if let Some(command_instance) = self.commands_instances.get(&construct_did) {
                match self.collect_command_instance_output(
                    &construct_did,
                    command_instance,
                    &mut action_items,
                    auth_context,
                ) {
                    LoopEvaluationResult::Continue => continue,
                    LoopEvaluationResult::Bail => return action_items,
                }
            };
            if let Some(embedded_runbook) = self.embedded_runbooks.get(&construct_did) {
                for (construct_did, command_instance) in embedded_runbook
                    .specification
                    .static_execution_context
                    .commands_instances
                    .iter()
                {
                    let res = self.collect_command_instance_output(
                        construct_did,
                        command_instance,
                        &mut action_items,
                        auth_context,
                    );
                    match res {
                        LoopEvaluationResult::Continue => continue,
                        LoopEvaluationResult::Bail => return action_items,
                    }
                }
            }
        }
        action_items
    }

    pub fn collect_command_instance_output(
        &self,
        construct_did: &ConstructDid,
        command_instance: &CommandInstance,
        action_items: &mut IndexMap<String, Vec<ActionItemRequest>>,
        auth_context: &AuthorizationContext,
    ) -> LoopEvaluationResult {
        if command_instance.specification.name.to_lowercase().eq(ActionItemKey::Output.as_ref()) {
            let Some(execution_result) = self.commands_execution_results.get(&construct_did) else {
                return LoopEvaluationResult::Continue;
            };
            let Some(input_evaluations) =
                self.commands_inputs_evaluation_results.get(&construct_did)
            else {
                return LoopEvaluationResult::Continue;
            };

            let Some(value) = execution_result.outputs.get("value") else {
                return LoopEvaluationResult::Continue;
            };

            let description =
                input_evaluations.inputs.get_string(DocumentationKey::Description).and_then(|d| Some(d.to_string()));
            let markdown = match input_evaluations.inputs.get_markdown(&auth_context) {
                Ok(md) => md,
                Err(e) => {
                    action_items.entry(command_instance.get_group()).or_insert_with(Vec::new).push(
                        ActionItemRequestType::DisplayErrorLog(DisplayErrorLogRequest {
                            diagnostic: diagnosed_error!(
                                "Error displaying output markdown documentation for `{}`: {}",
                                command_instance.name,
                                e.message
                            ),
                        })
                        .to_request(&command_instance.name, ActionItemKey::Output)
                        .with_construct_did(construct_did),
                    );
                    None
                }
            };

            action_items.entry(command_instance.get_group()).or_insert_with(Vec::new).push(
                ActionItemRequestType::DisplayOutput(DisplayOutputRequest {
                    name: command_instance.name.to_string(),
                    description: description.clone(),
                    value: value.clone(),
                })
                .to_request(&command_instance.name, ActionItemKey::Output)
                .with_construct_did(construct_did)
                .with_some_description(description)
                .with_some_markdown(markdown),
            );
        }
        LoopEvaluationResult::Continue
    }

    // During the simulation, our goal is to evaluate as many input evaluations as possible.
    //
    //
    pub async fn simulate_execution(
        &mut self,
        runtime_context: &RuntimeContext,
        workspace_context: &RunbookWorkspaceContext,
        supervision_context: &RunbookSupervisionContext,
        constructs_dids_frontier: &HashSet<ConstructDid>,
    ) -> EvaluationPassResult {
        let mut pass_result = EvaluationPassResult::new(&Uuid::new_v4());

        let mut unexecutable_nodes: HashSet<ConstructDid> = HashSet::new();

        let ordered_constructs = self.order_for_commands_execution.clone();

        let (tx, _rx) = unbounded();

        for construct_did in ordered_constructs.into_iter() {
            if let Some(_) = self.commands_instances.get(&construct_did) {
                match self
                    .simulate_command_instance(
                        &construct_did,
                        &mut pass_result,
                        &mut unexecutable_nodes,
                        runtime_context,
                        workspace_context,
                        supervision_context,
                        constructs_dids_frontier,
                        &tx,
                    )
                    .await
                {
                    LoopEvaluationResult::Continue => continue,
                    LoopEvaluationResult::Bail => return pass_result,
                };
            };
            // if let Some(_) = self.embedded_runbooks.get(&construct_did) {
            //     match self
            //         .simulate_embedded_runbook(
            //             &construct_did,
            //             &mut pass_result,
            //             &mut unexecutable_nodes,
            //             runtime_context,
            //             workspace_context,
            //             supervision_context,
            //             constructs_dids_frontier,
            //             &tx,
            //         )
            //         .await
            //     {
            //         LoopEvaluationResult::Continue => continue,
            //         LoopEvaluationResult::Bail => return pass_result,
            //     };
            // };
        }
        pass_result
    }

    async fn simulate_command_instance(
        &mut self,
        construct_did: &ConstructDid,
        pass_result: &mut EvaluationPassResult,
        unexecutable_nodes: &mut HashSet<ConstructDid>,
        runtime_context: &RuntimeContext,
        workspace_context: &RunbookWorkspaceContext,
        supervision_context: &RunbookSupervisionContext,
        constructs_dids_frontier: &HashSet<ConstructDid>,
        tx: &Sender<BlockEvent>,
    ) -> LoopEvaluationResult {
        // Is a command being considered? (vs signer, env, etc)
        let Some(command_instance) = self.commands_instances.get(&construct_did) else {
            return LoopEvaluationResult::Continue;
        };
        // Construct was already executed
        if let Some(_) = self.commands_execution_results.get(&construct_did) {
            return LoopEvaluationResult::Continue;
        };

        let add_ctx_to_diag = add_ctx_to_diag(
            "command".to_string(),
            command_instance.specification.matcher.clone(),
            command_instance.name.clone(),
            command_instance.namespace.clone(),
        );

        let construct_id = workspace_context.expect_construct_id(&construct_did);

        let addon_context_key =
            (command_instance.package_id.did(), command_instance.namespace.clone());
        let addon_defaults = workspace_context.get_addon_defaults(&addon_context_key);

        let input_evaluation_results = self.commands_inputs_evaluation_results.get(&construct_did);

        let mut cached_dependency_execution_results = DependencyExecutionResultCache::new();

        // Retrieve the construct_did of the inputs
        // Collect the outputs
        let references_expressions =
            command_instance.get_expressions_referencing_commands_from_inputs();

        // For each input referencing another construct_did, we'll resolve the reference
        // and make sure we have the evaluation results, and seed a temporary, lighter map.
        // This step could probably be removed.
        for (_input, expr) in references_expressions.into_iter() {
            let Some((dependency, _, _)) = workspace_context
                .try_resolve_construct_reference_in_expression(&command_instance.package_id, &expr)
                .unwrap()
            else {
                continue;
            };

            let Some(evaluation_result) = self.commands_execution_results.get(&dependency) else {
                continue;
            };

            match cached_dependency_execution_results.merge(&dependency, evaluation_result) {
                Ok(_) => (),
                Err(_) => {
                    continue;
                }
            };
        }

        // After this evaluation, commands should be able to tweak / override
        let evaluated_inputs_res = perform_inputs_evaluation(
            command_instance,
            &cached_dependency_execution_results,
            &input_evaluation_results,
            &addon_defaults,
            &None,
            &command_instance.package_id,
            workspace_context,
            self,
            runtime_context,
            false,
            false,
        );

        let mut evaluated_inputs = match evaluated_inputs_res {
            Ok(result) => match result {
                CommandInputEvaluationStatus::Complete(result) => result,
                CommandInputEvaluationStatus::NeedsUserInteraction(result) => result,
                CommandInputEvaluationStatus::Aborted(results, _) => results,
            },
            Err(diags) => {
                pass_result.append_diagnostics(diags, &construct_id, &add_ctx_to_diag);
                return LoopEvaluationResult::Continue;
            }
        };

        // Inject the evaluated inputs
        self.commands_inputs_evaluation_results
            .insert(construct_did.clone(), evaluated_inputs.clone());

        // Did we reach the frontier?
        if constructs_dids_frontier.contains(&construct_did) {
            if let Some(deps) = self.commands_dependencies.get(&construct_did) {
                for dep in deps.iter() {
                    unexecutable_nodes.insert(dep.clone());
                }
            }
            return LoopEvaluationResult::Continue;
        }

        if command_instance.specification.implements_signing_capability {
            return LoopEvaluationResult::Continue;
        }

        let executions_for_action =
            match command_instance.prepare_nested_execution(&construct_did, &evaluated_inputs) {
                Ok(executions) => executions,
                Err(diag) => {
                    pass_result.push_diagnostic(&diag, &construct_id, &add_ctx_to_diag);
                    return LoopEvaluationResult::Bail;
                }
            };

        // This time, we borrow a mutable reference
        let Some(command_instance) = self.commands_instances.get_mut(&construct_did) else {
            return LoopEvaluationResult::Continue;
        };

        for (nested_construct_did, nested_evaluation_values) in executions_for_action.iter() {
            if let Some(_) = self.commands_execution_results.get(&nested_construct_did) {
                continue;
            }

            match command_instance.check_executability(
                &construct_did,
                &nested_evaluation_values,
                &mut evaluated_inputs,
                &mut self.signers_instances,
                &None,
                supervision_context,
                &runtime_context.authorization_context,
            ) {
                Ok(new_actions) => {
                    if new_actions.has_pending_actions() {
                        if let Some(deps) = self.commands_dependencies.get(&construct_did) {
                            for dep in deps.iter() {
                                unexecutable_nodes.insert(dep.clone());
                            }
                        }
                        return LoopEvaluationResult::Continue;
                    }
                }
                Err(diag) => {
                    pass_result.push_diagnostic(&diag, &construct_id, &add_ctx_to_diag);
                    return LoopEvaluationResult::Continue;
                }
            }
            self.commands_inputs_evaluation_results
                .insert(construct_did.clone(), evaluated_inputs.clone());

            let execution_result = {
                command_instance
                    .perform_execution(
                        &construct_did,
                        &nested_evaluation_values,
                        &evaluated_inputs,
                        &mut vec![],
                        &None,
                        &tx,
                        &runtime_context.authorization_context,
                    )
                    .await
            };

            let execution_result = match execution_result {
                Ok(result) => Ok(result),
                Err(e) => {
                    if let Some(deps) = self.commands_dependencies.get(&construct_did) {
                        for dep in deps.iter() {
                            unexecutable_nodes.insert(dep.clone());
                        }
                    }
                    Err(e)
                }
            };

            let mut execution_result = match execution_result {
                Ok(res) => res,
                Err(diag) => {
                    pass_result.push_diagnostic(&diag, &construct_id, &add_ctx_to_diag);
                    return LoopEvaluationResult::Continue;
                }
            };
            self.commands_execution_results
                .entry(construct_did.clone())
                .or_insert_with(CommandExecutionResult::new)
                .append(&mut execution_result);
        }

        let res = command_instance.aggregate_nested_execution_results(
            &construct_did,
            &executions_for_action,
            &self.commands_execution_results,
        );

        match res {
            Ok(result) => {
                self.commands_execution_results.insert(construct_did.clone(), result);
            }
            Err(diag) => {
                pass_result.push_diagnostic(&diag, &construct_id, &add_ctx_to_diag);
                return LoopEvaluationResult::Continue;
            }
        }

        LoopEvaluationResult::Continue
    }

    pub async fn simulate_embedded_runbook(
        &mut self,
        construct_did: &ConstructDid,
        pass_result: &mut EvaluationPassResult,
        _unexecutable_nodes: &mut HashSet<ConstructDid>,
        runtime_context: &RuntimeContext,
        workspace_context: &RunbookWorkspaceContext,
        _supervision_context: &RunbookSupervisionContext,
        _constructs_dids_frontier: &HashSet<ConstructDid>,
        _tx: &Sender<BlockEvent>,
    ) -> LoopEvaluationResult {
        let Some(embedded_runbook) = self.embedded_runbooks.get(&construct_did) else {
            return LoopEvaluationResult::Continue;
        };
        // Construct was already executed
        if let Some(_) = self.commands_execution_results.get(&construct_did) {
            return LoopEvaluationResult::Continue;
        };

        let add_ctx_to_diag = add_ctx_to_embedded_runbook_diag(embedded_runbook.name.clone());

        let construct_id = workspace_context.expect_construct_id(&construct_did);

        let input_evaluation_results = self.commands_inputs_evaluation_results.get(&construct_did);

        let mut cached_dependency_execution_results = DependencyExecutionResultCache::new();

        // Retrieve the construct_did of the inputs
        // Collect the outputs
        let references_expressions =
            embedded_runbook.get_expressions_referencing_commands_from_inputs();

        // For each input referencing another construct_did, we'll resolve the reference
        // and make sure we have the evaluation results, and seed a temporary, lighter map.
        // This step could probably be removed.
        for (_input, expr) in references_expressions.into_iter() {
            let Some((dependency, _, _)) = workspace_context
                .try_resolve_construct_reference_in_expression(&embedded_runbook.package_id, &expr)
                .unwrap()
            else {
                continue;
            };

            let Some(evaluation_result) = self.commands_execution_results.get(&dependency) else {
                continue;
            };

            match cached_dependency_execution_results.merge(&dependency, evaluation_result) {
                Ok(_) => (),
                Err(_) => continue,
            };
        }

        // After this evaluation, commands should be able to tweak / override
        let evaluated_inputs_res = perform_inputs_evaluation(
            embedded_runbook,
            &cached_dependency_execution_results,
            &input_evaluation_results,
            &AddonDefaults::new("tmp"),
            &None,
            &embedded_runbook.package_id,
            workspace_context,
            self,
            runtime_context,
            false,
            false,
        );

        let evaluated_inputs = match evaluated_inputs_res {
            Ok(result) => match result {
                CommandInputEvaluationStatus::Complete(result) => result,
                CommandInputEvaluationStatus::NeedsUserInteraction(result) => result,
                CommandInputEvaluationStatus::Aborted(results, _) => results,
            },
            Err(diags) => {
                pass_result.append_diagnostics(diags, &construct_id, &add_ctx_to_diag);
                return LoopEvaluationResult::Continue;
            }
        };

        // Inject the evaluated inputs
        self.commands_inputs_evaluation_results
            .insert(construct_did.clone(), evaluated_inputs.clone());

        LoopEvaluationResult::Continue
    }

    pub async fn simulate_inputs_execution(
        &mut self,
        runtime_context: &RuntimeContext,
        workspace_context: &RunbookWorkspaceContext,
    ) -> Result<(), Diagnostic> {
        let command_instances = self.commands_instances.clone();
        for (construct_did, command_instance) in command_instances.iter() {
            self.simulate_command_instance_inputs_execution(
                construct_did,
                command_instance,
                runtime_context,
                workspace_context,
            )
            .await?;
        }

        // let embedded_runbooks = self.embedded_runbooks.clone();
        // for (construct_did, embedded_runbook) in embedded_runbooks.iter() {
        //     self.simulate_embedded_runbook_instance_inputs_execution(
        //         construct_did,
        //         embedded_runbook,
        //         runtime_context,
        //         workspace_context,
        //     )
        //     .await?;
        // }
        Ok(())
    }

    async fn simulate_command_instance_inputs_execution(
        &mut self,
        construct_did: &ConstructDid,
        command_instance: &CommandInstance,
        runtime_context: &RuntimeContext,
        workspace_context: &RunbookWorkspaceContext,
    ) -> Result<(), Diagnostic> {
        let inputs_simulation_results = self.commands_inputs_evaluation_results.get(&construct_did);

        let cached_dependency_execution_results = DependencyExecutionResultCache::new();

        let package_id = command_instance.package_id.clone();
        let construct_id = &workspace_context.expect_construct_id(&construct_did);
        let addon_context_key = (package_id.did(), command_instance.namespace.clone());
        let addon_defaults = workspace_context.get_addon_defaults(&addon_context_key);

        let evaluated_inputs_res = perform_inputs_evaluation(
            command_instance,
            &cached_dependency_execution_results,
            &inputs_simulation_results,
            &addon_defaults,
            &None,
            &command_instance.package_id,
            workspace_context,
            self,
            runtime_context,
            true,
            false,
        );

        let evaluated_inputs = match evaluated_inputs_res {
            Ok(result) => match result {
                CommandInputEvaluationStatus::Complete(result) => result,
                CommandInputEvaluationStatus::NeedsUserInteraction(result) => result,
                CommandInputEvaluationStatus::Aborted(results, _diags) => results,
            },
            Err(_d) => {
                // if we failed to evaluated inputs during simulation, swallow the error, but ensure we
                // re-evaluate all of the inputs in the actual execution by adding each required input
                // to the unevaluated_inputs map.
                let mut inputs = CommandInputsEvaluationResult::new(
                    &command_instance.name,
                    &addon_defaults.store,
                );
                for input in command_instance.specification.inputs.iter() {
                    if input.optional {
                        continue;
                    }
                    inputs.unevaluated_inputs.insert(input.name.clone(), None);
                }

                inputs
            }
        };

        let post_processed_inputs = command_instance
            .post_process_inputs_evaluations(evaluated_inputs.clone())
            .await
            .map_err(|d| {
                d.location(&construct_id.construct_location)
                    .set_span_range(command_instance.block.span())
            })?;

        self.commands_inputs_evaluation_results
            .insert(construct_did.clone(), post_processed_inputs);

        Ok(())
    }

    pub async fn simulate_embedded_runbook_instance_inputs_execution(
        &mut self,
        construct_did: &ConstructDid,
        embedded_runbook: &EmbeddedRunbookInstance,
        runtime_context: &RuntimeContext,
        workspace_context: &RunbookWorkspaceContext,
    ) -> Result<(), Diagnostic> {
        let inputs_simulation_results = self.commands_inputs_evaluation_results.get(&construct_did);

        let cached_dependency_execution_results = DependencyExecutionResultCache::new();

        let package_id = embedded_runbook.package_id.clone();
        let addon_defaults = &AddonDefaults::new("tmp");

        let evaluated_inputs_res = perform_inputs_evaluation(
            embedded_runbook,
            &cached_dependency_execution_results,
            &inputs_simulation_results,
            &addon_defaults,
            &None,
            &package_id,
            workspace_context,
            self,
            runtime_context,
            true,
            false,
        );

        let evaluated_inputs = match evaluated_inputs_res {
            Ok(result) => match result {
                CommandInputEvaluationStatus::Complete(result) => result,
                CommandInputEvaluationStatus::NeedsUserInteraction(result) => result,
                CommandInputEvaluationStatus::Aborted(results, _diags) => results,
            },
            Err(_d) => {
                // if we failed to evaluated inputs during simulation, swallow the error, but ensure we
                // re-evaluate all of the inputs in the actual execution by adding each required input
                // to the unevaluated_inputs map.
                let mut inputs = CommandInputsEvaluationResult::new(
                    &embedded_runbook.name,
                    &addon_defaults.store,
                );
                for input in embedded_runbook.specification.inputs.iter() {
                    inputs.unevaluated_inputs.insert(input.name(), None);
                }
                inputs
            }
        };

        self.commands_inputs_evaluation_results.insert(construct_did.clone(), evaluated_inputs);

        Ok(())
    }

    /// Takes a [RunbookFlowSnapshot] and applies the inputs to the `commands_inputs_evaluation_results` field
    /// and the outputs to the `commands_execution_results` field of the associated construct in the [RunbookExecutionContext].
    /// If an input or output value from the snapshot is already found in the simulation results, it will be ignored.
    pub fn apply_snapshot_to_execution_context(
        &mut self,
        snapshot: &RunbookFlowSnapshot,
        workspace_context: &RunbookWorkspaceContext,
    ) -> Result<(), Diagnostic> {
        for (construct_did, command_snapshot) in snapshot.commands.iter() {
            let Some(command_instance) = self.commands_instances.get_mut(&construct_did) else {
                continue;
            };

            let addon_context_key =
                (command_instance.package_id.did(), command_instance.namespace.clone());
            let addon_defaults = workspace_context.get_addon_defaults(&addon_context_key);

            let mut execution_result = self
                .commands_execution_results
                .get(&construct_did)
                .cloned()
                .unwrap_or(CommandExecutionResult::new());

            let mut inputs_evaluation_result =
                self.commands_inputs_evaluation_results.get(&construct_did).cloned().unwrap_or(
                    CommandInputsEvaluationResult::new(
                        &command_instance.name,
                        &addon_defaults.store,
                    ),
                );

            for (input_name, input_value_snapshot) in command_snapshot.inputs.iter() {
                // if our existing simulation results _don't_ have a value for this input that exists in the snapshot,
                // and this input is a actually a valid input for the command, then we'll add it to the simulation results.
                if inputs_evaluation_result.inputs.get_value(&input_name).is_none()
                    && command_instance.specification.inputs.iter().any(|i| i.name.eq(input_name))
                {
                    let value = match &input_value_snapshot.value_post_evaluation {
                        ValuePostEvaluation::Value(value) => value.clone(),
                        ValuePostEvaluation::ObjectValue(index_map) => Value::object(
                            index_map.iter().map(|(k, (v, _))| (k.clone(), v.clone())).collect(),
                        ),
                        ValuePostEvaluation::MapValue(index_maps) => Value::array(
                            index_maps
                                .iter()
                                .map(|index_map| {
                                    Value::object(
                                        index_map
                                            .iter()
                                            .map(|(k, (v, _))| (k.clone(), v.clone()))
                                            .collect(),
                                    )
                                })
                                .collect::<Vec<Value>>(),
                        ),
                    };

                    inputs_evaluation_result.inputs.insert(&input_name, value);
                }
            }

            for (output_name, output_value_snapshot) in command_snapshot.outputs.iter() {
                // if our existing simulation results _don't_ have a value for this input that exists in the snapshot,
                // and this input is a actually a valid input for the command, then we'll add it to the simulation results.
                if execution_result.outputs.get(output_name).is_none()
                    && command_instance.specification.outputs.iter().any(|i| i.name.eq(output_name))
                {
                    execution_result
                        .outputs
                        .insert(output_name.clone(), output_value_snapshot.value.clone());
                }
            }

            self.commands_execution_results.insert(construct_did.clone(), execution_result);
            self.commands_inputs_evaluation_results
                .insert(construct_did.clone(), inputs_evaluation_result);
        }
        Ok(())
    }

    pub fn construct_did_is_signed_or_signed_upstream(&self, construct_did: &ConstructDid) -> bool {
        self.signed_commands.contains(construct_did)
            || self
                .signed_commands_upstream_dependencies
                .iter()
                .any(|(_signed, upstream)| upstream.contains(construct_did))
    }

    /// Takes a [HashMap<ConstructDid, CommandInputsEvaluationResult>] and adds each of the entries to the `commands_inputs_evaluation_results` of [self].
    /// If the construct_did is already found in the `commands_inputs_evaluation_results` field, the inputs will be appended to the existing inputs without overriding any existing input values.
    pub fn append_command_inputs_evaluation_results_no_override(
        &mut self,
        source_inputs: &HashMap<ConstructDid, CommandInputsEvaluationResult>,
    ) {
        for (source_construct_did, source_results) in source_inputs.iter() {
            if let Some(evaluated_inputs) =
                self.commands_inputs_evaluation_results.get_mut(source_construct_did)
            {
                evaluated_inputs.inputs.append_no_override(&source_results.inputs);
            } else {
                self.commands_inputs_evaluation_results
                    .insert(source_construct_did.clone(), source_results.clone());
            }
        }
    }

    /// Takes a [HashMap<ConstructDid, CommandExecutionResult>] and iterates over it, calling [self].append_command_execution_result for each entry.
    pub fn append_commands_execution_results(
        &mut self,
        source_results: &HashMap<ConstructDid, CommandExecutionResult>,
    ) {
        for (source_construct_did, source_result) in source_results.iter() {
            self.append_commands_execution_result(source_construct_did, source_result);
        }
    }

    /// 1. Inserts the source result into the `commands_execution_results` of [self].
    ///     If the `source_construct_did` is already found in the `commands_execution_results` field, the outputs will be appended to the existing outputs, overriding any existing output values.
    /// 2. Checks if the `source_construct_did` is a construct in the `embedded_runbooks` of [self].
    ///     If so, the outputs of the source construct will be added to the outputs of the embedded runbook construct. The outputs will be stored as results in the embedded runbook construct's execution results in the form:
    ///    ```ignore
    ///     Value::object({ "construct_type": Value::object({ "construct_name": value }) })
    ///     ```
    ///
    ///     For example, if the embedded construct has
    ///     ```ignore
    ///     action "deploy" "evm::deploy_contract" {
    ///         ...
    ///     }
    ///     ```
    ///
    ///     The embedded construct will have an output:
    ///     ```ignore
    ///     Value::object({ "action": Value::object({ "deploy": value }) })
    ///     ```
    pub fn append_commands_execution_result(
        &mut self,
        source_construct_did: &ConstructDid,
        source_result: &CommandExecutionResult,
    ) {
        self.commands_execution_results
            .entry(source_construct_did.clone())
            .and_modify(|execution_result| {
                execution_result.apply(&source_result);
            })
            .or_insert(source_result.clone());

        for (embedded_runbook_did, embedded_runbook_instance) in self.embedded_runbooks.iter() {
            let Some(construct_id) = embedded_runbook_instance
                .specification
                .static_workspace_context
                .constructs
                .get(&source_construct_did)
            else {
                continue;
            };

            let value = ObjectType::from(source_result.outputs.iter().map(|(k, v)| (k, v.clone())))
                .to_value();

            self.commands_execution_results
                // try to get execution results for this embedded runbook id
                .entry(embedded_runbook_did.clone())
                // if we have some, we'll update them to include the results from its child construct's execution
                .and_modify(|execution_results| {
                    execution_results
                        .outputs
                        // check if we have any outputs for this construct type
                        .entry(construct_id.construct_type.clone())
                        // if we do, we'll update them to include the results from its child construct's execution
                        .and_modify(|object_value| {
                            object_value.as_object_mut().map(|object_props| {
                                object_props
                                    .insert(construct_id.construct_name.clone(), value.clone())
                            });
                        })
                        // if we don't, we'll create a new object value and insert the results from its child construct's execution
                        .or_insert(
                            ObjectType::from(vec![(&construct_id.construct_name, value.clone())])
                                .to_value(),
                        );
                })
                // if we don't have any execution results for this embedded runbook id, we'll create a new one
                .or_insert_with(|| {
                    let mut res = CommandExecutionResult::new();
                    res.insert(
                        &construct_id.construct_type,
                        ObjectType::from(vec![(&construct_id.construct_name, value.clone())])
                            .to_value(),
                    );
                    res
                });
        }
    }

    pub fn get_commands_implementing_cloud_service(&self) -> Vec<&CommandInstance> {
        self.commands_instances
            .values()
            .filter(|instance| instance.specification.implements_cloud_service)
            .collect::<Vec<_>>()
    }
}
