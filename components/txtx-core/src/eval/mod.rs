use crate::types::{Runbook, RunbookExecutionContext, RunbookResolutionContext, RuntimeContext};
use daggy::{Dag, NodeIndex, Walker};
use indexmap::IndexSet;
use kit::types::commands::CommandExecutionFuture;
use kit::types::frontend::{
    ActionItemRequestUpdate, ActionItemResponse, ActionItemResponseType, Actions, Block,
    BlockEvent, ErrorPanelData, Panel,
};
use kit::types::wallets::WalletsState;
use kit::types::PackageId;
use petgraph::algo::toposort;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::fmt::Display;
use txtx_addon_kit::{
    hcl::{
        expr::{BinaryOperator, Expression, UnaryOperator},
        template::Element,
    },
    types::{
        commands::{
            CommandExecutionContext, CommandExecutionResult, CommandInputsEvaluationResult,
            CommandInstance,
        },
        diagnostics::Diagnostic,
        frontend::{
            ActionItemRequest, ActionItemRequestType, ActionItemStatus, DisplayOutputRequest,
        },
        types::{PrimitiveValue, Value},
        wallets::WalletInstance,
        ConstructUuid, PackageUuid,
    },
    uuid::Uuid,
    AddonDefaults,
};

// The flow for wallet evaluation should be drastically different
// Instead of activating all the wallets detected in a graph, we should instead traverse the graph and collecting the wallets
// being used.
pub async fn run_wallets_evaluation(
    runbook_resolution_context: &RunbookResolutionContext,
    runbook_execution_context: &mut RunbookExecutionContext,
    runtime_ctx: &mut RuntimeContext,
    execution_context: &CommandExecutionContext,
    action_item_requests: &mut BTreeMap<ConstructUuid, Vec<&mut ActionItemRequest>>,
    action_item_responses: &BTreeMap<ConstructUuid, Vec<ActionItemResponse>>,
    progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
) -> EvaluationPassResult {
    let mut pass_result = EvaluationPassResult::new(&Uuid::new_v4());
    let _ = run_commands_updating_defaults(
        runbook_resolution_context,
        runbook_execution_context,
        runtime_ctx,
        progress_tx,
    )
    .await;

    let wallets_instances = &runbook_execution_context.signing_commands_instances;
    let instantiated_wallets = runbook_resolution_context
        .instantiated_signing_commands
        .clone();
    for (construct_uuid, instantiated) in instantiated_wallets.into_iter() {
        let package_id = {
            let Some(command) = runbook_execution_context
                .signing_commands_instances
                .get(&construct_uuid)
            else {
                continue;
            };
            command.package_id.clone()
        };

        let (evaluated_inputs_res, _group, addon_defaults) = match runbook_execution_context
            .signing_commands_instances
            .get(&construct_uuid)
        {
            None => continue,
            Some(wallet_instance) => {
                let mut cached_dependency_execution_results: HashMap<
                    ConstructUuid,
                    Result<&CommandExecutionResult, &Diagnostic>,
                > = HashMap::new();

                let references_expressions = wallet_instance
                    .get_expressions_referencing_commands_from_inputs()
                    .unwrap();

                for (input, expr) in references_expressions.into_iter() {
                    let res = runbook_resolution_context
                        .try_resolve_construct_reference_in_expression(
                            &package_id,
                            &expr,
                            &runbook_execution_context,
                        )
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
                                    pass_result.diagnostics.push((*diag).clone());
                                    continue;
                                }
                                Some(Ok(_)) => {}
                            }
                        }
                    }
                }

                let input_evaluation_results = runbook_execution_context
                    .commands_inputs_evaluations_results
                    .get(&construct_uuid.clone());

                let addon_context_key = (package_id.did(), wallet_instance.namespace.clone());
                let addon_defaults = runtime_ctx
                    .addons_ctx
                    .contexts
                    .get(&addon_context_key)
                    .and_then(|addon| Some(addon.defaults.clone()))
                    .unwrap_or(AddonDefaults::new());

                let res = perform_wallet_inputs_evaluation(
                    &wallet_instance,
                    &cached_dependency_execution_results,
                    &input_evaluation_results,
                    &package_id,
                    &runbook_resolution_context,
                    &runbook_execution_context,
                    runtime_ctx,
                );
                let group = wallet_instance.get_group();
                (res, group, addon_defaults)
            }
        };
        let mut evaluated_inputs = match evaluated_inputs_res {
            Ok(result) => match result {
                CommandInputEvaluationStatus::Complete(result) => result,
                CommandInputEvaluationStatus::NeedsUserInteraction => {
                    continue;
                }
                CommandInputEvaluationStatus::Aborted(mut diags) => {
                    pass_result.diagnostics.append(&mut diags);
                    continue;
                }
            },
            Err(mut diags) => {
                pass_result.diagnostics.append(&mut diags);
                return pass_result;
            }
        };

        let wallet = runbook_execution_context
            .signing_commands_instances
            .get(&construct_uuid)
            .unwrap();
        let mut wallets_state = runbook_execution_context
            .signing_commands_state
            .take()
            .unwrap();
        wallets_state.create_new_wallet(&construct_uuid, &wallet.name);

        let res = wallet
            .check_activability(
                &construct_uuid,
                &mut evaluated_inputs,
                wallets_state,
                wallets_instances,
                &addon_defaults,
                &action_item_requests.get(&construct_uuid),
                &action_item_responses.get(&construct_uuid),
                execution_context,
                instantiated,
                instantiated,
            )
            .await;

        let wallets_state = match res {
            Ok((wallets_state, mut new_actions)) => {
                if new_actions.has_pending_actions() {
                    runbook_execution_context.signing_commands_state = Some(wallets_state);
                    pass_result.actions.append(&mut new_actions);
                    continue;
                }
                pass_result.actions.append(&mut new_actions);
                wallets_state
            }
            Err((wallets_state, diag)) => {
                runbook_execution_context.signing_commands_state = Some(wallets_state);
                if let Some(requests) = action_item_requests.get_mut(&construct_uuid) {
                    for item in requests.iter_mut() {
                        // This should be improved / become more granular
                        let update = ActionItemRequestUpdate::from_id(&item.id)
                            .set_status(ActionItemStatus::Error(diag.clone()));
                        pass_result.actions.push_action_item_update(update);
                    }
                }
                pass_result.diagnostics.push(diag.clone());
                return pass_result;
            }
        };

        runbook_execution_context
            .commands_inputs_evaluations_results
            .insert(construct_uuid.clone(), evaluated_inputs.clone());

        let res = wallet
            .perform_activation(
                &construct_uuid,
                &evaluated_inputs,
                wallets_state,
                wallets_instances,
                &addon_defaults,
                progress_tx,
            )
            .await;

        let (mut result, wallets_state) = match res {
            Ok((wallets_state, result)) => (Some(result), Some(wallets_state)),
            Err((wallets_state, diag)) => {
                pass_result.diagnostics.push(diag);
                return pass_result;
            }
        };
        runbook_execution_context.signing_commands_state = wallets_state;
        let Some(result) = result.take() else {
            continue;
        };
        runbook_execution_context
            .commands_execution_results
            .insert(construct_uuid.clone(), result);
    }

    pass_result
}

