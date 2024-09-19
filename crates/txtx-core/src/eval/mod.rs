use crate::runbook::{
    get_source_context_for_diagnostic, RunbookExecutionMode, RunbookWorkspaceContext,
    RuntimeContext,
};
use crate::types::{RunbookExecutionContext, RunbookSources};
use kit::constants::{
    SIGNATURE_APPROVED, SIGNATURE_SKIPPABLE, SIGNED_MESSAGE_BYTES, SIGNED_TRANSACTION_BYTES,
};
use kit::indexmap::IndexMap;
use kit::types::commands::CommandExecutionFuture;
use kit::types::frontend::{
    ActionItemRequestUpdate, ActionItemResponse, ActionItemResponseType, Actions, Block,
    BlockEvent, ErrorPanelData, Panel,
};
use kit::types::signers::SignersState;
use kit::types::stores::AddonDefaults;
use kit::types::types::RunbookSupervisionContext;
use kit::types::{ConstructId, PackageId};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Display;
use txtx_addon_kit::{
    hcl::{
        expr::{BinaryOperator, Expression, UnaryOperator},
        template::Element,
    },
    types::{
        commands::{CommandExecutionResult, CommandInputsEvaluationResult, CommandInstance},
        diagnostics::Diagnostic,
        frontend::{ActionItemRequest, ActionItemStatus},
        signers::SignerInstance,
        types::Value,
        ConstructDid,
    },
    uuid::Uuid,
};

// The flow for signer evaluation should be drastically different
// Instead of activating all the signers detected in a graph, we should instead traverse the graph and collecting the signers
// being used.
pub async fn run_signers_evaluation(
    runbook_workspace_context: &RunbookWorkspaceContext,
    runbook_execution_context: &mut RunbookExecutionContext,
    runtime_context: &RuntimeContext,
    supervision_context: &RunbookSupervisionContext,
    action_item_requests: &mut BTreeMap<ConstructDid, Vec<&mut ActionItemRequest>>,
    action_item_responses: &BTreeMap<ConstructDid, Vec<ActionItemResponse>>,
    progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
) -> EvaluationPassResult {
    let mut pass_result = EvaluationPassResult::new(&Uuid::new_v4());

    let signers_instances = &runbook_execution_context.signers_instances;
    let instantiated_signers = runbook_execution_context.order_for_signers_initialization.clone();

    for construct_did in instantiated_signers.into_iter() {
        let package_id = {
            let Some(command) = runbook_execution_context.signers_instances.get(&construct_did)
            else {
                continue;
            };
            command.package_id.clone()
        };

        let construct_id = &runbook_workspace_context.expect_construct_id(&construct_did);

        let instantiated = runbook_execution_context.is_signer_instantiated(&construct_did);

        let (evaluated_inputs_res, _group) =
            match runbook_execution_context.signers_instances.get(&construct_did) {
                None => continue,
                Some(signer_instance) => {
                    let mut cached_dependency_execution_results: HashMap<
                        ConstructDid,
                        Result<&CommandExecutionResult, &Diagnostic>,
                    > = HashMap::new();

                    let references_expressions =
                        signer_instance.get_expressions_referencing_commands_from_inputs().unwrap();

                    for (_input, expr) in references_expressions.into_iter() {
                        let res = runbook_workspace_context
                            .try_resolve_construct_reference_in_expression(&package_id, &expr)
                            .unwrap();

                        if let Some((dependency, _, _)) = res {
                            let evaluation_result_opt = runbook_execution_context
                                .commands_execution_results
                                .get(&dependency);

                            if let Some(evaluation_result) = evaluation_result_opt {
                                match cached_dependency_execution_results.get(&dependency) {
                                    None => {
                                        cached_dependency_execution_results
                                            .insert(dependency, Ok(evaluation_result));
                                    }
                                    Some(Err(diag)) => {
                                        pass_result.push_diagnostic(diag, construct_id);
                                        continue;
                                    }
                                    Some(Ok(_)) => {}
                                }
                            }
                        }
                    }

                    let input_evaluation_results = runbook_execution_context
                        .commands_inputs_evaluation_results
                        .get(&construct_did.clone());

                    let addon_context_key = (package_id.did(), signer_instance.namespace.clone());
                    let addon_defaults =
                        runbook_workspace_context.get_addon_defaults(&addon_context_key);

                    let res = perform_signer_inputs_evaluation(
                        &signer_instance,
                        &cached_dependency_execution_results,
                        &input_evaluation_results,
                        addon_defaults,
                        &package_id,
                        &runbook_workspace_context,
                        &runbook_execution_context,
                        runtime_context,
                    );
                    let group = signer_instance.get_group();
                    (res, group)
                }
            };
        let evaluated_inputs = match evaluated_inputs_res {
            Ok(result) => match result {
                CommandInputEvaluationStatus::Complete(result) => result,
                CommandInputEvaluationStatus::NeedsUserInteraction(_) => {
                    continue;
                }
                CommandInputEvaluationStatus::Aborted(_, diags) => {
                    pass_result.append_diagnostics(diags, construct_id);
                    continue;
                }
            },
            Err(diags) => {
                pass_result.append_diagnostics(diags, construct_id);
                return pass_result;
            }
        };

        let signer = runbook_execution_context.signers_instances.get(&construct_did).unwrap();

        let mut signers_state = runbook_execution_context.signers_state.take().unwrap();
        signers_state.create_new_signer(&construct_did, &signer.name);

        let res = signer
            .check_activability(
                &construct_did,
                &evaluated_inputs,
                signers_state,
                signers_instances,
                &action_item_requests.get(&construct_did),
                &action_item_responses.get(&construct_did),
                supervision_context,
                instantiated,
                instantiated,
            )
            .await;

        let signers_state = match res {
            Ok((signers_state, mut new_actions)) => {
                if new_actions.has_pending_actions() {
                    runbook_execution_context.signers_state = Some(signers_state);
                    pass_result.actions.append(&mut new_actions);
                    continue;
                }
                pass_result.actions.append(&mut new_actions);
                signers_state
            }
            Err((signers_state, diag)) => {
                runbook_execution_context.signers_state = Some(signers_state);
                if let Some(requests) = action_item_requests.get_mut(&construct_did) {
                    for item in requests.iter_mut() {
                        // This should be improved / become more granular
                        let update = ActionItemRequestUpdate::from_id(&item.id)
                            .set_status(ActionItemStatus::Error(diag.clone()));
                        pass_result.actions.push_action_item_update(update);
                    }
                }
                pass_result.push_diagnostic(&diag, construct_id);
                return pass_result;
            }
        };

        runbook_execution_context
            .commands_inputs_evaluation_results
            .insert(construct_did.clone(), evaluated_inputs.clone());

        let res = signer
            .perform_activation(
                &construct_did,
                &evaluated_inputs,
                signers_state,
                signers_instances,
                progress_tx,
            )
            .await;

        let (mut result, signers_state) = match res {
            Ok((signers_state, result)) => (Some(result), Some(signers_state)),
            Err((signers_state, diag)) => {
                runbook_execution_context.signers_state = Some(signers_state);
                pass_result.push_diagnostic(&diag, construct_id);
                return pass_result;
            }
        };
        runbook_execution_context.signers_state = signers_state;
        let Some(result) = result.take() else {
            continue;
        };

        runbook_execution_context.commands_execution_results.insert(construct_did.clone(), result);
    }

    pass_result
}

