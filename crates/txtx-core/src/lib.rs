#[macro_use]
extern crate lazy_static;

#[macro_use]
pub extern crate txtx_addon_kit as kit;

mod constants;
pub mod errors;
pub mod eval;
pub mod manifest;
// pub mod snapshot;
pub mod runbook;
pub mod std;
pub mod types;

#[cfg(test)]
mod tests;

use ::std::collections::BTreeMap;
use ::std::future::Future;
use ::std::pin::Pin;
use ::std::thread::sleep;
use ::std::time::Duration;

use crate::runbook::flow_context::FlowContext;
use constants::ACTION_ITEM_ENV;
use constants::ACTION_ITEM_GENESIS;
use constants::ACTION_ITEM_VALIDATE_BLOCK;
use eval::run_constructs_evaluation;
use eval::run_signers_evaluation;
use kit::hcl::Span;
use kit::types::block_id::BlockId;
use kit::types::commands::CommandExecutionResult;
use kit::types::frontend::ActionItemRequestType;
use kit::types::frontend::ActionItemRequestUpdate;
use kit::types::frontend::ActionItemResponse;
use kit::types::frontend::ActionItemResponseType;
use kit::types::frontend::Actions;
use kit::types::frontend::Block;
use kit::types::frontend::BlockEvent;
use kit::types::frontend::ErrorPanelData;
use kit::types::frontend::NormalizedActionItemRequestUpdate;
use kit::types::frontend::Panel;
use kit::types::frontend::PickInputOptionRequest;
use kit::types::frontend::ProgressBarVisibilityUpdate;
use kit::types::frontend::ReviewedInputResponse;
use kit::types::frontend::ValidateBlockData;
use kit::types::types::RunbookSupervisionContext;
use kit::types::ConstructDid;
use kit::uuid::Uuid;
use runbook::get_source_context_for_diagnostic;
use tokio::sync::broadcast::error::TryRecvError;
use txtx_addon_kit::channel::{Receiver, Sender};
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::types::frontend::ActionItemRequest;
use txtx_addon_kit::types::frontend::ActionItemStatus;
use txtx_addon_kit::types::frontend::InputOption;
use types::Runbook;

lazy_static! {
    // create this action so we can reference its `id` property, which is built from the immutable data
     pub static ref SET_ENV_ACTION: ActionItemRequest = ActionItemRequest ::new(
        &None,
        "Select the environment to target",
        None,
        ActionItemStatus::Success(None),
        ActionItemRequestType::PickInputOption(PickInputOptionRequest {
            options: vec![],
            selected: InputOption::default(),
        }),
        ACTION_ITEM_ENV,
      );
}

pub async fn start_unsupervised_runbook_runloop(
    runbook: &mut Runbook,
    progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
) -> Result<(), Vec<Diagnostic>> {
    runbook.supervision_context = RunbookSupervisionContext {
        review_input_default_values: false,
        review_input_values: false,
        is_supervised: false,
    };
    for flow_context in runbook.flow_contexts.iter_mut() {
        if !flow_context.is_enabled() {
            continue;
        }

        let mut action_item_requests = BTreeMap::new();
        let action_item_responses = BTreeMap::new();

        let pass_results = run_signers_evaluation(
            &flow_context.workspace_context,
            &mut flow_context.execution_context,
            &runbook.runtime_context,
            &runbook.supervision_context,
            &mut action_item_requests,
            &action_item_responses,
            &progress_tx,
        )
        .await;

        if pass_results.actions.has_pending_actions() {
            return Err(vec![diagnosed_error!(
                "unsupervised executions should not be generating actions"
            )]);
        }

        if pass_results.has_diagnostics() {
            return Err(pass_results.with_spans_filled(&runbook.sources));
        }

        let mut uuid = Uuid::new_v4();
        let mut background_tasks_futures = vec![];
        let mut background_tasks_contructs_dids = vec![];
        let mut runbook_completed = false;

        loop {
            let mut pass_results = run_constructs_evaluation(
                &uuid,
                &flow_context.workspace_context,
                &mut flow_context.execution_context,
                &mut runbook.runtime_context,
                &runbook.supervision_context,
                &mut action_item_requests,
                &action_item_responses,
                &progress_tx,
            )
            .await;

            if pass_results.has_diagnostics() {
                return Err(pass_results.with_spans_filled(&runbook.sources));
            }

            if !pass_results.pending_background_tasks_constructs_uuids.is_empty() {
                background_tasks_futures.append(&mut pass_results.pending_background_tasks_futures);
                background_tasks_contructs_dids
                    .append(&mut pass_results.pending_background_tasks_constructs_uuids);
            }

            if !pass_results.actions.has_pending_actions()
                && background_tasks_contructs_dids.is_empty()
            {
                runbook_completed = true;
            }

            if background_tasks_futures.is_empty() {
                // sleep(time::Duration::from_secs(3));
            } else {
                process_background_tasks(
                    None,
                    background_tasks_contructs_dids,
                    background_tasks_futures,
                    flow_context,
                )
                .await
                .map_err(|mut diag| {
                    diag.span = get_source_context_for_diagnostic(&diag, &runbook.sources);
                    vec![diag]
                })?;
                background_tasks_futures = vec![];
                background_tasks_contructs_dids = vec![];
            }

            uuid = Uuid::new_v4();
            if runbook_completed {
                break;
            }
        }
    }

    Ok(())
}

