use kit::channel::unbounded;
use kit::hcl::Span;
use kit::indexmap::IndexMap;
use kit::types::diagnostics::Diagnostic;
use kit::types::frontend::ActionItemRequest;
use kit::types::frontend::ActionItemRequestType;
use kit::types::frontend::ActionItemStatus;
use kit::types::frontend::DisplayOutputRequest;
use kit::types::signers::SignersState;
use kit::types::types::RunbookSupervisionContext;
use kit::types::ConstructDid;
use kit::uuid::Uuid;
use std::collections::HashMap;
use std::collections::HashSet;
use txtx_addon_kit::types::commands::CommandInputsEvaluationResult;
use txtx_addon_kit::types::commands::{CommandExecutionResult, CommandInstance};
use txtx_addon_kit::types::signers::SignerInstance;

use crate::eval::perform_inputs_evaluation;
use crate::eval::CommandInputEvaluationStatus;
use crate::eval::EvaluationPassResult;

use super::RunbookWorkspaceContext;
use super::RuntimeContext;

#[derive(Debug, Clone)]
pub struct RunbookExecutionContext {
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

    pub fn collect_outputs_constructs_results(&self) -> IndexMap<String, Vec<ActionItemRequest>> {
        let mut action_items = IndexMap::new();

        for construct_did in self.order_for_commands_execution.iter() {
            let Some(command_instance) = self.commands_instances.get(&construct_did) else {
                // runtime_context.addons.index_command_instance(namespace, package_did, block)
                continue;
            };

            if command_instance.specification.name.to_lowercase().eq("output") {
                let Some(execution_result) = self.commands_execution_results.get(&construct_did)
                else {
                    return action_items;
                };
                let Some(input_evaluations) =
                    self.commands_inputs_evaluation_results.get(&construct_did)
                else {
                    return action_items;
                };

                let Some(value) = execution_result.outputs.get("value") else { unreachable!() };

                let description = input_evaluations
                    .inputs
                    .get_string("description")
                    .and_then(|d| Some(d.to_string()));

                action_items.entry(command_instance.get_group()).or_insert_with(Vec::new).push(
                    ActionItemRequest::new(
                        &Some(construct_did.clone()),
                        &command_instance.name,
                        None,
                        ActionItemStatus::Todo,
                        ActionItemRequestType::DisplayOutput(DisplayOutputRequest {
                            name: command_instance.name.to_string(),
                            description,
                            value: value.clone(),
                        }),
                        "output".into(),
                    ),
                );
            }
        }
        action_items
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
            // Is a command being considered? (vs signer, env, etc)
            let Some(command_instance) = self.commands_instances.get(&construct_did) else {
                continue;
            };

            // Construct was already executed
            if let Some(_) = self.commands_execution_results.get(&construct_did) {
                continue;
            };

            let construct_id = workspace_context.expect_construct_id(&construct_did);

            let addon_context_key =
                (command_instance.package_id.did(), command_instance.namespace.clone());
            let addon_defaults = workspace_context.get_addon_defaults(&addon_context_key);

            let input_evaluation_results =
                self.commands_inputs_evaluation_results.get(&construct_did);

            let mut cached_dependency_execution_results = HashMap::new();

            // Retrieve the construct_did of the inputs
            // Collect the outputs
            let references_expressions =
                command_instance.get_expressions_referencing_commands_from_inputs().unwrap();

            // For each input referencing another construct_did, we'll resolve the reference
            // and make sure we have the evaluation results, and seed a temporary, lighter map.
            // This step could probably be removed.
            for (_input, expr) in references_expressions.into_iter() {
                let res = workspace_context
                    .try_resolve_construct_reference_in_expression(
                        &command_instance.package_id,
                        &expr,
                    )
                    .unwrap();
                let Some((dependency, _, _)) = res else {
                    continue;
                };

                let evaluation_result_opt = self.commands_execution_results.get(&dependency);

                let Some(evaluation_result) = evaluation_result_opt else {
                    continue;
                };

                match cached_dependency_execution_results.get(&dependency) {
                    None => {
                        cached_dependency_execution_results
                            .insert(dependency, Ok(evaluation_result));
                    }
                    Some(Err(_)) => continue,
                    Some(Ok(_)) => {}
                }
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
            );

            let mut evaluated_inputs = match evaluated_inputs_res {
                Ok(result) => match result {
                    CommandInputEvaluationStatus::Complete(result) => result,
                    CommandInputEvaluationStatus::NeedsUserInteraction(result) => result,
                    CommandInputEvaluationStatus::Aborted(results, _) => results,
                },
                Err(diags) => {
                    pass_result.append_diagnostics(diags, &construct_id);
                    continue;
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
                continue;
            }

            if command_instance.specification.implements_signing_capability {
                continue;
            }

            // This time, we borrow a mutable reference
            let Some(command_instance) = self.commands_instances.get_mut(&construct_did) else {
                continue;
            };

            match command_instance.check_executability(
                &construct_did,
                &mut evaluated_inputs,
                &mut self.signers_instances,
                &None,
                supervision_context,
            ) {
                Ok(new_actions) => {
                    if new_actions.has_pending_actions() {
                        if let Some(deps) = self.commands_dependencies.get(&construct_did) {
                            for dep in deps.iter() {
                                unexecutable_nodes.insert(dep.clone());
                            }
                        }
                        continue;
                    }
                }
                Err(diag) => {
                    pass_result.push_diagnostic(&diag, &construct_id);
                    continue;
                }
            }

            self.commands_inputs_evaluation_results
                .insert(construct_did.clone(), evaluated_inputs.clone());

            let execution_result = {
                command_instance
                    .perform_execution(&construct_did, &evaluated_inputs, &mut vec![], &None, &tx)
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
                    pass_result.push_diagnostic(&diag, &construct_id);
                    continue;
                }
            };

            self.commands_execution_results
                .entry(construct_did)
                .or_insert_with(CommandExecutionResult::new)
                .append(&mut execution_result);
        }
        pass_result
    }

    pub async fn simulate_inputs_execution(
        &mut self,
        runtime_context: &RuntimeContext,
        workspace_context: &RunbookWorkspaceContext,
    ) -> Result<(), Diagnostic> {
        for (construct_did, command_instance) in self.commands_instances.iter() {
            let inputs_simulation_results =
                self.commands_inputs_evaluation_results.get(&construct_did);

            let cached_dependency_execution_results = HashMap::new();

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
            );

            let evaluated_inputs = match evaluated_inputs_res {
                Ok(result) => match result {
                    CommandInputEvaluationStatus::Complete(result) => result,
                    CommandInputEvaluationStatus::NeedsUserInteraction(result) => result,
                    CommandInputEvaluationStatus::Aborted(results, diags) => results,
                },
                Err(_d) => {
                    continue;
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
        }
        Ok(())
    }
}