pub struct EvaluationPassResult {
    pub actions: Actions,
    diagnostics: Vec<Diagnostic>,
    pub pending_background_tasks_futures: Vec<CommandExecutionFuture>,
    pub pending_background_tasks_constructs_uuids: Vec<ConstructDid>,
    pub background_tasks_uuid: Uuid,
}

impl EvaluationPassResult {
    pub fn new(background_tasks_uuid: &Uuid) -> Self {
        Self {
            actions: Actions::none(),
            diagnostics: vec![],
            pending_background_tasks_futures: vec![],
            pending_background_tasks_constructs_uuids: vec![],
            background_tasks_uuid: background_tasks_uuid.clone(),
        }
    }

    pub fn compile_diagnostics_to_block(&self) -> Option<Block> {
        if self.diagnostics.is_empty() {
            return None;
        };
        Some(Block {
            uuid: Uuid::new_v4(),
            visible: true,
            panel: Panel::ErrorPanel(ErrorPanelData::from_diagnostics(&self.diagnostics)),
        })
    }

    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        self.diagnostics.clone()
    }
    pub fn has_diagnostics(&self) -> bool {
        !self.diagnostics.is_empty()
    }

    pub fn push_diagnostic(&mut self, diag: &Diagnostic, construct_id: &ConstructId) {
        self.diagnostics.push(diag.clone().location(&construct_id.construct_location))
    }

    pub fn append_diagnostics(&mut self, diags: Vec<Diagnostic>, construct_id: &ConstructId) {
        diags.iter().for_each(|diag| self.push_diagnostic(diag, construct_id))
    }

    pub fn fill_diagnostic_span(&mut self, runbook_sources: &RunbookSources) {
        for diag in self.diagnostics.iter_mut() {
            diag.span = get_source_context_for_diagnostic(diag, runbook_sources);
        }
    }

    pub fn with_spans_filled(mut self, runbook_sources: &RunbookSources) -> Vec<Diagnostic> {
        self.fill_diagnostic_span(runbook_sources);
        self.diagnostics()
    }
}

impl Display for EvaluationPassResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "EvaluationPassResult {} {{", self.background_tasks_uuid)?;
        writeln!(f, "  actions: {:?}", self.actions)?;
        writeln!(f, "  diagnostics: {:?}", self.diagnostics)?;
        writeln!(
            f,
            "  pending_background_tasks: {:?}",
            self.pending_background_tasks_constructs_uuids
        )?;
        writeln!(f, "}}")
    }
}

