use crate::runbook::{RunbookExecutionMode, RunbookWorkspaceContext, RuntimeContext};
use crate::types::{RunbookExecutionContext, RunbookSources};
use kit::indexmap::IndexMap;
use kit::types::commands::CommandExecutionFuture;
use kit::types::diagnostics::DiagnosticSpan;
use kit::types::frontend::{
    ActionItemRequestUpdate, ActionItemResponse, ActionItemResponseType, Actions, Block,
    BlockEvent, ErrorPanelData, Panel,
};
use kit::types::types::RunbookSupervisionContext;
use kit::types::wallets::SigningCommandsState;
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
        types::Value,
        wallets::WalletInstance,
        ConstructDid,
    },
    uuid::Uuid,
};

// The flow for wallet evaluation should be drastically different
// Instead of activating all the wallets detected in a graph, we should instead traverse the graph and collecting the wallets
// being used.
pub async fn run_wallets_evaluation(
    runbook_workspace_context: &RunbookWorkspaceContext,
    runbook_execution_context: &mut RunbookExecutionContext,
    runtime_context: &RuntimeContext,
    supervision_context: &RunbookSupervisionContext,
    action_item_requests: &mut BTreeMap<ConstructDid, Vec<&mut ActionItemRequest>>,
    action_item_responses: &BTreeMap<ConstructDid, Vec<ActionItemResponse>>,
    progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
) -> EvaluationPassResult {
    let mut pass_result = EvaluationPassResult::new(&Uuid::new_v4());

    let wallets_instances = &runbook_execution_context.signing_commands_instances;
    let instantiated_wallets = runbook_execution_context
        .order_for_signing_commands_initialization
        .clone();

    for construct_did in instantiated_wallets.into_iter() {
        let package_id = {
            let Some(command) = runbook_execution_context
                .signing_commands_instances
                .get(&construct_did)
            else {
                continue;
            };
            command.package_id.clone()
        };

        let construct_id = &runbook_workspace_context.expect_construct_id(&construct_did);

        let instantiated =
            runbook_execution_context.is_signing_command_instantiated(&construct_did);

        let (evaluated_inputs_res, _group, addon_defaults) = match runbook_execution_context
            .signing_commands_instances
            .get(&construct_did)
        {
            None => continue,
            Some(wallet_instance) => {
                let mut cached_dependency_execution_results: HashMap<
                    ConstructDid,
                    Result<&CommandExecutionResult, &Diagnostic>,
                > = HashMap::new();

                let references_expressions = wallet_instance
                    .get_expressions_referencing_commands_from_inputs()
                    .unwrap();

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

                let addon_context_key = (package_id.did(), wallet_instance.namespace.clone());
                let addon_defaults =
                    runbook_workspace_context.get_addon_defaults(&addon_context_key);

                let res = perform_wallet_inputs_evaluation(
                    &wallet_instance,
                    &cached_dependency_execution_results,
                    &input_evaluation_results,
                    &package_id,
                    &runbook_workspace_context,
                    &runbook_execution_context,
                    runtime_context,
                );
                let group = wallet_instance.get_group();
                (res, group, addon_defaults)
            }
        };
        let mut evaluated_inputs = match evaluated_inputs_res {
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

        let wallet = runbook_execution_context
            .signing_commands_instances
            .get(&construct_did)
            .unwrap();

        let mut signing_commands_state = runbook_execution_context
            .signing_commands_state
            .take()
            .unwrap();
        signing_commands_state.create_new_wallet(&construct_did, &wallet.name);

        let res = wallet
            .check_activability(
                &construct_did,
                &mut evaluated_inputs,
                signing_commands_state,
                wallets_instances,
                &addon_defaults,
                &action_item_requests.get(&construct_did),
                &action_item_responses.get(&construct_did),
                supervision_context,
                instantiated,
                instantiated,
            )
            .await;

        let signing_commands_state = match res {
            Ok((signing_commands_state, mut new_actions)) => {
                if new_actions.has_pending_actions() {
                    runbook_execution_context.signing_commands_state = Some(signing_commands_state);
                    pass_result.actions.append(&mut new_actions);
                    continue;
                }
                pass_result.actions.append(&mut new_actions);
                signing_commands_state
            }
            Err((signing_commands_state, diag)) => {
                runbook_execution_context.signing_commands_state = Some(signing_commands_state);
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

        let res = wallet
            .perform_activation(
                &construct_did,
                &evaluated_inputs,
                signing_commands_state,
                wallets_instances,
                &addon_defaults,
                progress_tx,
            )
            .await;

        let (mut result, signing_commands_state) = match res {
            Ok((signing_commands_state, result)) => (Some(result), Some(signing_commands_state)),
            Err((signing_commands_state, diag)) => {
                runbook_execution_context.signing_commands_state = Some(signing_commands_state);
                pass_result.push_diagnostic(&diag, construct_id);
                return pass_result;
            }
        };
        runbook_execution_context.signing_commands_state = signing_commands_state;
        let Some(result) = result.take() else {
            continue;
        };
        runbook_execution_context
            .commands_execution_results
            .insert(construct_did.clone(), result);
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
        self.diagnostics
            .push(diag.clone().location(&construct_id.construct_location))
    }

    pub fn append_diagnostics(&mut self, diags: Vec<Diagnostic>, construct_id: &ConstructId) {
        diags
            .iter()
            .for_each(|diag| self.push_diagnostic(diag, construct_id))
    }

    pub fn fill_diagnostic_span(&mut self, runbook_sources: &RunbookSources) {
        for diag in self.diagnostics.iter_mut() {
            let Some(construct_location) = &diag.location else {
                continue;
            };
            let Some(span_range) = &diag.span_range() else {
                continue;
            };

            let Some((_, (_, raw_content))) = runbook_sources
                .tree
                .iter()
                .find(|(location, _)| location.eq(&construct_location))
            else {
                unimplemented!();
            };
            let mut bytes = vec![0u8; 2 * raw_content.len()];
            txtx_addon_kit::hex::encode_to_slice(raw_content, &mut bytes).unwrap();
            let mut lines = 1;
            let mut cols = 1;
            let mut span = DiagnosticSpan::new();

            let mut chars = raw_content.chars().enumerate().peekable();
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
            diag.span = Some(span)
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

    let environments_variables = runbook_workspace_context
        .environment_variables_values
        .clone();
    for (env_variable_uuid, value) in environments_variables.into_iter() {
        let mut res = CommandExecutionResult::new();
        res.outputs.insert("value".into(), value);
        runbook_execution_context
            .commands_execution_results
            .insert(env_variable_uuid, res);
    }

    for wallet_states in runbook_execution_context.signing_commands_state.iter_mut() {
        for (_, wallet) in wallet_states.store.iter_mut() {
            wallet.clear_autoincrementable_nonce();
        }
    }

    let mut genesis_dependency_execution_results = HashMap::new();
    let mut empty_result = CommandExecutionResult::new();
    empty_result
        .outputs
        .insert("value".into(), Value::bool(true));

    let mut wallets_results = HashMap::new();
    for (wallet_construct_did, _) in runbook_execution_context.signing_commands_instances.iter() {
        let mut result = CommandExecutionResult::new();
        result.outputs.insert(
            "value".into(),
            Value::string(wallet_construct_did.value().to_string()),
        );
        wallets_results.insert(wallet_construct_did.clone(), result);
    }

    for (wallet_construct_did, _) in runbook_execution_context.signing_commands_instances.iter() {
        let results: &CommandExecutionResult = wallets_results.get(wallet_construct_did).unwrap();
        genesis_dependency_execution_results.insert(wallet_construct_did.clone(), Ok(results));
    }

    let ordered_constructs = runbook_execution_context
        .order_for_commands_execution
        .clone();

    for construct_did in ordered_constructs.into_iter() {
        let Some(command_instance) = runbook_execution_context
            .commands_instances
            .get(&construct_did)
        else {
            // runtime_context.addons.index_command_instance(namespace, package_did, block)
            continue;
        };
        if let Some(_) = runbook_execution_context
            .commands_execution_results
            .get(&construct_did)
        {
            continue;
        };

        if let Some(_) = unexecutable_nodes.get(&construct_did) {
            if let Some(deps) = runbook_execution_context
                .commands_dependencies
                .get(&construct_did)
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

        // in general we want to ignore previous input evaluation results when evaluating for outputs.
        // we want to recompute the whole graph in case anything has changed since our last traversal.
        // however, if there was a start_node provided, this evaluation was initiated from a user interaction
        // that is stored in the input evaluation results, and we want to keep that data to evaluate that
        // commands dependents
        let input_evaluation_results = if supervision_context.review_input_default_values {
            None
        } else {
            runbook_execution_context
                .commands_inputs_evaluation_results
                .get(&construct_did.clone())
        };

        let mut cached_dependency_execution_results: HashMap<
            ConstructDid,
            Result<&CommandExecutionResult, &Diagnostic>,
        > = genesis_dependency_execution_results.clone();

        // Retrieve the construct_did of the inputs
        // Collect the outputs
        let references_expressions = command_instance
            .get_expressions_referencing_commands_from_inputs()
            .unwrap();

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
                        Some(Err(_)) => continue,
                        Some(Ok(_)) => {}
                    }
                }
            }
        }

        let evaluated_inputs_res = perform_inputs_evaluation(
            command_instance,
            &cached_dependency_execution_results,
            &input_evaluation_results,
            &action_item_responses.get(&construct_did),
            &package_id,
            runbook_workspace_context,
            runbook_execution_context,
            runtime_context,
            false,
        );
        let Some(command_instance) = runbook_execution_context
            .commands_instances
            .get_mut(&construct_did)
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
            let wallets = runbook_execution_context
                .signing_commands_state
                .take()
                .unwrap();
            let wallets = update_wallet_instances_from_action_response(
                wallets,
                &construct_did,
                &action_item_responses.get(&construct_did),
            );
            let res = command_instance
                .check_signed_executability(
                    &construct_did,
                    &mut evaluated_inputs,
                    wallets,
                    addon_defaults.clone(),
                    &mut runbook_execution_context.signing_commands_instances,
                    &action_item_responses.get(&construct_did),
                    &action_item_requests.get(&construct_did),
                    supervision_context,
                )
                .await;

            let wallets = match res {
                Ok((updated_wallets, mut new_actions)) => {
                    if new_actions.has_pending_actions() {
                        pass_result.actions.append(&mut new_actions);
                        runbook_execution_context.signing_commands_state = Some(updated_wallets);
                        if let Some(deps) = runbook_execution_context
                            .commands_dependencies
                            .get(&construct_did)
                        {
                            for dep in deps.iter() {
                                unexecutable_nodes.insert(dep.clone());
                            }
                        }
                        continue;
                    }
                    pass_result.actions.append(&mut new_actions);
                    updated_wallets
                }
                Err((updated_wallets, diag)) => {
                    pass_result.push_diagnostic(&diag, construct_id);
                    runbook_execution_context.signing_commands_state = Some(updated_wallets);
                    return pass_result;
                }
            };

            runbook_execution_context
                .commands_inputs_evaluation_results
                .insert(construct_did.clone(), evaluated_inputs.clone());

            let mut empty_vec = vec![];
            let action_items_requests = action_item_requests
                .get_mut(&construct_did)
                .unwrap_or(&mut empty_vec);
            let action_items_response = action_item_responses.get(&construct_did);

            let execution_result = command_instance
                .perform_signed_execution(
                    &construct_did,
                    &evaluated_inputs,
                    wallets,
                    addon_defaults.clone(),
                    &runbook_execution_context.signing_commands_instances,
                    action_items_requests,
                    &action_items_response,
                    progress_tx,
                )
                .await;

            let execution_result = match execution_result {
                // todo(lgalabru): return Diagnostic instead
                Ok((updated_wallets, result)) => {
                    runbook_execution_context.signing_commands_state = Some(updated_wallets);
                    Ok(result)
                }
                Err((updated_wallets, diag)) => {
                    runbook_execution_context.signing_commands_state = Some(updated_wallets);
                    if let Some(deps) = runbook_execution_context
                        .commands_dependencies
                        .get(&construct_did)
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
                addon_defaults.clone(),
                &mut runbook_execution_context.signing_commands_instances,
                &action_item_responses.get(&construct_did),
                supervision_context,
            ) {
                Ok(mut new_actions) => {
                    if new_actions.has_pending_actions() {
                        pass_result.actions.append(&mut new_actions);
                        if let Some(deps) = runbook_execution_context
                            .commands_dependencies
                            .get(&construct_did)
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
            let action_items_requests = action_item_requests
                .get_mut(&construct_did)
                .unwrap_or(&mut empty_vec);
            let action_items_response = action_item_responses.get(&construct_did);

            let execution_result = {
                command_instance
                    .perform_execution(
                        &construct_did,
                        &evaluated_inputs,
                        addon_defaults.clone(),
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
                    if let Some(deps) = runbook_execution_context
                        .commands_dependencies
                        .get(&construct_did)
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

        if command_instance
            .specification
            .implements_background_task_capability
        {
            let future_res = command_instance.build_background_task(
                &construct_did,
                &evaluated_inputs,
                &execution_result,
                addon_defaults.clone(),
                progress_tx,
                &pass_result.background_tasks_uuid,
                supervision_context,
            );
            let future = match future_res {
                Ok(future) => future,
                Err(diag) => {
                    pass_result.diagnostics.push(diag);
                    return pass_result;
                }
            };
            pass_result.pending_background_tasks_futures.push(future);
            pass_result
                .pending_background_tasks_constructs_uuids
                .push(construct_did.clone());
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
            let (dependency, mut components, mut subpath) = match runbook_workspace_context
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
                None => match runbook_execution_context
                    .commands_execution_results
                    .get(&dependency)
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
                BinaryOperator::Div => match rhs {
                    Value::Integer(_) => "div_int",
                    _ => "div_uint",
                },
                BinaryOperator::Eq => "eq",
                BinaryOperator::Greater => "gt",
                BinaryOperator::GreaterEq => "gte",
                BinaryOperator::Less => "lt",
                BinaryOperator::LessEq => "lte",
                BinaryOperator::Minus => "minus_uint",
                BinaryOperator::Mod => "modulo_uint",
                BinaryOperator::Mul => "multiply_uint",
                BinaryOperator::Plus => "add_uint",
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

pub fn update_wallet_instances_from_action_response(
    mut wallets: SigningCommandsState,
    construct_did: &ConstructDid,
    action_item_response: &Option<&Vec<ActionItemResponse>>,
) -> SigningCommandsState {
    match action_item_response {
        Some(responses) => responses.into_iter().for_each(
            |ActionItemResponse {
                 action_item_id: _,
                 payload,
             }| match payload {
                ActionItemResponseType::ProvideSignedTransaction(response) => {
                    if let Some(mut signing_command_state) =
                        wallets.pop_signing_command_state(&response.signer_uuid)
                    {
                        signing_command_state.insert_scoped_value(
                            &construct_did.value().to_string(),
                            "signed_transaction_bytes",
                            Value::string(response.signed_transaction_bytes.clone()),
                        );
                        wallets.push_signing_command_state(signing_command_state.clone());
                    }
                }
                ActionItemResponseType::ProvideSignedMessage(response) => {
                    if let Some(mut signing_command_state) =
                        wallets.pop_signing_command_state(&response.signer_uuid)
                    {
                        signing_command_state.insert_scoped_value(
                            &construct_did.value().to_string(),
                            "signed_message_bytes",
                            Value::string(response.signed_message_bytes.clone()),
                        );
                        wallets.push_signing_command_state(signing_command_state.clone());
                    }
                }
                _ => {}
            },
        ),
        None => {}
    }
    wallets
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
    action_item_response: &Option<&Vec<ActionItemResponse>>,
    package_id: &PackageId,
    runbook_workspace_context: &RunbookWorkspaceContext,
    runbook_execution_context: &RunbookExecutionContext,
    runtime_context: &RuntimeContext,
    simulation: bool,
) -> Result<CommandInputEvaluationStatus, Vec<Diagnostic>> {
    let mut results = CommandInputsEvaluationResult::new(&command_instance.name);
    let mut require_user_interaction = false;
    let mut diags = vec![];
    let inputs = command_instance.specification.inputs.clone();
    let mut fatal_error = false;

    match action_item_response {
        Some(responses) => responses.into_iter().for_each(
            |ActionItemResponse {
                 action_item_id: _,
                 payload,
             }| match payload {
                ActionItemResponseType::ReviewInput(_update) => {}
                ActionItemResponseType::ProvideInput(update) => {
                    results
                        .inputs
                        .insert(&update.input_name, update.updated_value.clone());
                }
                ActionItemResponseType::ProvideSignedTransaction(bytes) => {
                    results.insert(
                        "signed_transaction_bytes",
                        Value::string(bytes.signed_transaction_bytes.clone()),
                    );
                }
                ActionItemResponseType::ProvideSignedMessage(response) => {
                    results.insert(
                        "signed_message_bytes",
                        Value::string(response.signed_message_bytes.clone()),
                    );
                }
                _ => {}
            },
        ),
        None => {}
    }

    for input in inputs.into_iter() {
        if simulation && input.name.eq("signer") {
            continue;
        }
        let previously_evaluated_input = match input_evaluation_results {
            Some(input_evaluation_results) => {
                input_evaluation_results.inputs.get_value(&input.name)
            }
            None => None,
        };
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
                results.insert(&input.name, value);
                continue;
            }
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
                results.insert(&input.name, Value::object(object_values));
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

            results.insert(&input.name, value);
        } else if let Some(_) = input.as_action() {
            let value = if let Some(value) = previously_evaluated_input {
                value.clone()
            } else {
                let Some(expr) = command_instance.get_expression_from_input(&input)? else {
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
        } else {
            let value = if let Some(value) = previously_evaluated_input {
                value.clone()
            } else {
                let Some(expr) = command_instance.get_expression_from_input(&input)? else {
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

    if fatal_error {
        return Ok(CommandInputEvaluationStatus::Aborted(results, diags));
    }

    let results = command_instance
        .post_process_inputs_evaluations(results)
        .map_err(|d| vec![d])?;

    let status = match (fatal_error, require_user_interaction) {
        (false, false) => CommandInputEvaluationStatus::Complete(results),
        (_, _) => CommandInputEvaluationStatus::NeedsUserInteraction(results),
    };
    Ok(status)
}

pub fn perform_wallet_inputs_evaluation(
    wallet_instance: &WalletInstance,
    dependencies_execution_results: &HashMap<
        ConstructDid,
        Result<&CommandExecutionResult, &Diagnostic>,
    >,
    input_evaluation_results: &Option<&CommandInputsEvaluationResult>,
    package_id: &PackageId,
    runbook_workspace_context: &RunbookWorkspaceContext,
    runbook_execution_context: &RunbookExecutionContext,
    runtime_context: &RuntimeContext,
) -> Result<CommandInputEvaluationStatus, Vec<Diagnostic>> {
    let mut results = CommandInputsEvaluationResult::new(&wallet_instance.name);
    let mut require_user_interaction = false;
    let mut diags = vec![];
    let inputs = wallet_instance.specification.inputs.clone();
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
                    wallet_instance.get_expression_from_object_property(&input, &prop)?
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

            let Some(expr) = wallet_instance.get_expression_from_input(&input)? else {
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
                    let Expression::Array(exprs) = expr else {
                        panic!()
                    };
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
        } else if let Some(_) = input.as_action() {
            let value = if let Some(value) = previously_evaluated_input {
                value.clone()
            } else {
                let Some(expr) = wallet_instance.get_expression_from_input(&input)? else {
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
        } else {
            let value = if let Some(value) = previously_evaluated_input {
                value.clone()
            } else {
                let Some(expr) = wallet_instance.get_expression_from_input(&input)? else {
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