pub async fn start_supervised_runbook_runloop(
    runbook: &mut Runbook,
    block_tx: Sender<BlockEvent>,
    mut action_item_responses_rx: tokio::sync::broadcast::Receiver<ActionItemResponse>,
) -> Result<(), Vec<Diagnostic>> {
    // let mut runbook_state = BTreeMap::new();

    let mut runbook_initialized = false;
    runbook.supervision_context = RunbookSupervisionContext {
        review_input_default_values: true,
        review_input_values: true,
        is_supervised: true,
    };

    // Compute number of steps
    // A step is

    // store of action_item_ids and the associated action_item_request
    let mut action_item_requests: BTreeMap<BlockId, ActionItemRequest> = BTreeMap::new();
    // store of construct_dids and its associated action_item_response_types
    let mut action_item_responses = BTreeMap::new();

    let mut background_tasks_futures = vec![];
    let mut background_tasks_contructs_dids = vec![];
    let mut background_tasks_handle_uuid = Uuid::new_v4();
    let mut validated_blocks = 0;
    loop {
        let event_opt = match action_item_responses_rx.try_recv() {
            Ok(action) => Some(action),
            Err(TryRecvError::Empty) | Err(TryRecvError::Lagged(_))=> None,
            Err(TryRecvError::Closed) => return Ok(()),
        };

        if !runbook_initialized {
            runbook_initialized = true;

            let genesis_events = build_genesis_panel(
                runbook,
                &mut action_item_requests,
                &action_item_responses,
                &block_tx.clone(),
            )
            .await?;
            for event in genesis_events {
                let _ = block_tx.send(event).unwrap();
            }
        }

        // Cooldown
        let Some(action_item_response) = event_opt else {
            sleep(Duration::from_millis(50));
            continue;
        };
        let ActionItemResponse { action_item_id, payload } = action_item_response.clone();

        if action_item_id == SET_ENV_ACTION.id {
            if let Err(diags) = reset_runbook_execution(
                runbook,
                &payload,
                &mut action_item_requests,
                &action_item_responses,
                &block_tx.clone(),
            )
            .await
            {
                let _ = block_tx.send(BlockEvent::Error(Block {
                    uuid: Uuid::new_v4(),
                    visible: true,
                    panel: Panel::ErrorPanel(ErrorPanelData::from_diagnostics(&diags)),
                }));
                return Err(diags);
            };
            continue;
        }

        if let Some(action_item) = action_item_requests.get(&action_item_id) {
            let action_item = action_item.clone();
            if let Some(construct_did) = action_item.construct_did {
                if let Some(responses) = action_item_responses.get_mut(&construct_did) {
                    responses.push(action_item_response);
                } else {
                    action_item_responses.insert(construct_did, vec![action_item_response]);
                }
            }
        }

        match &payload {
            ActionItemResponseType::ValidateModal => {}
            ActionItemResponseType::ValidateBlock => {
                // Handle background tasks
                if !background_tasks_futures.is_empty() {
                    // todo: we still need to figure out how to have multiple flows in a supervised execution
                    let flow_context = runbook.flow_contexts.first_mut().unwrap();
                    process_background_tasks(
                        Some(SupervisedBackgroundTaskContext::new(
                            &block_tx,
                            &background_tasks_handle_uuid,
                            &action_item_id,
                        )),
                        background_tasks_contructs_dids,
                        background_tasks_futures,
                        flow_context,
                    )
                    .await
                    .map_err(|mut diag| {
                        diag.span = get_source_context_for_diagnostic(&diag, &runbook.sources);
                        vec![diag]
                    })?;
                    background_tasks_futures = vec![];
                    background_tasks_contructs_dids = vec![];
                }

                background_tasks_handle_uuid = Uuid::new_v4();

                // Retrieve the previous requests sent and update their statuses.
                let mut runbook_completed = false;
                let mut map: BTreeMap<ConstructDid, _> = BTreeMap::new();
                let flow_context = runbook.flow_contexts.first_mut().unwrap();
                let mut pass_results = run_constructs_evaluation(
                    &background_tasks_handle_uuid,
                    &flow_context.workspace_context,
                    &mut flow_context.execution_context,
                    &mut runbook.runtime_context,
                    &runbook.supervision_context,
                    &mut map,
                    &action_item_responses,
                    &block_tx.clone(),
                )
                .await;

                // if there were errors, return them to complete execution
                if let Some(error_event) = pass_results.compile_diagnostics_to_block() {
                    let _ = block_tx.send(BlockEvent::Error(error_event));
                    return Err(pass_results.with_spans_filled(&runbook.sources));
                }

                // if there are no pending actions and no background tasks, the runbook could be completed
                if !pass_results.actions.has_pending_actions()
                    && background_tasks_contructs_dids.is_empty()
                {
                    runbook_completed = true;

                    let flow_context = runbook.flow_contexts.first_mut().unwrap();
                    let grouped_actions_items =
                        flow_context.execution_context.collect_outputs_constructs_results();
                    let mut actions = Actions::new_panel("output review", "");
                    for (key, action_items) in grouped_actions_items.into_iter() {
                        actions.push_group(key.as_str(), action_items);
                    }
                    pass_results.actions.append(&mut actions);
                } else if !pass_results.actions.store.is_empty() {
                    validated_blocks = validated_blocks + 1;
                    pass_results.actions.push_sub_group(
                        None,
                        vec![ActionItemRequest::new(
                            &None,
                            "Validate",
                            None,
                            ActionItemStatus::Todo,
                            ActionItemRequestType::ValidateBlock(ValidateBlockData::new(
                                validated_blocks,
                            )),
                            ACTION_ITEM_VALIDATE_BLOCK,
                        )],
                    );
                }

                if !pass_results.pending_background_tasks_constructs_uuids.is_empty() {
                    background_tasks_futures
                        .append(&mut pass_results.pending_background_tasks_futures);
                    background_tasks_contructs_dids
                        .append(&mut pass_results.pending_background_tasks_constructs_uuids);
                }

                let update = ActionItemRequestUpdate::from_id(&action_item_id)
                    .set_status(ActionItemStatus::Success(None));
                pass_results.actions.push_action_item_update(update);
                for new_request in pass_results.actions.get_new_action_item_requests().into_iter() {
                    action_item_requests.insert(new_request.id.clone(), new_request.clone());
                }
                let block_events =
                    pass_results.actions.compile_actions_to_block_events(&action_item_requests);

                for event in block_events.into_iter() {
                    let _ = block_tx.send(event);
                }
                if runbook_completed {
                    let _ = block_tx.send(BlockEvent::RunbookCompleted);
                    return Ok(());
                }
            }
            ActionItemResponseType::PickInputOption(_) => {}
            ActionItemResponseType::ProvideInput(_) => {}
            ActionItemResponseType::ReviewInput(ReviewedInputResponse {
                value_checked,
                force_execution,
                ..
            }) => {
                let new_status = match value_checked {
                    true => ActionItemStatus::Success(None),
                    false => ActionItemStatus::Todo,
                };
                let _ = block_tx.send(BlockEvent::UpdateActionItems(vec![
                    ActionItemRequestUpdate::from_id(&action_item_id)
                        .set_status(new_status)
                        .normalize(&action_item_requests)
                        .unwrap(),
                ]));
                if *force_execution {
                    let running_context = runbook.flow_contexts.first_mut().unwrap();
                    let mut pass_results = run_constructs_evaluation(
                        &background_tasks_handle_uuid,
                        &running_context.workspace_context,
                        &mut running_context.execution_context,
                        &mut runbook.runtime_context,
                        &runbook.supervision_context,
                        &mut BTreeMap::new(),
                        &action_item_responses,
                        &block_tx.clone(),
                    )
                    .await;
                    let mut updated_actions = vec![];
                    for action in pass_results
                        .actions
                        .compile_actions_to_item_updates(&action_item_requests)
                        .into_iter()
                    {
                        updated_actions.push(action.normalize(&action_item_requests).unwrap())
                    }
                    let _ = block_tx.send(BlockEvent::UpdateActionItems(updated_actions));

                    if !pass_results.pending_background_tasks_constructs_uuids.is_empty() {
                        background_tasks_futures
                            .append(&mut pass_results.pending_background_tasks_futures);
                        background_tasks_contructs_dids
                            .append(&mut pass_results.pending_background_tasks_constructs_uuids);
                    }

                    if pass_results.has_diagnostics() {
                        pass_results.fill_diagnostic_span(&runbook.sources);
                    }
                    if let Some(error_event) = pass_results.compile_diagnostics_to_block() {
                        let _ = block_tx.send(BlockEvent::Error(error_event));
                    }
                }
            }
            ActionItemResponseType::ProvidePublicKey(_response) => {
                // Retrieve the previous requests sent and update their statuses.
                let Some((signer_construct_did, scoped_requests)) =
                    retrieve_related_action_items_requests(
                        &action_item_id,
                        &mut action_item_requests,
                    )
                else {
                    continue;
                };

                let mut map = BTreeMap::new();
                map.insert(signer_construct_did, scoped_requests);

                let running_context = runbook.flow_contexts.first_mut().unwrap();
                let mut pass_result = run_signers_evaluation(
                    &running_context.workspace_context,
                    &mut running_context.execution_context,
                    &mut runbook.runtime_context,
                    &runbook.supervision_context,
                    &mut map,
                    &action_item_responses,
                    &block_tx.clone(),
                )
                .await;

                if pass_result.has_diagnostics() {
                    pass_result.fill_diagnostic_span(&runbook.sources);
                }

                if let Some(error_event) = pass_result.compile_diagnostics_to_block() {
                    let _ = block_tx.send(BlockEvent::Error(error_event));
                } else {
                    let updated_actions = pass_result
                        .actions
                        .compile_actions_to_item_updates(&action_item_requests)
                        .into_iter()
                        .map(|u| u.normalize(&action_item_requests).unwrap())
                        .collect();
                    let _ = block_tx.send(BlockEvent::UpdateActionItems(updated_actions));
                }
            }
            ActionItemResponseType::ProvideSignedTransaction(_)
            | ActionItemResponseType::SendTransaction(_)
            | ActionItemResponseType::ProvideSignedMessage(_) => {
                // Retrieve the previous requests sent and update their statuses.
                let Some((signing_action_construct_did, scoped_requests)) =
                    retrieve_related_action_items_requests(
                        &action_item_id,
                        &mut action_item_requests,
                    )
                else {
                    continue;
                };
                let mut map: BTreeMap<ConstructDid, _> = BTreeMap::new();
                map.insert(signing_action_construct_did, scoped_requests);

                let running_context = runbook.flow_contexts.first_mut().unwrap();
                let mut pass_results = run_constructs_evaluation(
                    &background_tasks_handle_uuid,
                    &running_context.workspace_context,
                    &mut running_context.execution_context,
                    &mut runbook.runtime_context,
                    &runbook.supervision_context,
                    &mut map,
                    &action_item_responses,
                    &block_tx.clone(),
                )
                .await;

                let mut updated_actions = vec![];
                for action in pass_results
                    .actions
                    .compile_actions_to_item_updates(&action_item_requests)
                    .into_iter()
                {
                    updated_actions.push(action.normalize(&action_item_requests).unwrap())
                }
                let _ = block_tx.send(BlockEvent::UpdateActionItems(updated_actions));

                if !pass_results.pending_background_tasks_constructs_uuids.is_empty() {
                    background_tasks_futures
                        .append(&mut pass_results.pending_background_tasks_futures);
                    background_tasks_contructs_dids
                        .append(&mut pass_results.pending_background_tasks_constructs_uuids);
                }
                if pass_results.has_diagnostics() {
                    pass_results.fill_diagnostic_span(&runbook.sources);
                }
                if let Some(error_event) = pass_results.compile_diagnostics_to_block() {
                    let _ = block_tx.send(BlockEvent::Error(error_event));
                }
            }
        };
    }
}