// When the graph is being traversed, we are evaluating constructs one after the other.
// After ensuring their executability, we execute them.
// Unexecutable nodes are tainted.
// Before evaluating the executability, we first check if they depend on a tainted node.
pub async fn run_constructs_evaluation(
    background_tasks_uuid: &Uuid,
    runbook_workspace_context: &RunbookWorkspaceContext,
    runbook_execution_context: &mut RunbookExecutionContext,
    runtime_context: &RuntimeContext,
    supervision_context: &RunbookSupervisionContext,
    action_item_requests: &mut BTreeMap<ConstructDid, Vec<&mut ActionItemRequest>>,
    action_item_responses: &BTreeMap<ConstructDid, Vec<ActionItemResponse>>,
    progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
) -> EvaluationPassResult {
    let mut pass_result = EvaluationPassResult::new(background_tasks_uuid);

    let mut unexecutable_nodes: HashSet<ConstructDid> = HashSet::new();

    let top_level_inputs = runbook_workspace_context.top_level_inputs_values.clone();
    for (input_uuid, value) in top_level_inputs.into_iter() {
        let mut res = CommandExecutionResult::new();
        res.outputs.insert("value".into(), value);
        runbook_execution_context.commands_execution_results.insert(input_uuid, res);
    }

    for signer_states in runbook_execution_context.signers_state.iter_mut() {
        for (_, signer) in signer_states.store.iter_mut() {
            signer.clear_autoincrementable_nonce();
        }
    }

    let mut genesis_dependency_execution_results = HashMap::new();

    let mut signers_results = HashMap::new();
    for (signer_construct_did, _) in runbook_execution_context.signers_instances.iter() {
        let mut result = CommandExecutionResult::new();
        result
            .outputs
            .insert("value".into(), Value::string(signer_construct_did.value().to_string()));
        signers_results.insert(signer_construct_did.clone(), result);
    }

    for (signer_construct_did, _) in runbook_execution_context.signers_instances.iter() {
        let results: &CommandExecutionResult = signers_results.get(signer_construct_did).unwrap();
        genesis_dependency_execution_results.insert(signer_construct_did.clone(), Ok(results));
    }

    let ordered_constructs = runbook_execution_context.order_for_commands_execution.clone();

    for construct_did in ordered_constructs.into_iter() {
        let Some(command_instance) =
            runbook_execution_context.commands_instances.get(&construct_did)
        else {
            // runtime_context.addons.index_command_instance(namespace, package_did, block)
            continue;
        };
        if let Some(_) = runbook_execution_context.commands_execution_results.get(&construct_did) {
            continue;
        };

        if let Some(_) = unexecutable_nodes.get(&construct_did) {
            if let Some(deps) = runbook_execution_context.commands_dependencies.get(&construct_did)
            {
                for dep in deps.iter() {
                    unexecutable_nodes.insert(dep.clone());
                }
            }
            continue;
        }

        let package_id = command_instance.package_id.clone();
        let construct_id = &runbook_workspace_context.expect_construct_id(&construct_did);

        let addon_context_key = (package_id.did(), command_instance.namespace.clone());
        let addon_defaults = runbook_workspace_context.get_addon_defaults(&addon_context_key);

        let input_evaluation_results = runbook_execution_context
            .commands_inputs_evaluation_results
            .get(&construct_did.clone());

        let mut cached_dependency_execution_results: HashMap<
            ConstructDid,
            Result<&CommandExecutionResult, &Diagnostic>,
        > = genesis_dependency_execution_results.clone();

        // Retrieve the construct_did of the inputs
        // Collect the outputs
        let references_expressions =
            command_instance.get_expressions_referencing_commands_from_inputs().unwrap();

        for (_input, expr) in references_expressions.into_iter() {
            let res = runbook_workspace_context
                .try_resolve_construct_reference_in_expression(&package_id, &expr)
                .unwrap();

            if let Some((dependency, _, _)) = res {
                let evaluation_result_opt =
                    runbook_execution_context.commands_execution_results.get(&dependency);

                if let Some(evaluation_result) = evaluation_result_opt {
                    match cached_dependency_execution_results.get(&dependency) {
                        None => {
                            cached_dependency_execution_results
                                .insert(dependency, Ok(evaluation_result));
                        }
                        Some(Err(_)) => continue,
                        Some(Ok(_)) => {
                            cached_dependency_execution_results
                                .insert(dependency, Ok(evaluation_result));
                        }
                    }
                }
            }
        }

        let evaluated_inputs_res = perform_inputs_evaluation(
            command_instance,
            &cached_dependency_execution_results,
            &input_evaluation_results,
            addon_defaults,
            &action_item_responses.get(&construct_did),
            &package_id,
            runbook_workspace_context,
            runbook_execution_context,
            runtime_context,
            false,
        );
        let Some(command_instance) =
            runbook_execution_context.commands_instances.get_mut(&construct_did)
        else {
            // runtime_context.addons.index_command_instance(namespace, package_did, block)
            continue;
        };

        let mut evaluated_inputs = match evaluated_inputs_res {
            Ok(result) => match result {
                CommandInputEvaluationStatus::Complete(result) => result,
                CommandInputEvaluationStatus::NeedsUserInteraction(_) => continue,
                CommandInputEvaluationStatus::Aborted(_, diags) => {
                    pass_result.append_diagnostics(diags, construct_id);
                    return pass_result;
                }
            },
            Err(diags) => {
                pass_result.append_diagnostics(diags, construct_id);
                return pass_result;
            }
        };

        let execution_result = if command_instance.specification.implements_signing_capability {
            let signers = runbook_execution_context.signers_state.take().unwrap();
            let signers = update_signer_instances_from_action_response(
                signers,
                &construct_did,
                &action_item_responses.get(&construct_did),
            );
            let res = command_instance
                .check_signed_executability(
                    &construct_did,
                    &mut evaluated_inputs,
                    signers,
                    &mut runbook_execution_context.signers_instances,
                    &action_item_responses.get(&construct_did),
                    &action_item_requests.get(&construct_did),
                    supervision_context,
                )
                .await;

            let signers = match res {
                Ok((updated_signers, mut new_actions)) => {
                    if new_actions.has_pending_actions() {
                        pass_result.actions.append(&mut new_actions);
                        runbook_execution_context.signers_state = Some(updated_signers);
                        if let Some(deps) =
                            runbook_execution_context.commands_dependencies.get(&construct_did)
                        {
                            for dep in deps.iter() {
                                unexecutable_nodes.insert(dep.clone());
                            }
                        }
                        continue;
                    }
                    pass_result.actions.append(&mut new_actions);
                    updated_signers
                }
                Err((updated_signers, diag)) => {
                    pass_result.push_diagnostic(&diag, construct_id);
                    runbook_execution_context.signers_state = Some(updated_signers);
                    return pass_result;
                }
            };

            runbook_execution_context
                .commands_inputs_evaluation_results
                .insert(construct_did.clone(), evaluated_inputs.clone());

            let mut empty_vec = vec![];
            let action_items_requests =
                action_item_requests.get_mut(&construct_did).unwrap_or(&mut empty_vec);
            let action_items_response = action_item_responses.get(&construct_did);

            let execution_result = command_instance
                .perform_signed_execution(
                    &construct_did,
                    &evaluated_inputs,
                    signers,
                    &runbook_execution_context.signers_instances,
                    action_items_requests,
                    &action_items_response,
                    progress_tx,
                )
                .await;

            let execution_result = match execution_result {
                // todo(lgalabru): return Diagnostic instead
                Ok((updated_signers, result)) => {
                    runbook_execution_context.signers_state = Some(updated_signers);
                    Ok(result)
                }
                Err((updated_signers, diag)) => {
                    runbook_execution_context.signers_state = Some(updated_signers);
                    if let Some(deps) =
                        runbook_execution_context.commands_dependencies.get(&construct_did)
                    {
                        for dep in deps.iter() {
                            unexecutable_nodes.insert(dep.clone());
                        }
                    }
                    Err(diag)
                }
            };
            execution_result
        } else {
            match command_instance.check_executability(
                &construct_did,
                &mut evaluated_inputs,
                &mut runbook_execution_context.signers_instances,
                &action_item_responses.get(&construct_did),
                supervision_context,
            ) {
                Ok(mut new_actions) => {
                    if new_actions.has_pending_actions() {
                        pass_result.actions.append(&mut new_actions);
                        if let Some(deps) =
                            runbook_execution_context.commands_dependencies.get(&construct_did)
                        {
                            for dep in deps.iter() {
                                unexecutable_nodes.insert(dep.clone());
                            }
                        }
                        continue;
                    }
                    pass_result.actions.append(&mut new_actions);
                }
                Err(diag) => {
                    pass_result.push_diagnostic(&diag, construct_id);
                    return pass_result;
                }
            }

            runbook_execution_context
                .commands_inputs_evaluation_results
                .insert(construct_did.clone(), evaluated_inputs.clone());

            let mut empty_vec = vec![];
            let action_items_requests =
                action_item_requests.get_mut(&construct_did).unwrap_or(&mut empty_vec);
            let action_items_response = action_item_responses.get(&construct_did);

            let execution_result = {
                command_instance
                    .perform_execution(
                        &construct_did,
                        &evaluated_inputs,
                        action_items_requests,
                        &action_items_response,
                        progress_tx,
                    )
                    .await
            };

            let execution_result = match execution_result {
                // todo(lgalabru): return Diagnostic instead
                Ok(result) => Ok(result),
                Err(e) => {
                    if let Some(deps) =
                        runbook_execution_context.commands_dependencies.get(&construct_did)
                    {
                        for dep in deps.iter() {
                            unexecutable_nodes.insert(dep.clone());
                        }
                    }
                    Err(e)
                }
            };
            execution_result
        };

        let mut execution_result = match execution_result {
            Ok(res) => res,
            Err(diag) => {
                pass_result.push_diagnostic(&diag, construct_id);
                continue;
            }
        };

        if let RunbookExecutionMode::Partial(ref mut executed_constructs) =
            runbook_execution_context.execution_mode
        {
            executed_constructs.push(construct_did.clone());
        }

        if command_instance.specification.implements_background_task_capability {
            let future_res = command_instance.build_background_task(
                &construct_did,
                &evaluated_inputs,
                &execution_result,
                progress_tx,
                &pass_result.background_tasks_uuid,
                supervision_context,
            );
            let future = match future_res {
                Ok(future) => future,
                Err(diag) => {
                    pass_result.push_diagnostic(&diag, construct_id);
                    return pass_result;
                }
            };
            if let Some(deps) = runbook_execution_context.commands_dependencies.get(&construct_did)
            {
                for dep in deps.iter() {
                    unexecutable_nodes.insert(dep.clone());
                }
            }
            pass_result.pending_background_tasks_futures.push(future);
            pass_result.pending_background_tasks_constructs_uuids.push(construct_did.clone());
        } else {
            runbook_execution_context
                .commands_execution_results
                .entry(construct_did)
                .or_insert_with(CommandExecutionResult::new)
                .append(&mut execution_result);
        }
    }
    pass_result
}