pub async fn run_commands_updating_defaults(
    runbook_resolution_context: &RunbookResolutionContext,
    runbook_execution_context: &mut RunbookExecutionContext,
    runtime_ctx: &mut RuntimeContext,
    progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
) -> Result<(), Vec<Diagnostic>> {
    // Update environment variables
    let environments_variables = runbook_resolution_context
        .environment_variables_values
        .clone();
    for (env_variable_uuid, value) in environments_variables.into_iter() {
        let mut res = CommandExecutionResult::new();
        res.outputs.insert("value".into(), Value::string(value));
        runbook_execution_context
            .commands_execution_results
            .insert(env_variable_uuid, res);
    }

    let mut cached_dependency_execution_results = HashMap::new();

    let input_evaluation_results = None;

    // Run commands updating defaults
    let construct_uuids = runbook_execution_context
        .commands_instances
        .iter()
        .map(|(c, _)| c.clone())
        .collect::<Vec<_>>();

    for construct_uuid in construct_uuids.iter() {
        let evaluated_inputs_res = {
            let command_instance = runbook_execution_context
                .commands_instances
                .get(construct_uuid)
                .unwrap();
            if !command_instance.specification.update_addon_defaults {
                continue;
            }

            let package_id = &command_instance.package_id.clone();

            // Retrieve the construct_uuid of the inputs
            // Collect the outputs
            let references_expressions = command_instance
                .get_expressions_referencing_commands_from_inputs()
                .unwrap();

            for (input, expr) in references_expressions.into_iter() {
                let res = runbook_resolution_context
                    .try_resolve_construct_reference_in_expression(
                        &command_instance.package_id,
                        &expr,
                        &runbook_execution_context,
                    )
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
                &None,
                &package_id,
                &runbook_resolution_context,
                &runbook_execution_context,
                runtime_ctx,
            );
            evaluated_inputs_res
        };

        let _execution_result = {
            let Some(command_instance) = runbook_execution_context
                .commands_instances
                .get_mut(&construct_uuid)
            else {
                // runtime_ctx.addons.index_command_instance(namespace, package_uuid, block)
                continue;
            };

            let addon_context_key = (
                command_instance.package_id.did(),
                command_instance.namespace.clone(),
            );

            let addon_defaults = runtime_ctx
                .addons_ctx
                .contexts
                .get(&addon_context_key)
                .and_then(|addon| Some(addon.defaults.clone()))
                .unwrap_or(AddonDefaults::new()); // todo(lgalabru): to investigate

            let mut evaluated_inputs = CommandInputsEvaluationResult::new(&command_instance.name);

            match evaluated_inputs_res {
                Ok(result) => match result {
                    CommandInputEvaluationStatus::Complete(result) => evaluated_inputs = result,
                    CommandInputEvaluationStatus::NeedsUserInteraction => {}
                    CommandInputEvaluationStatus::Aborted(diags) => return Err(diags),
                },
                Err(diags) => return Err(diags),
            };

            {
                runbook_execution_context
                    .commands_inputs_evaluations_results
                    .insert(construct_uuid.clone(), evaluated_inputs.clone());
            }

            let execution_result = {
                command_instance
                    .perform_execution(
                        &construct_uuid,
                        &evaluated_inputs,
                        addon_defaults.clone(),
                        &mut vec![],
                        &None,
                        progress_tx,
                    )
                    .await
            };

            let execution_result = match execution_result {
                // todo(lgalabru): return Diagnostic instead
                Ok(result) => {
                    if command_instance.specification.update_addon_defaults {
                        let addon_context_key = (
                            command_instance.package_id.did(),
                            command_instance.namespace.clone(),
                        );
                        if let Some(ref mut addon_context) =
                            runtime_ctx.addons_ctx.contexts.get_mut(&addon_context_key)
                        {
                            for (k, v) in result.outputs.iter() {
                                addon_context.defaults.keys.insert(k.clone(), v.to_string());
                            }
                        }
                    }
                    Ok(result)
                }
                Err(e) => Err(e),
            };
            execution_result
        };
    }
    Ok(())
}