pub fn register_action_items_from_actions(
    actions: &Actions,
    action_item_requests: &mut BTreeMap<BlockId, ActionItemRequest>,
) {
    for action in actions.get_new_action_item_requests().into_iter() {
        action_item_requests.insert(action.id.clone(), action.clone());
    }
}

pub fn retrieve_related_action_items_requests<'a>(
    action_item_id: &BlockId,
    action_item_requests: &'a mut BTreeMap<BlockId, ActionItemRequest>,
) -> Option<(ConstructDid, Vec<&'a mut ActionItemRequest>)> {
    let Some(signer_construct_did) =
        action_item_requests.get(&action_item_id).and_then(|a| a.construct_did.clone())
    else {
        eprintln!("unable to retrieve {}", action_item_id);
        // todo: log error
        return None;
    };
    // // Retrieve the previous requests sent
    // // and update their statuses.
    let mut scoped_requests = vec![];
    for (_, request) in action_item_requests.iter_mut() {
        let Some(ref construct_did) = request.construct_did else {
            continue;
        };
        if construct_did.eq(&signer_construct_did) {
            scoped_requests.push(request);
        }
    }
    Some((signer_construct_did, scoped_requests))
}

pub async fn reset_runbook_execution(
    runbook: &mut Runbook,
    payload: &ActionItemResponseType,
    action_item_requests: &mut BTreeMap<BlockId, ActionItemRequest>,
    action_item_responses: &BTreeMap<ConstructDid, Vec<ActionItemResponse>>,
    progress_tx: &Sender<BlockEvent>,
) -> Result<(), Vec<Diagnostic>> {
    let ActionItemResponseType::PickInputOption(environment_key) = payload else {
        unreachable!(
            "Action item event wih environment uuid sent with invalid payload {:?}",
            payload
        );
    };

    let reset = runbook.update_inputs_selector(Some(environment_key.to_string()), true).await?;

    if !reset {
        unimplemented!()
    }

    let _ = progress_tx.send(BlockEvent::Clear);
    let genesis_events =
        build_genesis_panel(runbook, action_item_requests, action_item_responses, &progress_tx)
            .await?;
    for event in genesis_events {
        let _ = progress_tx.send(event).unwrap();
    }
    Ok(())
}