// When the graph is being traversed, we are evaluating constructs one after the other.
// After ensuring their executability, we execute them.
// Unexecutable nodes are tainted.
// Before evaluating the executability, we first check if they depend on a tainted node.

#[derive(Debug)]
pub enum ExpressionEvaluationStatus {
    CompleteOk(Value),
    CompleteErr(Diagnostic),
    DependencyNotComputed,
}

pub fn eval_expression(
    expr: &Expression,
    dependencies_execution_results: &HashMap<
        ConstructDid,
        Result<&CommandExecutionResult, &Diagnostic>,
    >,
    package_id: &PackageId,
    runbook_workspace_context: &RunbookWorkspaceContext,
    runbook_execution_context: &RunbookExecutionContext,
    runtime_context: &RuntimeContext,
) -> Result<ExpressionEvaluationStatus, Diagnostic> {
    let value = match expr {
        // Represents a null value.
        Expression::Null(_decorated_null) => Value::null(),
        // Represents a boolean.
        Expression::Bool(decorated_bool) => Value::bool(*decorated_bool.value()),
        // Represents a number, either integer or float.
        Expression::Number(formatted_number) => {
            match (
                formatted_number.value().as_u64(),
                formatted_number.value().as_i64(),
                formatted_number.value().as_f64(),
            ) {
                (Some(value), _, _) => Value::integer(value.into()),
                (_, Some(value), _) => Value::integer(value.into()),
                (_, _, Some(value)) => Value::float(value),
                (None, None, None) => unreachable!(), // todo(lgalabru): return Diagnostic
            }
        }
        // Represents a string that does not contain any template interpolations or template directives.
        Expression::String(decorated_string) => Value::string(decorated_string.to_string()),
        // Represents an HCL array.
        Expression::Array(entries) => {
            let mut res = vec![];
            for entry_expr in entries {
                let value = match eval_expression(
                    entry_expr,
                    dependencies_execution_results,
                    package_id,
                    runbook_workspace_context,
                    runbook_execution_context,
                    runtime_context,
                )? {
                    ExpressionEvaluationStatus::CompleteOk(result) => result,
                    ExpressionEvaluationStatus::CompleteErr(e) => {
                        return Ok(ExpressionEvaluationStatus::CompleteErr(e))
                    }
                    ExpressionEvaluationStatus::DependencyNotComputed => {
                        return Ok(ExpressionEvaluationStatus::DependencyNotComputed)
                    }
                };
                res.push(value);
            }
            Value::array(res)
        }
        // Represents an HCL object.
        Expression::Object(object) => {
            let mut map = IndexMap::new();
            for (k, v) in object.into_iter() {
                let key = match k {
                    txtx_addon_kit::hcl::expr::ObjectKey::Expression(k_expr) => {
                        match eval_expression(
                            k_expr,
                            dependencies_execution_results,
                            package_id,
                            runbook_workspace_context,
                            runbook_execution_context,
                            runtime_context,
                        )? {
                            ExpressionEvaluationStatus::CompleteOk(result) => match result {
                                Value::String(result) => result,
                                _ => {
                                    return Ok(ExpressionEvaluationStatus::CompleteErr(
                                        Diagnostic::error_from_string(
                                            "object key must evaluate to a string".to_string(),
                                        ),
                                    ))
                                }
                            },
                            ExpressionEvaluationStatus::CompleteErr(e) => {
                                return Ok(ExpressionEvaluationStatus::CompleteErr(e))
                            }
                            ExpressionEvaluationStatus::DependencyNotComputed => {
                                return Ok(ExpressionEvaluationStatus::DependencyNotComputed)
                            }
                        }
                    }
                    txtx_addon_kit::hcl::expr::ObjectKey::Ident(k_ident) => k_ident.to_string(),
                };
                let value = match eval_expression(
                    v.expr(),
                    dependencies_execution_results,
                    package_id,
                    runbook_workspace_context,
                    runbook_execution_context,
                    runtime_context,
                )? {
                    ExpressionEvaluationStatus::CompleteOk(result) => result,
                    ExpressionEvaluationStatus::CompleteErr(e) => {
                        return Ok(ExpressionEvaluationStatus::CompleteErr(e))
                    }
                    ExpressionEvaluationStatus::DependencyNotComputed => {
                        return Ok(ExpressionEvaluationStatus::DependencyNotComputed)
                    }
                };
                map.insert(key, value);
            }
            Value::Object(map)
        }
        // Represents a string containing template interpolations and template directives.
        Expression::StringTemplate(string_template) => {
            let mut res = String::new();
            for element in string_template.into_iter() {
                match element {
                    Element::Literal(literal) => {
                        res.push_str(literal.value());
                    }
                    Element::Interpolation(interpolation) => {
                        let value = match eval_expression(
                            &interpolation.expr,
                            dependencies_execution_results,
                            package_id,
                            runbook_workspace_context,
                            runbook_execution_context,
                            runtime_context,
                        )? {
                            ExpressionEvaluationStatus::CompleteOk(result) => result.to_string(),
                            ExpressionEvaluationStatus::CompleteErr(e) => {
                                return Ok(ExpressionEvaluationStatus::CompleteErr(e))
                            }
                            ExpressionEvaluationStatus::DependencyNotComputed => {
                                return Ok(ExpressionEvaluationStatus::DependencyNotComputed)
                            }
                        };
                        res.push_str(&value);
                    }
                    Element::Directive(_) => {
                        unimplemented!("string templates with directives not yet supported")
                    }
                };
            }
            Value::string(res)
        }
        // Represents an HCL heredoc template.
        Expression::HeredocTemplate(_heredoc_template) => {
            unimplemented!()
        }
        // Represents a sub-expression wrapped in parenthesis.
        Expression::Parenthesis(_sub_expr) => {
            unimplemented!()
        }
        // Represents a variable identifier.
        Expression::Variable(_decorated_var) => {
            unimplemented!()
        }
        // Represents conditional operator which selects one of two expressions based on the outcome of a boolean expression.
        Expression::Conditional(_conditional) => {
            unimplemented!()
        }
        // Represents a function call.
        Expression::FuncCall(function_call) => {
            let func_namespace = function_call.name.namespace.first().map(|n| n.to_string());
            let func_name = function_call.name.name.to_string();
            let mut args = vec![];
            for expr in function_call.args.iter() {
                let value = match eval_expression(
                    expr,
                    dependencies_execution_results,
                    package_id,
                    runbook_workspace_context,
                    runbook_execution_context,
                    runtime_context,
                )? {
                    ExpressionEvaluationStatus::CompleteOk(result) => result,
                    ExpressionEvaluationStatus::CompleteErr(e) => {
                        return Ok(ExpressionEvaluationStatus::CompleteErr(e))
                    }
                    ExpressionEvaluationStatus::DependencyNotComputed => {
                        return Ok(ExpressionEvaluationStatus::DependencyNotComputed)
                    }
                };
                args.push(value);
            }
            runtime_context
                .execute_function(
                    package_id.did(),
                    func_namespace,
                    &func_name,
                    &args,
                    &runtime_context.authorization_context,
                )
                .map_err(|e| e)?
        }
        // Represents an attribute or element traversal.
        Expression::Traversal(_) => {
            let (dependency, mut components, _subpath) = match runbook_workspace_context
                .try_resolve_construct_reference_in_expression(package_id, expr)
            {
                Ok(Some(res)) => res,
                Ok(None) => {
                    return Err(diagnosed_error!(
                        "unable to resolve expression '{}'",
                        expr.to_string().trim()
                    ));
                }
                Err(e) => {
                    return Err(diagnosed_error!(
                        "unable to resolve expression '{}': {}",
                        expr.to_string().trim(),
                        e
                    ))
                }
            };

            let res: &CommandExecutionResult = match dependencies_execution_results.get(&dependency)
            {
                Some(res) => match res.clone() {
                    Ok(res) => res,
                    Err(e) => return Ok(ExpressionEvaluationStatus::CompleteErr(e.clone())),
                },
                None => match runbook_execution_context.commands_execution_results.get(&dependency)
                {
                    Some(res) => res,
                    None => return Ok(ExpressionEvaluationStatus::DependencyNotComputed),
                },
            };

            let attribute = components.pop_front().unwrap_or("value".into());
            // this is a bit hacky. in some cases, our outputs are nested in a "value", but we don't want the user
            // to have to provide it. if that's the case, the above line consumed an attribute we want to use and
            // didn't actually use the default "value" key. so if fetching the provided attribute key yields no
            // results, fetch "value", and add our attribute back to the list of components
            match res.outputs.get(&attribute).or(res.outputs.get("value")) {
                Some(output) => {
                    if let Some(_) = output.as_object() {
                        components.push_front(attribute);
                        output.get_keys_from_object(components)?
                    } else {
                        output.clone()
                    }
                }
                None => return Ok(ExpressionEvaluationStatus::DependencyNotComputed),
            }
        }
        // Represents an operation which applies a unary operator to an expression.
        Expression::UnaryOp(unary_op) => {
            let _expr = eval_expression(
                &unary_op.expr,
                dependencies_execution_results,
                package_id,
                runbook_workspace_context,
                runbook_execution_context,
                runtime_context,
            )?;
            match &unary_op.operator.value() {
                UnaryOperator::Neg => {}
                UnaryOperator::Not => {}
            }
            unimplemented!()
        }
        // Represents an operation which applies a binary operator to two expressions.
        Expression::BinaryOp(binary_op) => {
            let lhs = match eval_expression(
                &binary_op.lhs_expr,
                dependencies_execution_results,
                package_id,
                runbook_workspace_context,
                runbook_execution_context,
                runtime_context,
            )? {
                ExpressionEvaluationStatus::CompleteOk(result) => result,
                ExpressionEvaluationStatus::CompleteErr(e) => {
                    return Ok(ExpressionEvaluationStatus::CompleteErr(e))
                }
                ExpressionEvaluationStatus::DependencyNotComputed => {
                    return Ok(ExpressionEvaluationStatus::DependencyNotComputed)
                }
            };
            let rhs = match eval_expression(
                &binary_op.rhs_expr,
                dependencies_execution_results,
                package_id,
                runbook_workspace_context,
                runbook_execution_context,
                runtime_context,
            )? {
                ExpressionEvaluationStatus::CompleteOk(result) => result,
                ExpressionEvaluationStatus::CompleteErr(e) => {
                    return Ok(ExpressionEvaluationStatus::CompleteErr(e))
                }
                ExpressionEvaluationStatus::DependencyNotComputed => {
                    return Ok(ExpressionEvaluationStatus::DependencyNotComputed)
                }
            };
            if !lhs.is_type_eq(&rhs) {
                unimplemented!() // todo(lgalabru): return diagnostic
            }

            let func = match &binary_op.operator.value() {
                BinaryOperator::And => "and_bool",
                BinaryOperator::Div => "div",
                BinaryOperator::Eq => "eq",
                BinaryOperator::Greater => "gt",
                BinaryOperator::GreaterEq => "gte",
                BinaryOperator::Less => "lt",
                BinaryOperator::LessEq => "lte",
                BinaryOperator::Minus => "minus",
                BinaryOperator::Mod => "modulo",
                BinaryOperator::Mul => "multiply",
                BinaryOperator::Plus => "add",
                BinaryOperator::NotEq => "neq",
                BinaryOperator::Or => "or_bool",
            };
            runtime_context.execute_function(
                package_id.did(),
                None,
                func,
                &vec![lhs, rhs],
                &runtime_context.authorization_context,
            )?
        }
        // Represents a construct for constructing a collection by projecting the items from another collection.
        Expression::ForExpr(_for_expr) => {
            unimplemented!()
        }
    };

    Ok(ExpressionEvaluationStatus::CompleteOk(value))
}

