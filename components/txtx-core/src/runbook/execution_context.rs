use kit::channel::unbounded;
use kit::types::frontend::ActionItemRequest;
use kit::types::frontend::ActionItemRequestType;
use kit::types::frontend::ActionItemStatus;
use kit::types::frontend::DisplayOutputRequest;
use kit::types::types::RunbookSupervisionContext;
use kit::types::wallets::SigningCommandsState;
use kit::types::ConstructDid;
use kit::uuid::Uuid;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use txtx_addon_kit::types::commands::CommandInputsEvaluationResult;
use txtx_addon_kit::types::commands::{CommandExecutionResult, CommandInstance};
use txtx_addon_kit::types::wallets::WalletInstance;

use crate::eval::perform_inputs_evaluation;
use crate::eval::CommandInputEvaluationStatus;
use crate::eval::EvaluationPassResult;

use super::RunbookWorkspaceContext;
use super::RuntimeContext;

#[derive(Debug, Clone)]
pub struct RunbookExecutionContext {
    /// Map of executable commands (input, output, action)
    pub commands_instances: HashMap<ConstructDid, CommandInstance>,
    /// Map of signing commands (wallet)
    pub signing_commands_instances: HashMap<ConstructDid, WalletInstance>,
    /// State of the signing commands states (stateful)
    pub signing_commands_state: Option<SigningCommandsState>,
    /// Results of commands executions
    pub commands_execution_results: HashMap<ConstructDid, CommandExecutionResult>,
    /// Results of commands inputs evaluations
    pub commands_inputs_evaluations_results: HashMap<ConstructDid, CommandInputsEvaluationResult>,
    /// Constructs depending on a given Construct.
    pub commands_dependencies: HashMap<ConstructDid, Vec<ConstructDid>>,
    /// Constructs depending on a given Construct performing signing.
    pub signing_commands_downstream_dependencies: Vec<(ConstructDid, Vec<ConstructDid>)>,
    /// Constructs depending on a given Construct being signed.
    pub signed_commands_upstream_dependencies: HashMap<ConstructDid, Vec<ConstructDid>>,
    /// Constructs depending on a given Construct being signed.
    pub signed_commands: HashSet<ConstructDid>,
    /// Commands execution order.
    pub order_for_commands_execution: Vec<ConstructDid>,
    /// Signing commands initialization order.
    pub order_for_signing_commands_initialization: Vec<ConstructDid>,
    /// Wether or not this running context is enabled
    pub execution_mode: RunbookExecutionMode,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunbookExecutionMode {
    Ignored,
    Partial(Vec<ConstructDid>),
    Full,
}

impl RunbookExecutionContext {
    pub fn new() -> Self {
        Self {
            commands_instances: HashMap::new(),
            signing_commands_instances: HashMap::new(),
            signing_commands_state: Some(SigningCommandsState::new()),
            commands_execution_results: HashMap::new(),
            commands_inputs_evaluations_results: HashMap::new(),
            commands_dependencies: HashMap::new(),
            signing_commands_downstream_dependencies: vec![],
            signed_commands_upstream_dependencies: HashMap::new(),
            signed_commands: HashSet::new(),
            order_for_commands_execution: vec![],
            order_for_signing_commands_initialization: vec![],
            execution_mode: RunbookExecutionMode::Ignored,
        }
    }

    pub fn is_signing_command_instantiated(&self, construct_did: &ConstructDid) -> bool {
        for (c, deps) in self.signing_commands_downstream_dependencies.iter() {
            if c.eq(&construct_did) && deps.len() > 0 {
                return true;
            }
        }
        false
    }

    pub fn collect_outputs_constructs_results(&self) -> BTreeMap<String, Vec<ActionItemRequest>> {
        let mut action_items = BTreeMap::new();

        for construct_did in self.order_for_commands_execution.iter() {
            let Some(command_instance) = self.commands_instances.get(&construct_did) else {
                // runtime_context.addons.index_command_instance(namespace, package_did, block)
                continue;
            };

            if command_instance
                .specification
                .name
                .to_lowercase()
                .eq("output")
            {
                let Some(execution_result) = self.commands_execution_results.get(&construct_did)
                else {
                    return action_items;
                };

                let Some(value) = execution_result.outputs.get("value") else {
                    unreachable!()
                };

                action_items
                    .entry(command_instance.get_group())
                    .or_insert_with(Vec::new)
                    .push(ActionItemRequest::new(
                        &Some(construct_did.clone()),
                        &command_instance.name,
                        None,
                        ActionItemStatus::Todo,
                        ActionItemRequestType::DisplayOutput(DisplayOutputRequest {
                            name: command_instance.name.to_string(),
                            description: None,
                            value: value.clone(),
                        }),
                        "output".into(),
                    ));
            }
        }
        action_items
    }

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
            // Is a command being considered? (vs signing_command, env, etc)
            let Some(command_instance) = self.commands_instances.get(&construct_did) else {
                continue;
            };

            // Construct was already executed
            if let Some(_) = self.commands_execution_results.get(&construct_did) {
                continue;
            };

            let addon_context_key = (
                command_instance.package_id.did(),
                command_instance.namespace.clone(),
            );
            let addon_defaults = workspace_context.get_addon_defaults(&addon_context_key);

            let input_evaluation_results =
                self.commands_inputs_evaluations_results.get(&construct_did);

            let mut cached_dependency_execution_results = HashMap::new();

            // Retrieve the construct_did of the inputs
            // Collect the outputs
            let references_expressions = command_instance
                .get_expressions_referencing_commands_from_inputs()
                .unwrap();

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

            let evaluated_inputs_res = perform_inputs_evaluation(
                command_instance,
                &cached_dependency_execution_results,
                &input_evaluation_results,
                &None,
                &command_instance.package_id,
                workspace_context,
                self,
                runtime_context,
            );

            let mut evaluated_inputs = match evaluated_inputs_res {
                Ok(result) => match result {
                    CommandInputEvaluationStatus::Complete(result) => result,
                    CommandInputEvaluationStatus::NeedsUserInteraction(result) => result,
                    CommandInputEvaluationStatus::Aborted(results, _) => results,
                },
                Err(mut diags) => {
                    pass_result.diagnostics.append(&mut diags);
                    continue;
                }
            };

            // Inject the evaluated inputs
            self.commands_inputs_evaluations_results
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
                addon_defaults.clone(),
                &mut self.signing_commands_instances,
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
                    pass_result.diagnostics.push(diag);
                    continue;
                }
            }

            self.commands_inputs_evaluations_results
                .insert(construct_did.clone(), evaluated_inputs.clone());

            let execution_result = {
                command_instance
                    .perform_execution(
                        &construct_did,
                        &evaluated_inputs,
                        addon_defaults.clone(),
                        &mut vec![],
                        &None,
                        &tx,
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
                    pass_result.diagnostics.push(diag);
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
}
