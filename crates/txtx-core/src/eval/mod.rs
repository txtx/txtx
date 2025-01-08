use crate::runbook::embedded_runbook::ExecutableEmbeddedRunbookInstance;
use crate::runbook::{
    get_source_context_for_diagnostic, RunbookExecutionMode, RunbookWorkspaceContext,
    RuntimeContext,
};
use crate::types::{RunbookExecutionContext, RunbookSources};
use kit::constants::{
    SIGNATURE_APPROVED, SIGNATURE_SKIPPABLE, SIGNED_MESSAGE_BYTES, SIGNED_TRANSACTION_BYTES,
    TX_HASH,
};
use kit::hcl::structure::Block as HclBlock;
use kit::helpers::hcl::visit_optional_untyped_attribute;
use kit::indexmap::IndexMap;
use kit::types::commands::{
    add_ctx_to_diag, add_ctx_to_embedded_runbook_diag, CommandExecutionFuture,
    DependencyExecutionResultCache, UnevaluatedInputsMap,
};
use kit::types::embedded_runbooks::EmbeddedRunbookStatefulExecutionContext;
use kit::types::frontend::{
    ActionItemRequestUpdate, ActionItemResponse, ActionItemResponseType, Actions, Block,
    BlockEvent, ErrorPanelData, Panel,
};
use kit::types::signers::SignersState;
use kit::types::stores::AddonDefaults;
use kit::types::types::{ObjectProperty, RunbookSupervisionContext, Type};
use kit::types::{ConstructId, PackageId};
use kit::types::{EvaluatableInput, WithEvaluatableInputs};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Display;
use txtx_addon_kit::{
    hcl::{
        expr::{BinaryOperator, Expression, UnaryOperator},
        template::Element,
    },
    types::{
        commands::{CommandExecutionResult, CommandInputsEvaluationResult},
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

        let Some(signer_instance) = runbook_execution_context.signers_instances.get(&construct_did)
        else {
            continue;
        };

        let add_ctx_to_diag = add_ctx_to_diag(
            "signer".to_string(),
            signer_instance.specification.matcher.clone(),
            signer_instance.name.clone(),
            signer_instance.namespace.clone(),
        );

        let mut cached_dependency_execution_results = DependencyExecutionResultCache::new();

        let references_expressions =
            signer_instance.get_expressions_referencing_commands_from_inputs().unwrap();

        for (_input, expr) in references_expressions.into_iter() {
            if let Some((dependency, _, _)) = runbook_workspace_context
                .try_resolve_construct_reference_in_expression(&package_id, &expr)
                .unwrap()
            {
                if let Some(evaluation_result) =
                    runbook_execution_context.commands_execution_results.get(&dependency)
                {
                    match cached_dependency_execution_results.merge(&dependency, &evaluation_result)
                    {
                        Ok(_) => {}
                        Err(diag) => {
                            pass_result.push_diagnostic(&diag, construct_id, &add_ctx_to_diag);
                            continue;
                        }
                    }
                }
            }
        }

        let input_evaluation_results = runbook_execution_context
            .commands_inputs_evaluation_results
            .get(&construct_did.clone());

        let addon_context_key = (package_id.did(), signer_instance.namespace.clone());
        let addon_defaults = runbook_workspace_context.get_addon_defaults(&addon_context_key);

        let evaluated_inputs_res = perform_signer_inputs_evaluation(
            &signer_instance,
            &cached_dependency_execution_results,
            &input_evaluation_results,
            addon_defaults,
            &package_id,
            &runbook_workspace_context,
            &runbook_execution_context,
            runtime_context,
        );

        let evaluated_inputs = match evaluated_inputs_res {
            Ok(result) => match result {
                CommandInputEvaluationStatus::Complete(result) => result,
                CommandInputEvaluationStatus::NeedsUserInteraction(_) => {
                    continue;
                }
                CommandInputEvaluationStatus::Aborted(_, diags) => {
                    pass_result.append_diagnostics(diags, construct_id, &add_ctx_to_diag);
                    continue;
                }
            },
            Err(diags) => {
                pass_result.append_diagnostics(diags, construct_id, &add_ctx_to_diag);
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
                &runtime_context.authorization_context,
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
                println!("check activability diag: {:?}", diag);
                pass_result.push_diagnostic(&diag, construct_id, &add_ctx_to_diag);
                println!("pass results diags: {:?}", pass_result.diagnostics);
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
                pass_result.push_diagnostic(&diag, construct_id, &add_ctx_to_diag);
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

    pub fn merge(&mut self, mut other: EvaluationPassResult) {
        self.actions.append(&mut other.actions);
        self.diagnostics.append(&mut other.diagnostics);
        self.pending_background_tasks_futures.append(&mut other.pending_background_tasks_futures);
        self.pending_background_tasks_constructs_uuids
            .append(&mut other.pending_background_tasks_constructs_uuids);
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

    pub fn push_diagnostic(
        &mut self,
        diag: &Diagnostic,
        construct_id: &ConstructId,
        ctx_adder: &impl Fn(&Diagnostic) -> Diagnostic,
    ) {
        self.diagnostics.push(ctx_adder(diag).location(&construct_id.construct_location))
    }

    pub fn append_diagnostics(
        &mut self,
        diags: Vec<Diagnostic>,
        construct_id: &ConstructId,
        ctx_adder: &impl Fn(&Diagnostic) -> Diagnostic,
    ) {
        diags.iter().for_each(|diag| self.push_diagnostic(diag, construct_id, &ctx_adder))
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

    let mut genesis_dependency_execution_results = DependencyExecutionResultCache::new();

    let mut signers_results = HashMap::new();
    for (signer_construct_did, _) in runbook_execution_context.signers_instances.iter() {
        let mut result = CommandExecutionResult::new();
        result
            .outputs
            .insert("value".into(), Value::string(signer_construct_did.value().to_string()));
        signers_results.insert(signer_construct_did.clone(), result);
    }

    for (signer_construct_did, _) in runbook_execution_context.signers_instances.iter() {
        let results = signers_results.get(signer_construct_did).unwrap();
        genesis_dependency_execution_results
            .insert(signer_construct_did.clone(), Ok(results.clone()));
    }

    let ordered_constructs = runbook_execution_context.order_for_commands_execution.clone();

    for construct_did in ordered_constructs.into_iter() {
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

        if let Some(_) = runbook_execution_context.commands_instances.get(&construct_did) {
            match evaluate_command_instance(
                &construct_did,
                &mut pass_result,
                &mut unexecutable_nodes,
                &mut genesis_dependency_execution_results,
                runbook_workspace_context,
                runbook_execution_context,
                runtime_context,
                supervision_context,
                action_item_requests,
                action_item_responses,
                progress_tx,
            )
            .await
            {
                LoopEvaluationResult::Continue => continue,
                LoopEvaluationResult::Bail => {
                    return pass_result;
                }
            }
        }

        if let Some(_) = runbook_execution_context.embedded_runbooks.get(&construct_did) {
            match evaluate_embedded_runbook_instance(
                background_tasks_uuid,
                &construct_did,
                &mut pass_result,
                &mut unexecutable_nodes,
                &mut genesis_dependency_execution_results,
                runbook_workspace_context,
                runbook_execution_context,
                runtime_context,
                supervision_context,
                action_item_requests,
                action_item_responses,
                progress_tx,
            )
            .await
            {
                LoopEvaluationResult::Continue => continue,
                LoopEvaluationResult::Bail => {
                    return pass_result;
                }
            }
        }
    }
    pass_result
}

pub enum LoopEvaluationResult {
    Continue,
    Bail,
}

pub async fn evaluate_command_instance(
    construct_did: &ConstructDid,
    pass_result: &mut EvaluationPassResult,
    unexecutable_nodes: &mut HashSet<ConstructDid>,
    genesis_dependency_execution_results: &mut DependencyExecutionResultCache,
    runbook_workspace_context: &RunbookWorkspaceContext,
    runbook_execution_context: &mut RunbookExecutionContext,
    runtime_context: &RuntimeContext,
    supervision_context: &RunbookSupervisionContext,
    action_item_requests: &mut BTreeMap<ConstructDid, Vec<&mut ActionItemRequest>>,
    action_item_responses: &BTreeMap<ConstructDid, Vec<ActionItemResponse>>,
    progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
) -> LoopEvaluationResult {
    let Some(command_instance) = runbook_execution_context.commands_instances.get(&construct_did)
    else {
        // runtime_context.addons.index_command_instance(namespace, package_did, block)
        return LoopEvaluationResult::Continue;
    };

    let add_ctx_to_diag = add_ctx_to_diag(
        "command".to_string(),
        command_instance.specification.matcher.clone(),
        command_instance.name.clone(),
        command_instance.namespace.clone(),
    );

    let package_id = command_instance.package_id.clone();
    let construct_id = &runbook_workspace_context.expect_construct_id(&construct_did);

    let addon_context_key = (package_id.did(), command_instance.namespace.clone());
    let addon_defaults = runbook_workspace_context.get_addon_defaults(&addon_context_key);

    let input_evaluation_results =
        runbook_execution_context.commands_inputs_evaluation_results.get(&construct_did.clone());

    let mut cached_dependency_execution_results = genesis_dependency_execution_results.clone();

    // Retrieve the construct_did of the inputs
    // Collect the outputs
    let references_expressions =
        command_instance.get_expressions_referencing_commands_from_inputs().unwrap();

    for (_input, expr) in references_expressions.into_iter() {
        if let Some((dependency, _, _)) = runbook_workspace_context
            .try_resolve_construct_reference_in_expression(&package_id, &expr)
            .unwrap()
        {
            if let Some(evaluation_result) =
                runbook_execution_context.commands_execution_results.get(&dependency)
            {
                match cached_dependency_execution_results.merge(&dependency, evaluation_result) {
                    Ok(_) => {}
                    Err(_) => continue,
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
        return LoopEvaluationResult::Continue;
    };

    let mut evaluated_inputs = match evaluated_inputs_res {
        Ok(result) => match result {
            CommandInputEvaluationStatus::Complete(result) => result,
            CommandInputEvaluationStatus::NeedsUserInteraction(_) => {
                return LoopEvaluationResult::Continue;
            }
            CommandInputEvaluationStatus::Aborted(_, diags) => {
                pass_result.append_diagnostics(diags, construct_id, &add_ctx_to_diag);
                return LoopEvaluationResult::Bail;
            }
        },
        Err(diags) => {
            pass_result.append_diagnostics(diags, construct_id, &add_ctx_to_diag);
            return LoopEvaluationResult::Bail;
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
                    return LoopEvaluationResult::Continue;
                }
                pass_result.actions.append(&mut new_actions);
                updated_signers
            }
            Err((updated_signers, diag)) => {
                pass_result.push_diagnostic(&diag, construct_id, &add_ctx_to_diag);
                runbook_execution_context.signers_state = Some(updated_signers);
                return LoopEvaluationResult::Bail;
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
                    return LoopEvaluationResult::Continue;
                }
                pass_result.actions.append(&mut new_actions);
            }
            Err(diag) => {
                pass_result.push_diagnostic(&diag, construct_id, &add_ctx_to_diag);
                return LoopEvaluationResult::Bail;
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
            pass_result.push_diagnostic(&diag, construct_id, &add_ctx_to_diag);
            return LoopEvaluationResult::Continue;
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
                pass_result.push_diagnostic(&diag, construct_id, &add_ctx_to_diag);
                return LoopEvaluationResult::Bail;
            }
        };
        if let Some(deps) = runbook_execution_context.commands_dependencies.get(&construct_did) {
            for dep in deps.iter() {
                unexecutable_nodes.insert(dep.clone());
            }
        }
        pass_result.pending_background_tasks_futures.push(future);
        pass_result.pending_background_tasks_constructs_uuids.push(construct_did.clone());
    } else {
        runbook_execution_context
            .commands_execution_results
            .entry(construct_did.clone())
            .or_insert_with(CommandExecutionResult::new)
            .append(&mut execution_result);
    }

    LoopEvaluationResult::Continue
}

pub async fn evaluate_embedded_runbook_instance(
    background_tasks_uuid: &Uuid,
    construct_did: &ConstructDid,
    pass_result: &mut EvaluationPassResult,
    unexecutable_nodes: &mut HashSet<ConstructDid>,
    genesis_dependency_execution_results: &mut DependencyExecutionResultCache,
    runbook_workspace_context: &RunbookWorkspaceContext,
    runbook_execution_context: &mut RunbookExecutionContext,
    runtime_context: &RuntimeContext,
    supervision_context: &RunbookSupervisionContext,
    action_item_requests: &mut BTreeMap<ConstructDid, Vec<&mut ActionItemRequest>>,
    action_item_responses: &BTreeMap<ConstructDid, Vec<ActionItemResponse>>,
    progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
) -> LoopEvaluationResult {
    let Some(embedded_runbook) = runbook_execution_context.embedded_runbooks.get(&construct_did)
    else {
        return LoopEvaluationResult::Continue;
    };

    let add_ctx_to_diag = add_ctx_to_embedded_runbook_diag(embedded_runbook.name.clone());

    let package_id = embedded_runbook.package_id.clone();
    let construct_id = &runbook_workspace_context.expect_construct_id(&construct_did);

    let input_evaluation_results =
        runbook_execution_context.commands_inputs_evaluation_results.get(&construct_did.clone());

    let mut cached_dependency_execution_results = genesis_dependency_execution_results.clone();

    // Retrieve the construct_did of the inputs
    // Collect the outputs
    let references_expressions =
        embedded_runbook.get_expressions_referencing_commands_from_runbook_inputs().unwrap();

    for (_input, expr) in references_expressions.into_iter() {
        if let Some((dependency, _, _)) = runbook_workspace_context
            .try_resolve_construct_reference_in_expression(&package_id, &expr)
            .unwrap()
        {
            if let Some(evaluation_result) =
                runbook_execution_context.commands_execution_results.get(&dependency)
            {
                match cached_dependency_execution_results.merge(&dependency, evaluation_result) {
                    Ok(_) => {}
                    Err(_) => continue,
                }
            }
        }
    }

    let evaluated_inputs_res = perform_inputs_evaluation(
        embedded_runbook,
        &cached_dependency_execution_results,
        &input_evaluation_results,
        &AddonDefaults::new("empty"),
        &action_item_responses.get(&construct_did),
        &package_id,
        runbook_workspace_context,
        runbook_execution_context,
        runtime_context,
        false,
    );
    let evaluated_inputs = match evaluated_inputs_res {
        Ok(result) => match result {
            CommandInputEvaluationStatus::Complete(result) => result,
            CommandInputEvaluationStatus::NeedsUserInteraction(_) => {
                return LoopEvaluationResult::Continue
            }
            CommandInputEvaluationStatus::Aborted(_, diags) => {
                pass_result.append_diagnostics(diags, construct_id, &add_ctx_to_diag);
                return LoopEvaluationResult::Bail;
            }
        },
        Err(diags) => {
            pass_result.append_diagnostics(diags, construct_id, &add_ctx_to_diag);
            return LoopEvaluationResult::Bail;
        }
    };

    // todo: assert that the evaluated inputs are sufficient to execute the embedded runbook (we have all required inputs)

    let mut executable_embedded_runbook = match ExecutableEmbeddedRunbookInstance::new(
        embedded_runbook.clone(),
        EmbeddedRunbookStatefulExecutionContext::new(
            &runbook_execution_context.signers_instances,
            &runbook_execution_context.signers_state,
            &runbook_workspace_context
                .constructs
                .iter()
                .filter_map(|(did, id)| {
                    if runbook_execution_context.signers_instances.contains_key(did) {
                        Some((did.clone(), id.clone()))
                    } else {
                        None
                    }
                })
                .collect(),
        ),
        &evaluated_inputs.inputs,
        runtime_context,
    ) {
        Ok(res) => res,
        Err(diag) => {
            pass_result.push_diagnostic(&diag, construct_id, &add_ctx_to_diag);
            return LoopEvaluationResult::Bail;
        }
    };

    let result = Box::pin(run_constructs_evaluation(
        background_tasks_uuid,
        &executable_embedded_runbook.context.workspace_context,
        &mut executable_embedded_runbook.context.execution_context,
        runtime_context,
        supervision_context,
        action_item_requests,
        action_item_responses,
        progress_tx,
    ))
    .await;

    runbook_execution_context
        .commands_inputs_evaluation_results
        .insert(construct_did.clone(), evaluated_inputs.clone());

    // update the runbook's context with the results of the embedded runbook
    runbook_execution_context.append_command_inputs_evaluation_results_no_override(
        &executable_embedded_runbook.context.execution_context.commands_inputs_evaluation_results,
    );

    runbook_execution_context.signers_state =
        executable_embedded_runbook.context.execution_context.signers_state;

    pass_result.merge(result);

    let has_diags = !pass_result.diagnostics.is_empty();
    let has_pending_actions = pass_result.actions.has_pending_actions();
    let has_pending_background_tasks = !pass_result.pending_background_tasks_futures.is_empty();

    if has_diags || has_pending_actions || has_pending_background_tasks {
        if let Some(deps) = runbook_execution_context.commands_dependencies.get(&construct_did) {
            for dep in deps.iter() {
                unexecutable_nodes.insert(dep.clone());
            }
        }
        return LoopEvaluationResult::Continue;
    } else {
        // loop over all of the results of executing this embedded runbook and merge them into the current runbook's context
        runbook_execution_context.append_commands_execution_results(
            &executable_embedded_runbook.context.execution_context.commands_execution_results,
        );
    }

    LoopEvaluationResult::Continue
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
    dependencies_execution_results: &DependencyExecutionResultCache,
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
            return Err(diagnosed_error!(
                "Directly referencing a variable is not supported. Did you mean `variable.{}`?",
                _decorated_var.as_str()
            )
            .into());
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

            let res = match dependencies_execution_results.get(&dependency) {
                Some(res) => match res.clone() {
                    Ok(res) => res,
                    Err(e) => return Ok(ExpressionEvaluationStatus::CompleteErr(e.clone())),
                },
                None => match runbook_execution_context.commands_execution_results.get(&dependency)
                {
                    Some(res) => res.clone(),
                    None => return Ok(ExpressionEvaluationStatus::DependencyNotComputed),
                },
            };

            let attribute = components.pop_front().unwrap_or("value".into());

            match res.outputs.get(&attribute) {
                Some(output) => {
                    if let Some(_) = output.as_object() {
                        output.get_keys_from_object(components)?
                    } else {
                        output.clone()
                    }
                }
                // this is a bit hacky. in some cases, our outputs are nested in a "value" key, but we don't want the user
                // to have to provide that key. if that's the case, the above line consumed an attribute we want to use and
                // didn't actually use the default "value" key. so if fetching the provided attribute key yields no
                // results, fetch "value", and add our attribute back to the list of components
                None => match res.outputs.get("value") {
                    Some(output) => {
                        if let Some(_) = output.as_object() {
                            components.push_front(attribute);
                            output.get_keys_from_object(components)?
                        } else {
                            output.clone()
                        }
                    }
                    None => return Ok(ExpressionEvaluationStatus::DependencyNotComputed),
                },
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
                    ActionItemResponseType::SendTransaction(response) => {
                        if let Some(mut signer_state) =
                            signers.pop_signer_state(&response.signer_uuid)
                        {
                            let did = &construct_did.to_string();
                            signer_state.insert_scoped_value(
                                &did,
                                TX_HASH,
                                Value::string(response.transaction_hash.clone()),
                            );

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
    with_evaluatable_inputs: &impl WithEvaluatableInputs,
    dependencies_execution_results: &DependencyExecutionResultCache,
    input_evaluation_results: &Option<&CommandInputsEvaluationResult>,
    addon_defaults: &AddonDefaults,
    action_item_response: &Option<&Vec<ActionItemResponse>>,
    package_id: &PackageId,
    runbook_workspace_context: &RunbookWorkspaceContext,
    runbook_execution_context: &RunbookExecutionContext,
    runtime_context: &RuntimeContext,
    simulation: bool,
) -> Result<CommandInputEvaluationStatus, Vec<Diagnostic>> {
    let mut has_existing_evaluation_results = true;
    let mut results = match *input_evaluation_results {
        Some(evaluated_inputs) => {
            let mut inputs = evaluated_inputs.clone();
            inputs.inputs = inputs.inputs.with_defaults(&addon_defaults.store);
            inputs
        }
        None => {
            has_existing_evaluation_results = false;
            CommandInputsEvaluationResult::new(
                &with_evaluatable_inputs.name(),
                &addon_defaults.store,
            )
        }
    };
    let mut require_user_interaction = false;
    let mut diags = vec![];
    let inputs = with_evaluatable_inputs.spec_inputs();
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
        let input_name = input.name();
        let input_typing = input.typing();
        let input_optional = input.optional();

        if simulation {
            // Hard coding "signer" here is a shortcut - to be improved, we should retrieve a pointer instead that is defined on the spec
            if input_name.eq("signer") {
                results.unevaluated_inputs.insert("signer".into(), None);
                continue;
            }
            if input_name.eq("signers") {
                results.unevaluated_inputs.insert("signers".into(), None);
                continue;
            }
        } else if has_existing_evaluation_results {
            if !results.unevaluated_inputs.contains_key(&input_name) {
                continue;
            }
        }
        if let Some(object_props) = input.as_object() {
            // get this object expression to check if it's a traversal. if the expected
            // object type is a traversal, we should parse it as a regular field rather than
            // looking at each property of the object
            let Some(expr) =
                with_evaluatable_inputs.get_expression_from_object(&input_name, &input_typing)?
            else {
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
                        results.unevaluated_inputs.insert(input_name.clone(), Some(e.clone()));
                        diags.push(e);
                        continue;
                    }
                    Err(e) => {
                        if e.is_error() {
                            fatal_error = true;
                        }
                        results.unevaluated_inputs.insert(input_name.clone(), Some(e.clone()));
                        diags.push(e);
                        continue;
                    }
                    Ok(ExpressionEvaluationStatus::DependencyNotComputed) => {
                        require_user_interaction = true;
                        results.unevaluated_inputs.insert(input_name.clone(), None);
                        continue;
                    }
                };
                results.insert(&input_name, value);
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
                        results.unevaluated_inputs.insert(input_name.to_string(), Some(e.clone()));
                        diags.push(e);
                        continue;
                    }
                    Err(e) => {
                        if e.is_error() {
                            fatal_error = true;
                        }
                        results.unevaluated_inputs.insert(input_name.to_string(), Some(e.clone()));
                        diags.push(e);
                        continue;
                    }
                    Ok(ExpressionEvaluationStatus::DependencyNotComputed) => {
                        require_user_interaction = true;
                        results.unevaluated_inputs.insert(input_name.to_string(), None);
                        continue;
                    }
                };
                results.insert(&input_name, value);
                continue;
            }
            let mut object_values = IndexMap::new();
            for prop in object_props.iter() {
                let Some(expr) =
                    with_evaluatable_inputs.get_expression_from_object_property(&input_name, &prop)
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
                        results.unevaluated_inputs.insert(input_name.clone(), Some(e.clone()));
                        diags.push(e);
                        continue;
                    }
                    Err(e) => {
                        if e.is_error() {
                            fatal_error = true;
                        }
                        results.unevaluated_inputs.insert(input_name.clone(), Some(e.clone()));
                        diags.push(e);
                        continue;
                    }
                    Ok(ExpressionEvaluationStatus::DependencyNotComputed) => {
                        require_user_interaction = true;
                        results.unevaluated_inputs.insert(input_name.clone(), None);
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
                results.insert(&input_name, Value::object(object_values));
            }
        } else if let Some(_) = input.as_array() {
            let mut array_values = vec![];
            let Some(expr) = with_evaluatable_inputs.get_expression_from_input(&input_name) else {
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
                    results.unevaluated_inputs.insert(input_name.clone(), Some(e.clone()));
                    diags.push(e);
                    continue;
                }
                Err(e) => {
                    if e.is_error() {
                        fatal_error = true;
                    }
                    results.unevaluated_inputs.insert(input_name.clone(), Some(e.clone()));
                    diags.push(e);
                    continue;
                }
                Ok(ExpressionEvaluationStatus::DependencyNotComputed) => {
                    require_user_interaction = true;
                    results.unevaluated_inputs.insert(input_name.clone(), None);
                    continue;
                }
            };

            results.insert(&input_name, value);
        } else if let Some(_) = input.as_map() {
            match evaluate_map_input(
                results.clone(),
                &input,
                with_evaluatable_inputs,
                dependencies_execution_results,
                package_id,
                runbook_workspace_context,
                runbook_execution_context,
                runtime_context,
            ) {
                Ok(Some(res)) => {
                    if res.fatal_error {
                        fatal_error = true;
                    }
                    if res.require_user_interaction {
                        require_user_interaction = true;
                    }
                    results = res.result;
                    diags.extend(res.diags);
                }
                Ok(None) => continue,
                Err(e) => return Err(e),
            };
        } else {
            let Some(expr) = with_evaluatable_inputs.get_expression_from_input(&input_name) else {
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
                    results.unevaluated_inputs.insert(input_name.clone(), Some(e.clone()));
                    diags.push(e);
                    continue;
                }
                Err(e) => {
                    if e.is_error() {
                        fatal_error = true;
                    }
                    results.unevaluated_inputs.insert(input_name.clone(), Some(e.clone()));
                    diags.push(e);
                    continue;
                }
                Ok(ExpressionEvaluationStatus::DependencyNotComputed) => {
                    require_user_interaction = true;
                    results.unevaluated_inputs.insert(input_name.clone(), None);
                    continue;
                }
            };
            results.insert(&input_name, value);
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
    dependencies_execution_results: &DependencyExecutionResultCache,
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

                let Some(expr) = signer_instance.get_expression_from_object_property(&input, &prop)
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

            let Some(expr) = signer_instance.get_expression_from_input(&input) else {
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
                let Some(expr) = signer_instance.get_expression_from_input(&input) else {
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

#[derive(Clone, Debug)]
struct EvaluateMapInputResult {
    result: CommandInputsEvaluationResult,
    require_user_interaction: bool,
    diags: Vec<Diagnostic>,
    fatal_error: bool,
}
/// Evaluating a map input is different from other inputs because it can contain nested blocks.
/// For other types, once we have an input block and an identifier in the block, we can expect that identifier to point to an "attribute".
/// For a map, it could point to another block, so we need to recursively look inside maps.
fn evaluate_map_input(
    mut result: CommandInputsEvaluationResult,
    input_spec: &impl EvaluatableInput,
    with_evaluatable_inputs: &impl WithEvaluatableInputs,
    dependencies_execution_results: &DependencyExecutionResultCache,
    package_id: &PackageId,
    runbook_workspace_context: &RunbookWorkspaceContext,
    runbook_execution_context: &RunbookExecutionContext,
    runtime_context: &RuntimeContext,
) -> Result<Option<EvaluateMapInputResult>, Vec<Diagnostic>> {
    let input_name = input_spec.name();
    let input_typing = input_spec.typing();
    let input_optional = input_spec.optional();

    let spec_object_props = input_spec.as_map().expect("expected input to be a map");

    let Some(blocks) =
        with_evaluatable_inputs.get_blocks_for_map(&input_name, &input_typing, input_optional)?
    else {
        return Ok(None);
    };
    let res = EvaluateMapObjectPropResult::new();
    match evaluate_map_object_prop(
        &input_name,
        res,
        blocks,
        spec_object_props,
        dependencies_execution_results,
        package_id,
        runbook_workspace_context,
        runbook_execution_context,
        runtime_context,
    ) {
        Ok(Some(res)) => {
            result.insert(&input_name, Value::array(res.entries));
            result.unevaluated_inputs.merge(&res.unevaluated_inputs);
            return Ok(Some(EvaluateMapInputResult {
                result,
                require_user_interaction: res.require_user_interaction,
                diags: res.diags,
                fatal_error: res.fatal_error,
            }));
        }
        Ok(None) => return Ok(None),
        Err(e) => return Err(e),
    };
}

#[derive(Clone, Debug)]
struct EvaluateMapObjectPropResult {
    entries: Vec<Value>,
    unevaluated_inputs: UnevaluatedInputsMap,
    require_user_interaction: bool,
    diags: Vec<Diagnostic>,
    fatal_error: bool,
}
impl EvaluateMapObjectPropResult {
    fn new() -> Self {
        Self {
            entries: vec![],
            unevaluated_inputs: UnevaluatedInputsMap::new(),
            require_user_interaction: false,
            diags: vec![],
            fatal_error: false,
        }
    }
}

fn evaluate_map_object_prop(
    spec_input_name: &str,
    mut parent_result: EvaluateMapObjectPropResult,
    blocks: Vec<HclBlock>,
    spec_object_props: &Vec<ObjectProperty>,
    dependencies_execution_results: &DependencyExecutionResultCache,
    package_id: &PackageId,
    runbook_workspace_context: &RunbookWorkspaceContext,
    runbook_execution_context: &RunbookExecutionContext,
    runtime_context: &RuntimeContext,
) -> Result<Option<EvaluateMapObjectPropResult>, Vec<Diagnostic>> {
    for block in blocks.iter() {
        let mut object_values = IndexMap::new();
        for spec_object_prop in spec_object_props.iter() {
            let value = if let Some(expr) =
                visit_optional_untyped_attribute(&spec_object_prop.name, &block)
            {
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
                            parent_result.fatal_error = true;
                        }
                        parent_result
                            .unevaluated_inputs
                            .insert(spec_input_name.to_string(), Some(e.clone()));
                        parent_result.diags.push(e);
                        continue;
                    }
                    Err(e) => {
                        if e.is_error() {
                            parent_result.fatal_error = true;
                        }
                        parent_result
                            .unevaluated_inputs
                            .insert(spec_input_name.to_string(), Some(e.clone()));
                        parent_result.diags.push(e);
                        continue;
                    }
                    Ok(ExpressionEvaluationStatus::DependencyNotComputed) => {
                        parent_result.require_user_interaction = true;
                        parent_result.unevaluated_inputs.insert(spec_input_name.to_string(), None);
                        continue;
                    }
                };
                value
            } else {
                let child_map_blocks = block
                    .body
                    .get_blocks(&spec_object_prop.name)
                    .into_iter()
                    .map(|b| b.clone())
                    .collect::<Vec<_>>();
                if child_map_blocks.is_empty() {
                    continue;
                }
                let Type::Map(ref child_map_spec_object_props) = spec_object_prop.typing else {
                    return Err(vec![diagnosed_error!(
                        "expected type {} for property {}, found map",
                        spec_object_prop.typing.to_string(),
                        spec_object_prop.name
                    )]);
                };
                match evaluate_map_object_prop(
                    spec_input_name,
                    parent_result.clone(),
                    child_map_blocks,
                    child_map_spec_object_props,
                    dependencies_execution_results,
                    package_id,
                    runbook_workspace_context,
                    runbook_execution_context,
                    runtime_context,
                ) {
                    Ok(Some(res)) => {
                        parent_result.unevaluated_inputs = res.unevaluated_inputs;
                        let mut diags = parent_result.diags.clone();
                        diags.extend(res.diags);
                        parent_result.diags = diags;
                        if res.fatal_error {
                            parent_result.fatal_error = true;
                            continue;
                        }
                        if res.require_user_interaction {
                            parent_result.require_user_interaction = true;
                            continue;
                        }
                        Value::array(res.entries)
                    }
                    Ok(None) => continue,
                    Err(e) => return Err(e),
                }
            };

            match value.clone() {
                Value::Object(obj) => {
                    for (k, v) in obj.into_iter() {
                        object_values.insert(k, v);
                    }
                }
                v => {
                    object_values.insert(spec_object_prop.name.to_string(), v);
                }
            };
        }
        parent_result.entries.push(Value::object(object_values));
    }

    Ok(Some(parent_result))
}