// pub struct EvaluatedExpression {
//     value: Value,
// }

pub fn update_signer_instances_from_action_response(
    mut signers: SignersState,
    construct_did: &ConstructDid,
    action_item_response: &Option<&Vec<ActionItemResponse>>,
) -> SignersState {
    match action_item_response {
        Some(responses) => {
            responses.into_iter().for_each(|ActionItemResponse { action_item_id: _, payload }| {
                match payload {
                    ActionItemResponseType::ProvideSignedTransaction(response) => {
                        if let Some(mut signer_state) =
                            signers.pop_signer_state(&response.signer_uuid)
                        {
                            let did = &construct_did.to_string();
                            match &response.signed_transaction_bytes {
                                Some(bytes) => {
                                    signer_state.insert_scoped_value(
                                        &did,
                                        SIGNED_TRANSACTION_BYTES,
                                        Value::string(bytes.clone()),
                                    );
                                }
                                None => match response.signature_approved {
                                    Some(true) => {
                                        signer_state.insert_scoped_value(
                                            &did,
                                            SIGNATURE_APPROVED,
                                            Value::bool(true),
                                        );
                                    }
                                    Some(false) => {}
                                    None => {
                                        let skippable = signer_state
                                            .get_scoped_value(&did, SIGNATURE_SKIPPABLE)
                                            .and_then(|v| v.as_bool())
                                            .unwrap_or(false);
                                        if skippable {
                                            signer_state.insert_scoped_value(
                                                &did,
                                                SIGNED_TRANSACTION_BYTES,
                                                Value::null(),
                                            );
                                        }
                                    }
                                },
                            }
                            signers.push_signer_state(signer_state.clone());
                        }
                    }
                    ActionItemResponseType::ProvideSignedMessage(response) => {
                        if let Some(mut signer_state) =
                            signers.pop_signer_state(&response.signer_uuid)
                        {
                            signer_state.insert_scoped_value(
                                &construct_did.value().to_string(),
                                SIGNED_MESSAGE_BYTES,
                                Value::string(response.signed_message_bytes.clone()),
                            );
                            signers.push_signer_state(signer_state.clone());
                        }
                    }
                    _ => {}
                }
            });
        }
        None => {}
    }

    signers
}