pub async fn build_genesis_panel(
    runbook: &mut Runbook,
    action_item_requests: &mut BTreeMap<BlockId, ActionItemRequest>,
    action_item_responses: &BTreeMap<ConstructDid, Vec<ActionItemResponse>>,
    progress_tx: &Sender<BlockEvent>,
) -> Result<Vec<BlockEvent>, Vec<Diagnostic>> {
    let mut actions = Actions::new_panel("runbook checklist", "");
    let environments = runbook.get_inputs_selectors();
    let selector = runbook.get_active_inputs_selector();

    if environments.len() > 0 {
        let input_options: Vec<InputOption> = environments
            .iter()
            .map(|k| InputOption { value: k.to_string(), displayed_value: k.to_string() })
            .collect();
        let selected_option: InputOption = selector
            .clone()
            .and_then(|e| Some(InputOption { value: e.clone(), displayed_value: e.clone() }))
            .unwrap_or({
                let k = environments.iter().next().unwrap();
                InputOption { value: k.clone(), displayed_value: k.clone() }
            });
        let action_request = ActionItemRequest::new(
            &None,
            "Select the environment to target",
            None,
            ActionItemStatus::Success(None),
            ActionItemRequestType::PickInputOption(PickInputOptionRequest {
                options: input_options,
                selected: selected_option,
            }),
            ACTION_ITEM_ENV,
        );
        actions.push_sub_group(None, vec![action_request]);
    }

    let Some(running_context) = runbook.flow_contexts.first_mut() else {
        return Err(vec![diagnosed_error!(
            "runbook empty"
        )])
    };
    let mut pass_result: eval::EvaluationPassResult = run_signers_evaluation(
        &running_context.workspace_context,
        &mut running_context.execution_context,
        &runbook.runtime_context,
        &runbook.supervision_context,
        &mut BTreeMap::new(),
        &action_item_responses,
        &progress_tx,
    )
    .await;

    if pass_result.has_diagnostics() {
        pass_result.fill_diagnostic_span(&runbook.sources);

        println!("Diagnostics");
        for diag in pass_result.diagnostics().iter() {
            println!("- {}", diag);
        }
    }

    actions.append(&mut pass_result.actions);

    let validate_action = ActionItemRequest::new(
        &None,
        "start runbook".into(),
        None,
        ActionItemStatus::Todo,
        ActionItemRequestType::ValidateBlock(ValidateBlockData::new(0)),
        ACTION_ITEM_GENESIS,
    );
    actions.push_sub_group(None, vec![validate_action]);

    register_action_items_from_actions(&actions, action_item_requests);

    let panels = actions.compile_actions_to_block_events(&action_item_requests);
    for panel in panels.iter() {
        match panel {
            BlockEvent::Modal(_) => {}
            BlockEvent::Action(_) => {}
            _ => {
                println!("-----");
            }
        }
    }
    // assert_eq!(panels.len(), 1);

    Ok(panels)
}