pub struct EvaluationPassResult {
    pub actions: Actions,
    pub diagnostics: Vec<Diagnostic>,
    pub pending_background_tasks_futures: Vec<CommandExecutionFuture>,
    pub pending_background_tasks_constructs_uuids: Vec<ConstructUuid>,
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
    runbook_resolution_context: &RunbookResolutionContext,
    runbook_execution_context: &mut RunbookExecutionContext,
    runtime_ctx: &mut RuntimeContext,
    execution_context: &CommandExecutionContext,
    action_item_requests: &mut BTreeMap<ConstructUuid, Vec<&mut ActionItemRequest>>,
    action_item_responses: &BTreeMap<ConstructUuid, Vec<ActionItemResponse>>,
    progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
) -> EvaluationPassResult {
    let mut pass_result = EvaluationPassResult::new(background_tasks_uuid);

    let mut unexecutable_nodes: HashSet<ConstructUuid> = HashSet::new();

    let environments_variables = runbook_resolution_context
        .environment_variables_values
        .clone();
    for (env_variable_uuid, value) in environments_variables.into_iter() {
        let mut res = CommandExecutionResult::new();
        res.outputs.insert("value".into(), Value::string(value));
        runbook_execution_context
            .commands_execution_results
            .insert(env_variable_uuid, res);
    }

    let mut genesis_dependency_execution_results = HashMap::new();
    let mut empty_result = CommandExecutionResult::new();
    empty_result
        .outputs
        .insert("value".into(), Value::bool(true));

    let mut wallets_results = HashMap::new();
    for (wallet_construct_uuid, _) in runbook_execution_context.signing_commands_instances.iter() {
        let mut result = CommandExecutionResult::new();
        result.outputs.insert(
            "value".into(),
            Value::string(wallet_construct_uuid.value().to_string()),
        );
        wallets_results.insert(wallet_construct_uuid.clone(), result);
    }

    for (wallet_construct_uuid, _) in runbook_execution_context.signing_commands_instances.iter() {
        let results = wallets_results.get(wallet_construct_uuid).unwrap();
        genesis_dependency_execution_results.insert(wallet_construct_uuid.clone(), Ok(results));
    }

    let ordered_constructs = runbook_execution_context
        .ordered_executable_constructs
        .clone();

    for construct_uuid in ordered_constructs.into_iter() {
        let Some(command_instance) = runbook_execution_context
            .commands_instances
            .get(&construct_uuid)
        else {
            // runtime_ctx.addons.index_command_instance(namespace, package_uuid, block)
            continue;
        };
        if let Some(_) = runbook_execution_context
            .commands_execution_results
            .get(&construct_uuid)
        {
            continue;
        };

        if let Some(_) = unexecutable_nodes.get(&construct_uuid) {
            if let Some(deps) = runbook_execution_context
                .commands_dependencies
                .get(&construct_uuid)
            {
                for dep in deps.iter() {
                    unexecutable_nodes.insert(dep.clone());
                }
            }
            continue;
        }

        let package_id = command_instance.package_id.clone();

        let addon_context_key = (package_id.did(), command_instance.namespace.clone());
        let addon_defaults = runtime_ctx
            .addons_ctx
            .contexts
            .get(&addon_context_key)
            .and_then(|addon| Some(addon.defaults.clone()))
            .unwrap_or(AddonDefaults::new()); // todo(lgalabru): to investigate

        // in general we want to ignore previous input evaluation results when evaluating for outputs.
        // we want to recompute the whole graph in case anything has changed since our last traversal.
        // however, if there was a start_node provided, this evaluation was initiated from a user interaction
        // that is stored in the input evaluation results, and we want to keep that data to evaluate that
        // commands dependents
        let input_evaluation_results = if execution_context.review_input_default_values {
            None
        } else {
            runbook_execution_context
                .commands_inputs_evaluations_results
                .get(&construct_uuid.clone())
        };

        let mut cached_dependency_execution_results: HashMap<
            ConstructUuid,
            Result<&CommandExecutionResult, &Diagnostic>,
        > = genesis_dependency_execution_results.clone();

        // Retrieve the construct_uuid of the inputs
        // Collect the outputs
        let references_expressions = command_instance
            .get_expressions_referencing_commands_from_inputs()
            .unwrap();

        for (input, expr) in references_expressions.into_iter() {
            let res = runbook_resolution_context
                .try_resolve_construct_reference_in_expression(
                    &package_id,
                    &expr,
                    &runbook_execution_context,
                )
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
            &action_item_responses.get(&construct_uuid),
            &package_id,
            runbook_resolution_context,
            runbook_execution_context,
            runtime_ctx,
        );
        let Some(command_instance) = runbook_execution_context
            .commands_instances
            .get_mut(&construct_uuid)
        else {
            // runtime_ctx.addons.index_command_instance(namespace, package_uuid, block)
            continue;
        };

        let mut evaluated_inputs = CommandInputsEvaluationResult::new(&command_instance.name);

        match evaluated_inputs_res {
            Ok(result) => match result {
                CommandInputEvaluationStatus::Complete(result) => evaluated_inputs = result,
                CommandInputEvaluationStatus::NeedsUserInteraction => continue,
                CommandInputEvaluationStatus::Aborted(mut diags) => {
                    pass_result.diagnostics.append(&mut diags);
                    return pass_result;
                }
            },
            Err(mut diags) => {
                pass_result.diagnostics.append(&mut diags);
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
                &construct_uuid,
                &action_item_responses.get(&construct_uuid),
            );
            let res = command_instance
                .check_signed_executability(
                    &construct_uuid,
                    &mut evaluated_inputs,
                    wallets,
                    addon_defaults.clone(),
                    &mut runbook_execution_context.signing_commands_instances,
                    &action_item_responses.get(&construct_uuid),
                    &action_item_requests.get(&construct_uuid),
                    execution_context,
                )
                .await;

            let wallets = match res {
                Ok((updated_wallets, mut new_actions)) => {
                    if new_actions.has_pending_actions() {
                        pass_result.actions.append(&mut new_actions);
                        runbook_execution_context.signing_commands_state = Some(updated_wallets);
                        if let Some(deps) = runbook_execution_context
                            .commands_dependencies
                            .get(&construct_uuid)
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
                    pass_result.diagnostics.push(diag);
                    runbook_execution_context.signing_commands_state = Some(updated_wallets);
                    return pass_result;
                }
            };

            runbook_execution_context
                .commands_inputs_evaluations_results
                .insert(construct_uuid.clone(), evaluated_inputs.clone());

            let mut empty_vec = vec![];
            let action_items_requests = action_item_requests
                .get_mut(&construct_uuid)
                .unwrap_or(&mut empty_vec);
            let action_items_response = action_item_responses.get(&construct_uuid);

            let execution_result = command_instance
                .perform_signed_execution(
                    &construct_uuid,
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
                    if command_instance.specification.update_addon_defaults {
                        let addon_context_key =
                            (package_id.did(), command_instance.namespace.clone());
                        if let Some(ref mut addon_context) =
                            runtime_ctx.addons_ctx.contexts.get_mut(&addon_context_key)
                        {
                            for (k, v) in result.outputs.iter() {
                                addon_context.defaults.keys.insert(k.clone(), v.to_string());
                            }
                        }
                    }
                    runbook_execution_context.signing_commands_state = Some(updated_wallets);
                    Ok(result)
                }
                Err((updated_wallets, diag)) => {
                    runbook_execution_context.signing_commands_state = Some(updated_wallets);
                    if let Some(deps) = runbook_execution_context
                        .commands_dependencies
                        .get(&construct_uuid)
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
                &construct_uuid,
                &mut evaluated_inputs,
                addon_defaults.clone(),
                &mut runbook_execution_context.signing_commands_instances,
                &action_item_responses.get(&construct_uuid),
                execution_context,
            ) {
                Ok(mut new_actions) => {
                    if new_actions.has_pending_actions() {
                        pass_result.actions.append(&mut new_actions);
                        if let Some(deps) = runbook_execution_context
                            .commands_dependencies
                            .get(&construct_uuid)
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
                    pass_result.diagnostics.push(diag);
                    return pass_result;
                }
            }

            runbook_execution_context
                .commands_inputs_evaluations_results
                .insert(construct_uuid.clone(), evaluated_inputs.clone());

            let mut empty_vec = vec![];
            let action_items_requests = action_item_requests
                .get_mut(&construct_uuid)
                .unwrap_or(&mut empty_vec);
            let action_items_response = action_item_responses.get(&construct_uuid);

            let execution_result = {
                command_instance
                    .perform_execution(
                        &construct_uuid,
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
                Ok(result) => {
                    if command_instance.specification.update_addon_defaults {
                        let addon_context_key =
                            (package_id.did(), command_instance.namespace.clone());
                        if let Some(ref mut addon_context) =
                            runtime_ctx.addons_ctx.contexts.get_mut(&addon_context_key)
                        {
                            for (k, v) in result.outputs.iter() {
                                addon_context.defaults.keys.insert(k.clone(), v.to_string());
                            }
                        }
                    }
                    Ok(result)
                }
                Err(e) => {
                    if let Some(deps) = runbook_execution_context
                        .commands_dependencies
                        .get(&construct_uuid)
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
                pass_result.diagnostics.push(diag);
                continue;
            }
        };

        if command_instance
            .specification
            .implements_background_task_capability
        {
            let future_res = command_instance.build_background_task(
                &construct_uuid,
                &evaluated_inputs,
                &execution_result,
                addon_defaults.clone(),
                progress_tx,
                &pass_result.background_tasks_uuid,
                execution_context,
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
                .push(construct_uuid.clone());
        }
        runbook_execution_context
            .commands_execution_results
            .entry(construct_uuid)
            .or_insert_with(CommandExecutionResult::new)
            .append(&mut execution_result);
    }
    pass_result
}

pub enum ExpressionEvaluationStatus {
    CompleteOk(Value),
    CompleteErr(Diagnostic),
    DependencyNotComputed,
}

pub fn eval_expression(
    expr: &Expression,
    dependencies_execution_results: &HashMap<
        ConstructUuid,
        Result<&CommandExecutionResult, &Diagnostic>,
    >,
    package_id: &PackageId,
    runbook_resolution_context: &RunbookResolutionContext,
    runbook_execution_context: &RunbookExecutionContext,
    runtime_ctx: &RuntimeContext,
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
                (Some(value), _, _) => Value::uint(value),
                (_, Some(value), _) => Value::int(value),
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
                    runbook_resolution_context,
                    runbook_execution_context,
                    runtime_ctx,
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
            let mut map = HashMap::new();
            for (k, v) in object.into_iter() {
                let key = match k {
                    txtx_addon_kit::hcl::expr::ObjectKey::Expression(k_expr) => {
                        match eval_expression(
                            k_expr,
                            dependencies_execution_results,
                            package_id,
                            runbook_resolution_context,
                            runbook_execution_context,
                            runtime_ctx,
                        )? {
                            ExpressionEvaluationStatus::CompleteOk(result) => match result {
                                Value::Primitive(PrimitiveValue::String(result)) => result,
                                Value::Primitive(_)
                                | Value::Addon(_)
                                | Value::Array(_)
                                | Value::Object(_) => {
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
                    runbook_resolution_context,
                    runbook_execution_context,
                    runtime_ctx,
                )? {
                    ExpressionEvaluationStatus::CompleteOk(result) => Ok(result),
                    ExpressionEvaluationStatus::CompleteErr(e) => Err(e),
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
                            runbook_resolution_context,
                            runbook_execution_context,
                            runtime_ctx,
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
                    runbook_resolution_context,
                    runbook_execution_context,
                    runtime_ctx,
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
            runtime_ctx
                .execute_function(package_id.did(), func_namespace, &func_name, &args)
                .map_err(|e| e)?
        }
        // Represents an attribute or element traversal.
        Expression::Traversal(_) => {
            let Ok(Some((dependency, mut components, mut subpath))) = runbook_resolution_context
                .try_resolve_construct_reference_in_expression(
                    package_id,
                    expr,
                    runbook_execution_context,
                )
            else {
                todo!("implement diagnostic for unresolvable references")
            };
            let res: &CommandExecutionResult = match dependencies_execution_results.get(&dependency)
            {
                Some(res) => match res.clone() {
                    Ok(res) => res,
                    Err(e) => return Ok(ExpressionEvaluationStatus::CompleteErr(e.clone())),
                },
                None => return Ok(ExpressionEvaluationStatus::DependencyNotComputed),
            };

            let attribute = components.pop_front().unwrap_or("value".into());

            match res.outputs.get(&attribute) {
                Some(output) => {
                    if let Some(ref object) = output.as_object() {
                        if let Some(key) = subpath.pop_front() {
                            object
                                .get(&key.to_string())
                                .as_ref()
                                .clone()
                                .unwrap()
                                .as_ref()
                                .unwrap()
                                .clone()
                        } else {
                            output.clone()
                        }
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
                runbook_resolution_context,
                runbook_execution_context,
                runtime_ctx,
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
                runbook_resolution_context,
                runbook_execution_context,
                runtime_ctx,
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
                runbook_resolution_context,
                runbook_execution_context,
                runtime_ctx,
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
                    Value::Primitive(PrimitiveValue::SignedInteger(_)) => "div_int",
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
            runtime_ctx.execute_function(package_id.did(), None, func, &vec![lhs, rhs])?
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
    mut wallets: WalletsState,
    construct_uuid: &ConstructUuid,
    action_item_response: &Option<&Vec<ActionItemResponse>>,
) -> WalletsState {
    match action_item_response {
        Some(responses) => responses.into_iter().for_each(
            |ActionItemResponse {
                 action_item_id: _,
                 payload,
             }| match payload {
                ActionItemResponseType::ProvideSignedTransaction(response) => {
                    if let Some(mut wallet_state) = wallets.pop_wallet_state(&response.signer_uuid)
                    {
                        wallet_state.insert_scoped_value(
                            &construct_uuid.value().to_string(),
                            "signed_transaction_bytes",
                            Value::string(response.signed_transaction_bytes.clone()),
                        );
                        wallets.push_wallet_state(wallet_state.clone());
                    }
                }
                ActionItemResponseType::ProvideSignedMessage(response) => {
                    if let Some(mut wallet_state) = wallets.pop_wallet_state(&response.signer_uuid)
                    {
                        wallet_state.insert_scoped_value(
                            &construct_uuid.value().to_string(),
                            "signed_message_bytes",
                            Value::string(response.signed_message_bytes.clone()),
                        );
                        wallets.push_wallet_state(wallet_state.clone());
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
    NeedsUserInteraction,
    Aborted(Vec<Diagnostic>),
}

pub fn perform_inputs_evaluation(
    command_instance: &CommandInstance,
    dependencies_execution_results: &HashMap<
        ConstructUuid,
        Result<&CommandExecutionResult, &Diagnostic>,
    >,
    input_evaluation_results: &Option<&CommandInputsEvaluationResult>,
    action_item_response: &Option<&Vec<ActionItemResponse>>,
    package_id: &PackageId,
    runbook_resolution_context: &RunbookResolutionContext,
    runbook_execution_context: &RunbookExecutionContext,
    runtime_ctx: &RuntimeContext,
) -> Result<CommandInputEvaluationStatus, Vec<Diagnostic>> {
    let mut results = CommandInputsEvaluationResult::new(&command_instance.name);
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
        // todo(micaiah): this value still needs to be for inputs that are objects
        let previously_evaluated_input = match input_evaluation_results {
            Some(input_evaluation_results) => {
                input_evaluation_results.inputs.get_value(&input.name)
            }
            None => None,
        };
        if let Some(object_props) = input.as_object() {
            // todo(micaiah) - figure out how user-input values work for this branch
            let mut object_values = HashMap::new();
            for prop in object_props.iter() {
                if let Some(value) = previously_evaluated_input {
                    match value.clone() {
                        Value::Object(obj) => {
                            for (k, v) in obj.into_iter() {
                                object_values.insert(k, v);
                            }
                        }
                        v => {
                            object_values.insert(prop.name.to_string(), Ok(v));
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
                    runbook_resolution_context,
                    runbook_execution_context,
                    runtime_ctx,
                ) {
                    Ok(ExpressionEvaluationStatus::CompleteOk(result)) => Ok(result),
                    Ok(ExpressionEvaluationStatus::CompleteErr(e)) => Err(e),
                    Err(e) => Err(e),
                    Ok(ExpressionEvaluationStatus::DependencyNotComputed) => {
                        return Ok(CommandInputEvaluationStatus::NeedsUserInteraction);
                    }
                };

                if let Err(ref diag) = value {
                    diags.push(diag.clone());
                    if diag.is_error() {
                        fatal_error = true;
                    }
                }

                match value.clone() {
                    Ok(Value::Object(obj)) => {
                        for (k, v) in obj.into_iter() {
                            object_values.insert(k, v);
                        }
                    }
                    Ok(v) => {
                        object_values.insert(prop.name.to_string(), Ok(v));
                    }
                    Err(diag) => {
                        object_values.insert(prop.name.to_string(), Err(diag));
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
                    Value::Primitive(_) | Value::Object(_) | Value::Addon(_) => {
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
                runbook_resolution_context,
                runbook_execution_context,
                runtime_ctx,
            ) {
                Ok(ExpressionEvaluationStatus::CompleteOk(result)) => match result {
                    Value::Primitive(_) | Value::Object(_) | Value::Addon(_) => unreachable!(),
                    Value::Array(entries) => {
                        for (i, entry) in entries.into_iter().enumerate() {
                            array_values.insert(i, entry); // todo: is it okay that we possibly overwrite array values from previous input evals?
                        }
                        Value::array(array_values)
                    }
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
                    return Ok(CommandInputEvaluationStatus::NeedsUserInteraction);
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
                    runbook_resolution_context,
                    runbook_execution_context,
                    runtime_ctx,
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
                        return Ok(CommandInputEvaluationStatus::NeedsUserInteraction);
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
                    runbook_resolution_context,
                    runbook_execution_context,
                    runtime_ctx,
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
                        return Ok(CommandInputEvaluationStatus::NeedsUserInteraction);
                    }
                }
            };
            results.insert(&input.name, value);
        }
    }

    let status = match fatal_error {
        false => CommandInputEvaluationStatus::Complete(results),
        true => CommandInputEvaluationStatus::Aborted(diags),
    };
    Ok(status)
}

pub fn perform_wallet_inputs_evaluation(
    wallet_instance: &WalletInstance,
    dependencies_execution_results: &HashMap<
        ConstructUuid,
        Result<&CommandExecutionResult, &Diagnostic>,
    >,
    input_evaluation_results: &Option<&CommandInputsEvaluationResult>,
    package_id: &PackageId,
    runbook_resolution_context: &RunbookResolutionContext,
    runbook_execution_context: &RunbookExecutionContext,
    runtime_ctx: &RuntimeContext,
) -> Result<CommandInputEvaluationStatus, Vec<Diagnostic>> {
    let mut results = CommandInputsEvaluationResult::new(&wallet_instance.name);
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
            let mut object_values = HashMap::new();
            for prop in object_props.iter() {
                if let Some(value) = previously_evaluated_input {
                    match value.clone() {
                        Value::Object(obj) => {
                            for (k, v) in obj.into_iter() {
                                object_values.insert(k, v);
                            }
                        }
                        v => {
                            object_values.insert(prop.name.to_string(), Ok(v));
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
                    runbook_resolution_context,
                    runbook_execution_context,
                    runtime_ctx,
                ) {
                    Ok(ExpressionEvaluationStatus::CompleteOk(result)) => Ok(result),
                    Ok(ExpressionEvaluationStatus::CompleteErr(e)) => Err(e),
                    Err(e) => Err(e),
                    Ok(ExpressionEvaluationStatus::DependencyNotComputed) => {
                        return Ok(CommandInputEvaluationStatus::NeedsUserInteraction);
                    }
                };

                if let Err(ref diag) = value {
                    diags.push(diag.clone());
                    if diag.is_error() {
                        fatal_error = true;
                    }
                }

                match value.clone() {
                    Ok(Value::Object(obj)) => {
                        for (k, v) in obj.into_iter() {
                            object_values.insert(k, v);
                        }
                    }
                    Ok(v) => {
                        object_values.insert(prop.name.to_string(), Ok(v));
                    }
                    Err(diag) => {
                        object_values.insert(prop.name.to_string(), Err(diag));
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
                    Value::Primitive(_) | Value::Object(_) | Value::Addon(_) => {
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
                runbook_resolution_context,
                runbook_execution_context,
                runtime_ctx,
            ) {
                Ok(ExpressionEvaluationStatus::CompleteOk(result)) => match result {
                    Value::Primitive(_) | Value::Object(_) | Value::Addon(_) => unreachable!(),
                    Value::Array(entries) => {
                        for (i, entry) in entries.into_iter().enumerate() {
                            array_values.insert(i, entry); // todo: is it okay that we possibly overwrite array values from previous input evals?
                        }
                        Value::array(array_values)
                    }
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
                        let result = runbook_resolution_context
                            .try_resolve_construct_reference_in_expression(
                                package_id,
                                &expr,
                                runbook_execution_context,
                            );
                        if let Ok(Some((construct_uuid, _, _))) = result {
                            references.push(Value::string(construct_uuid.value().to_string()));
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
                    runbook_resolution_context,
                    runbook_execution_context,
                    runtime_ctx,
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
                        return Ok(CommandInputEvaluationStatus::NeedsUserInteraction);
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
                    runbook_resolution_context,
                    runbook_execution_context,
                    runtime_ctx,
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
                        return Ok(CommandInputEvaluationStatus::NeedsUserInteraction);
                    }
                }
            };

            results.insert(&input.name, value);
        }
    }

    let status = match fatal_error {
        false => CommandInputEvaluationStatus::Complete(results),
        true => CommandInputEvaluationStatus::Aborted(diags),
    };
    Ok(status)
}