#[derive(Debug)]
pub enum CommandInputEvaluationStatus {
    Complete(CommandInputsEvaluationResult),
    NeedsUserInteraction(CommandInputsEvaluationResult),
    Aborted(CommandInputsEvaluationResult, Vec<Diagnostic>),
}

pub fn perform_inputs_evaluation(
    command_instance: &CommandInstance,
    dependencies_execution_results: &HashMap<
        ConstructDid,
        Result<&CommandExecutionResult, &Diagnostic>,
    >,
    input_evaluation_results: &Option<&CommandInputsEvaluationResult>,
    addon_defaults: &AddonDefaults,
    action_item_response: &Option<&Vec<ActionItemResponse>>,
    package_id: &PackageId,
    runbook_workspace_context: &RunbookWorkspaceContext,
    runbook_execution_context: &RunbookExecutionContext,
    runtime_context: &RuntimeContext,
    simulation: bool,
) -> Result<CommandInputEvaluationStatus, Vec<Diagnostic>> {
    let mut results = match *input_evaluation_results {
        Some(evaluated_inputs) => evaluated_inputs.clone(),
        None => CommandInputsEvaluationResult::new(&command_instance.name, &addon_defaults.store),
    };
    let mut require_user_interaction = false;
    let mut diags = vec![];
    let inputs = command_instance.specification.inputs.clone();
    let mut fatal_error = false;

    match action_item_response {
        Some(responses) => {
            responses.into_iter().for_each(|ActionItemResponse { action_item_id: _, payload }| {
                match payload {
                    ActionItemResponseType::ReviewInput(_update) => {}
                    ActionItemResponseType::ProvideInput(update) => {
                        results.inputs.insert(&update.input_name, update.updated_value.clone());
                    }
                    _ => {}
                }
            })
        }
        None => {}
    }

    for input in inputs.into_iter() {
        if simulation {
            // Hard coding "signer" here is a shortcut - to be improved, we should retrieve a pointer instead that is defined on the spec
            if input.name.eq("signer") {
                results.unevaluated_inputs.insert("signer".into(), None);
                continue;
            }
            if input.name.eq("signers") {
                results.unevaluated_inputs.insert("signers".into(), None);
                continue;
            }
        } else {
            if !results.unevaluated_inputs.contains_key(&input.name) {
                continue;
            }
        }
        if let Some(object_props) = input.as_object() {
            // get this object expression to check if it's a traversal. if the expected
            // object type is a traversal, we should parse it as a regular field rather than
            // looking at each property of the object
            let Some(expr) = command_instance.get_expression_from_object(&input)? else {
                continue;
            };
            if let Expression::Traversal(traversal) = &expr {
                let value = match eval_expression(
                    &Expression::Traversal(traversal.clone()),
                    dependencies_execution_results,
                    package_id,
                    runbook_workspace_context,
                    runbook_execution_context,
                    runtime_context,
                ) {
                    Ok(ExpressionEvaluationStatus::CompleteOk(result)) => result,
                    Ok(ExpressionEvaluationStatus::CompleteErr(e)) => {
                        if e.is_error() {
                            fatal_error = true;
                        }
                        results.unevaluated_inputs.insert(input.name.clone(), Some(e.clone()));
                        diags.push(e);
                        continue;
                    }
                    Err(e) => {
                        if e.is_error() {
                            fatal_error = true;
                        }
                        results.unevaluated_inputs.insert(input.name.clone(), Some(e.clone()));
                        diags.push(e);
                        continue;
                    }
                    Ok(ExpressionEvaluationStatus::DependencyNotComputed) => {
                        require_user_interaction = true;
                        results.unevaluated_inputs.insert(input.name.clone(), None);
                        continue;
                    }
                };
                results.insert(&input.name, value);
                continue;
            }

            if let Expression::FuncCall(ref function_call) = expr {
                let value = match eval_expression(
                    &Expression::FuncCall(function_call.clone()),
                    dependencies_execution_results,
                    package_id,
                    runbook_workspace_context,
                    runbook_execution_context,
                    runtime_context,
                ) {
                    Ok(ExpressionEvaluationStatus::CompleteOk(result)) => result,
                    Ok(ExpressionEvaluationStatus::CompleteErr(e)) => {
                        if e.is_error() {
                            fatal_error = true;
                        }
                        results.unevaluated_inputs.insert(input.name.clone(), Some(e.clone()));
                        diags.push(e);
                        continue;
                    }
                    Err(e) => {
                        if e.is_error() {
                            fatal_error = true;
                        }
                        results.unevaluated_inputs.insert(input.name.clone(), Some(e.clone()));
                        diags.push(e);
                        continue;
                    }
                    Ok(ExpressionEvaluationStatus::DependencyNotComputed) => {
                        require_user_interaction = true;
                        results.unevaluated_inputs.insert(input.name.clone(), None);
                        continue;
                    }
                };
                results.insert(&input.name, value);
                continue;
            }
            let mut object_values = IndexMap::new();
            for prop in object_props.iter() {
                let Some(expr) =
                    command_instance.get_expression_from_object_property(&input, &prop)?
                else {
                    continue;
                };

                let value = match eval_expression(
                    &expr,
                    dependencies_execution_results,
                    package_id,
                    runbook_workspace_context,
                    runbook_execution_context,
                    runtime_context,
                ) {
                    Ok(ExpressionEvaluationStatus::CompleteOk(result)) => result,
                    Ok(ExpressionEvaluationStatus::CompleteErr(e)) => {
                        if e.is_error() {
                            fatal_error = true;
                        }
                        results.unevaluated_inputs.insert(input.name.clone(), Some(e.clone()));
                        diags.push(e);
                        continue;
                    }
                    Err(e) => {
                        if e.is_error() {
                            fatal_error = true;
                        }
                        results.unevaluated_inputs.insert(input.name.clone(), Some(e.clone()));
                        diags.push(e);
                        continue;
                    }
                    Ok(ExpressionEvaluationStatus::DependencyNotComputed) => {
                        require_user_interaction = true;
                        results.unevaluated_inputs.insert(input.name.clone(), None);
                        continue;
                    }
                };

                match value.clone() {
                    Value::Object(obj) => {
                        for (k, v) in obj.into_iter() {
                            object_values.insert(k, v);
                        }
                    }
                    v => {
                        object_values.insert(prop.name.to_string(), v);
                    }
                };
            }

            if !object_values.is_empty() {
                results.insert(&input.name, Value::object(object_values));
            }
        } else if let Some(_) = input.as_array() {
            let mut array_values = vec![];
            let Some(expr) = command_instance.get_expression_from_input(&input)? else {
                continue;
            };
            let value = match eval_expression(
                &expr,
                dependencies_execution_results,
                package_id,
                runbook_workspace_context,
                runbook_execution_context,
                runtime_context,
            ) {
                Ok(ExpressionEvaluationStatus::CompleteOk(result)) => match result {
                    Value::Addon(_) => unreachable!(),
                    Value::Object(_) => unreachable!(),
                    Value::Array(entries) => {
                        for (i, entry) in entries.into_iter().enumerate() {
                            array_values.insert(i, entry); // todo: is it okay that we possibly overwrite array values from previous input evals?
                        }
                        Value::array(array_values)
                    }
                    _ => result,
                },
                Ok(ExpressionEvaluationStatus::CompleteErr(e)) => {
                    if e.is_error() {
                        fatal_error = true;
                    }
                    results.unevaluated_inputs.insert(input.name.clone(), Some(e.clone()));
                    diags.push(e);
                    continue;
                }
                Err(e) => {
                    if e.is_error() {
                        fatal_error = true;
                    }
                    results.unevaluated_inputs.insert(input.name.clone(), Some(e.clone()));
                    diags.push(e);
                    continue;
                }
                Ok(ExpressionEvaluationStatus::DependencyNotComputed) => {
                    require_user_interaction = true;
                    results.unevaluated_inputs.insert(input.name.clone(), None);
                    continue;
                }
            };

            results.insert(&input.name, value);
        } else if let Some(object_props) = input.as_map() {
            let mut entries = vec![];
            let blocks = command_instance.get_blocks_for_map(&input)?;
            for block in blocks.iter() {
                let mut object_values = IndexMap::new();
                for prop in object_props.iter() {
                    let Some(expr) = command_instance.get_expression_from_block(&block, &prop)?
                    else {
                        continue;
                    };
                    let value = match eval_expression(
                        &expr,
                        dependencies_execution_results,
                        package_id,
                        runbook_workspace_context,
                        runbook_execution_context,
                        runtime_context,
                    ) {
                        Ok(ExpressionEvaluationStatus::CompleteOk(result)) => result,
                        Ok(ExpressionEvaluationStatus::CompleteErr(e)) => {
                            if e.is_error() {
                                fatal_error = true;
                            }
                            results.unevaluated_inputs.insert(input.name.clone(), Some(e.clone()));
                            diags.push(e);
                            continue;
                        }
                        Err(e) => {
                            if e.is_error() {
                                fatal_error = true;
                            }
                            results.unevaluated_inputs.insert(input.name.clone(), Some(e.clone()));
                            diags.push(e);
                            continue;
                        }
                        Ok(ExpressionEvaluationStatus::DependencyNotComputed) => {
                            require_user_interaction = true;
                            results.unevaluated_inputs.insert(input.name.clone(), None);
                            continue;
                        }
                    };

                    match value.clone() {
                        Value::Object(obj) => {
                            for (k, v) in obj.into_iter() {
                                object_values.insert(k, v);
                            }
                        }
                        v => {
                            object_values.insert(prop.name.to_string(), v);
                        }
                    };
                }
                entries.push(Value::object(object_values));
            }
            results.insert(&input.name, Value::array(entries));
        } else {
            let Some(expr) = command_instance.get_expression_from_input(&input)? else {
                continue;
            };
            let value = match eval_expression(
                &expr,
                dependencies_execution_results,
                package_id,
                runbook_workspace_context,
                runbook_execution_context,
                runtime_context,
            ) {
                Ok(ExpressionEvaluationStatus::CompleteOk(result)) => result,
                Ok(ExpressionEvaluationStatus::CompleteErr(e)) => {
                    if e.is_error() {
                        fatal_error = true;
                    }
                    results.unevaluated_inputs.insert(input.name.clone(), Some(e.clone()));
                    diags.push(e);
                    continue;
                }
                Err(e) => {
                    if e.is_error() {
                        fatal_error = true;
                    }
                    results.unevaluated_inputs.insert(input.name.clone(), Some(e.clone()));
                    diags.push(e);
                    continue;
                }
                Ok(ExpressionEvaluationStatus::DependencyNotComputed) => {
                    require_user_interaction = true;
                    results.unevaluated_inputs.insert(input.name.clone(), None);
                    continue;
                }
            };
            results.insert(&input.name, value);
        }
    }

    if fatal_error {
        return Ok(CommandInputEvaluationStatus::Aborted(results, diags));
    }

    let status = match (fatal_error, require_user_interaction) {
        (false, false) => CommandInputEvaluationStatus::Complete(results),
        (_, _) => CommandInputEvaluationStatus::NeedsUserInteraction(results),
    };
    Ok(status)
}