#[derive(Debug, Clone)]
pub struct SupervisedBackgroundTaskContext {
    block_tx: Sender<BlockEvent>,
    background_tasks_handle_uuid: Uuid,
    action_item_id: BlockId,
}
impl SupervisedBackgroundTaskContext {
    pub fn new(
        block_tx: &Sender<BlockEvent>,
        background_tasks_handle_uuid: &Uuid,
        action_item_id: &BlockId,
    ) -> Self {
        SupervisedBackgroundTaskContext {
            block_tx: block_tx.clone(),
            background_tasks_handle_uuid: background_tasks_handle_uuid.clone(),
            action_item_id: action_item_id.clone(),
        }
    }
}

pub async fn process_background_tasks(
    supervised_context: Option<SupervisedBackgroundTaskContext>,
    background_tasks_contructs_dids: Vec<ConstructDid>,
    background_tasks_futures: Vec<
        Pin<Box<dyn Future<Output = Result<CommandExecutionResult, Diagnostic>> + Send>>,
    >,
    flow_context: &mut FlowContext,
) -> Result<(), Diagnostic> {
    if let Some(SupervisedBackgroundTaskContext {
        block_tx,
        background_tasks_handle_uuid,
        action_item_id,
    }) = supervised_context.as_ref()
    {
        let _ =
            block_tx.send(BlockEvent::UpdateActionItems(vec![NormalizedActionItemRequestUpdate {
                id: action_item_id.clone(),
                action_status: Some(ActionItemStatus::Success(None)),
                action_type: None,
            }]));
        let _ = block_tx.send(BlockEvent::ProgressBar(Block::new(
            &background_tasks_handle_uuid,
            Panel::ProgressBar(vec![]),
        )));
    }

    let results: Vec<Result<CommandExecutionResult, Diagnostic>> =
        kit::futures::future::join_all(background_tasks_futures).await;
    for (construct_did, result) in background_tasks_contructs_dids.into_iter().zip(results) {
        match result {
            Ok(result) => {
                flow_context
                    .execution_context
                    .append_commands_execution_result(&construct_did, &result);
            }
            Err(mut diag) => {
                let construct_id =
                    flow_context.workspace_context.expect_construct_id(&construct_did);
                diag = diag.location(&construct_id.construct_location);
                if let Some(command_instance) =
                    flow_context.execution_context.commands_instances.get_mut(&construct_did)
                {
                    diag = diag.set_span_range(command_instance.block.span());
                };
                if let Some(SupervisedBackgroundTaskContext {
                    block_tx,
                    background_tasks_handle_uuid,
                    ..
                }) = supervised_context.as_ref()
                {
                    let _ = block_tx.send(BlockEvent::Error(Block {
                        uuid: Uuid::new_v4(),
                        visible: true,
                        panel: Panel::ErrorPanel(ErrorPanelData::from_diagnostics(&vec![
                            diag.clone()
                        ])),
                    }));
                    let _ = block_tx.send(BlockEvent::UpdateProgressBarVisibility(
                        ProgressBarVisibilityUpdate::new(&background_tasks_handle_uuid, false),
                    ));
                }
                return Err(diag);
            }
        }
    }

    if let Some(SupervisedBackgroundTaskContext {
        block_tx, background_tasks_handle_uuid, ..
    }) = supervised_context.as_ref()
    {
        let _ = block_tx.send(BlockEvent::UpdateProgressBarVisibility(
            ProgressBarVisibilityUpdate::new(&background_tasks_handle_uuid, false),
        ));
    }
    Ok(())
}