pub fn perform_signer_inputs_evaluation(
    signer_instance: &SignerInstance,
    dependencies_execution_results: &HashMap<
        ConstructDid,
        Result<&CommandExecutionResult, &Diagnostic>,
    >,
    input_evaluation_results: &Option<&CommandInputsEvaluationResult>,
    addon_defaults: &AddonDefaults,
    package_id: &PackageId,
    runbook_workspace_context: &RunbookWorkspaceContext,
    runbook_execution_context: &RunbookExecutionContext,
    runtime_context: &RuntimeContext,
) -> Result<CommandInputEvaluationStatus, Vec<Diagnostic>> {
    let mut results =
        CommandInputsEvaluationResult::new(&signer_instance.name, &addon_defaults.store);
    let mut require_user_interaction = false;
    let mut diags = vec![];
    let inputs = signer_instance.specification.inputs.clone();
    let mut fatal_error = false;

    for input in inputs.into_iter() {
        // todo(micaiah): this value still needs to be for inputs that are objects
        let previously_evaluated_input = match input_evaluation_results {
            Some(input_evaluation_results) => {
                input_evaluation_results.inputs.get_value(&input.name)
            }
            None => None,
        };
        if let Some(object_props) = input.as_object() {
            // todo(micaiah) - figure out how user-input values work for this branch
            let mut object_values = IndexMap::new();
            for prop in object_props.iter() {
                if let Some(value) = previously_evaluated_input {
                    match value.clone() {
                        Value::Object(obj) => {
                            for (k, v) in obj.into_iter() {
                                object_values.insert(k, v);
                            }
                        }
                        v => {
                            object_values.insert(prop.name.to_string(), v);
                        }
                    };
                }

                let Some(expr) =
                    signer_instance.get_expression_from_object_property(&input, &prop)?
                else {
                    continue;
                };
                let value = match eval_expression(
                    &expr,
                    dependencies_execution_results,
                    package_id,
                    runbook_workspace_context,
                    runbook_execution_context,
                    runtime_context,
                ) {
                    Ok(ExpressionEvaluationStatus::CompleteOk(result)) => result,
                    Ok(ExpressionEvaluationStatus::CompleteErr(e)) => {
                        if e.is_error() {
                            fatal_error = true;
                        }
                        diags.push(e);
                        continue;
                    }
                    Err(e) => {
                        if e.is_error() {
                            fatal_error = true;
                        }
                        diags.push(e);
                        continue;
                    }
                    Ok(ExpressionEvaluationStatus::DependencyNotComputed) => {
                        require_user_interaction = true;
                        continue;
                    }
                };

                match value.clone() {
                    Value::Object(obj) => {
                        for (k, v) in obj.into_iter() {
                            object_values.insert(k, v);
                        }
                    }
                    v => {
                        object_values.insert(prop.name.to_string(), v);
                    }
                };
            }
            if !object_values.is_empty() {
                results.insert(&input.name, Value::Object(object_values));
            }
        } else if let Some(_) = input.as_array() {
            let mut array_values = vec![];
            if let Some(value) = previously_evaluated_input {
                match value.clone() {
                    Value::Array(entries) => {
                        array_values.extend::<Vec<Value>>(entries.into_iter().collect());
                    }
                    _ => {
                        unreachable!()
                    }
                }
            }

            let Some(expr) = signer_instance.get_expression_from_input(&input)? else {
                continue;
            };
            let value = match eval_expression(
                &expr,
                dependencies_execution_results,
                package_id,
                runbook_workspace_context,
                runbook_execution_context,
                runtime_context,
            ) {
                Ok(ExpressionEvaluationStatus::CompleteOk(result)) => match result {
                    Value::Array(entries) => {
                        for (i, entry) in entries.into_iter().enumerate() {
                            array_values.insert(i, entry); // todo: is it okay that we possibly overwrite array values from previous input evals?
                        }
                        Value::array(array_values)
                    }
                    _ => unreachable!(),
                },
                Ok(ExpressionEvaluationStatus::CompleteErr(e)) => {
                    if e.is_error() {
                        fatal_error = true;
                    }
                    diags.push(e);
                    continue;
                }
                Err(e) => {
                    if e.is_error() {
                        fatal_error = true;
                    }
                    diags.push(e);
                    continue;
                }
                Ok(ExpressionEvaluationStatus::DependencyNotComputed) => {
                    // todo
                    let Expression::Array(exprs) = expr else { panic!() };
                    let mut references = vec![];
                    for expr in exprs.iter() {
                        let result = runbook_workspace_context
                            .try_resolve_construct_reference_in_expression(package_id, &expr);
                        if let Ok(Some((construct_did, _, _))) = result {
                            references.push(Value::string(construct_did.value().to_string()));
                        }
                    }
                    results.inputs.insert(&input.name, Value::array(references));
                    continue;
                }
            };
            results.insert(&input.name, value);
        } else {
            let value = if let Some(value) = previously_evaluated_input {
                value.clone()
            } else {
                let Some(expr) = signer_instance.get_expression_from_input(&input)? else {
                    continue;
                };
                match eval_expression(
                    &expr,
                    dependencies_execution_results,
                    package_id,
                    runbook_workspace_context,
                    runbook_execution_context,
                    runtime_context,
                ) {
                    Ok(ExpressionEvaluationStatus::CompleteOk(result)) => result,
                    Ok(ExpressionEvaluationStatus::CompleteErr(e)) => {
                        if e.is_error() {
                            fatal_error = true;
                        }
                        diags.push(e);
                        continue;
                    }
                    Err(e) => {
                        if e.is_error() {
                            fatal_error = true;
                        }
                        diags.push(e);
                        continue;
                    }
                    Ok(ExpressionEvaluationStatus::DependencyNotComputed) => {
                        require_user_interaction = true;
                        continue;
                    }
                }
            };

            results.insert(&input.name, value);
        }
    }

    let status = match (fatal_error, require_user_interaction) {
        (false, false) => CommandInputEvaluationStatus::Complete(results),
        (true, _) => CommandInputEvaluationStatus::Aborted(results, diags),
        (false, _) => CommandInputEvaluationStatus::NeedsUserInteraction(results),
    };
    Ok(status)
}
