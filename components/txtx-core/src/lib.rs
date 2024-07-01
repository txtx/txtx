#[macro_use]
extern crate lazy_static;

#[macro_use]
pub extern crate txtx_addon_kit as kit;

#[macro_use]
extern crate serde_derive;

mod constants;
pub mod errors;
pub mod eval;
pub mod std;
pub mod types;
pub mod visitor;

#[cfg(test)]
mod tests;

use ::std::collections::BTreeMap;
use ::std::collections::HashMap;
use ::std::thread::sleep;
use ::std::time;
use ::std::time::Duration;

use constants::ACTION_ITEM_ENV;
use constants::ACTION_ITEM_GENESIS;
use eval::collect_runbook_outputs;
use eval::run_constructs_evaluation;
use eval::run_wallets_evaluation;
use kit::channel;
use kit::types::block_id::BlockId;
use kit::types::commands::CommandExecutionContext;
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
use kit::types::wallets::WalletInstance;
use kit::uuid::Uuid;
use txtx_addon_kit::channel::{Receiver, Sender, TryRecvError};
use txtx_addon_kit::hcl::structure::Block as CodeBlock;
use txtx_addon_kit::helpers::fs::FileLocation;
use txtx_addon_kit::types::commands::CommandId;
use txtx_addon_kit::types::commands::CommandInstanceOrParts;
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::types::frontend::ActionItemRequest;
use txtx_addon_kit::types::frontend::ActionItemStatus;
use txtx_addon_kit::types::frontend::InputOption;
use txtx_addon_kit::types::functions::FunctionSpecification;
use txtx_addon_kit::types::PackageUuid;
use txtx_addon_kit::AddonContext;
use types::RuntimeContext;
use visitor::run_constructs_dependencies_indexing;

use txtx_addon_kit::Addon;
use types::Runbook;
use visitor::run_constructs_checks;
use visitor::run_constructs_indexing;

pub fn pre_compute_runbook(
    runbook: &mut Runbook,
    runtime_context: &mut RuntimeContext,
) -> Result<(), Vec<Diagnostic>> {
    let res = run_constructs_indexing(runbook, runtime_context)?;
    if res {
        return Err(runbook.errors.clone());
    }
    let _ = run_constructs_checks(runbook, &mut runtime_context.addons_ctx)?;
    Ok(())
}

#[derive(Debug)]
pub struct AddonsContext {
    addons: HashMap<String, (Box<dyn Addon>, bool)>,
    contexts: HashMap<(PackageUuid, String), AddonContext>,
}

impl AddonsContext {
    pub fn new() -> Self {
        Self {
            addons: HashMap::new(),
            contexts: HashMap::new(),
        }
    }

    pub fn register(&mut self, addon: Box<dyn Addon>, scope: bool) {
        self.addons
            .insert(addon.get_namespace().to_string(), (addon, scope));
    }

    pub fn consolidate_functions_to_register(&mut self) -> Vec<FunctionSpecification> {
        let mut functions = vec![];
        for (_, (addon, _)) in self.addons.iter() {
            let mut addon_functions = addon.get_functions();
            functions.append(&mut addon_functions);
        }
        functions
    }

    fn find_or_create_context(
        &mut self,
        namespace: &str,
        package_uuid: &PackageUuid,
    ) -> Result<&AddonContext, Diagnostic> {
        let key = (package_uuid.clone(), namespace.to_string());
        if self.contexts.get(&key).is_none() {
            let Some((addon, _)) = self.addons.get(namespace) else {
                unimplemented!();
            };
            let ctx = addon.create_context();
            self.contexts.insert(key.clone(), ctx);
        }
        return Ok(self.contexts.get(&key).unwrap());
    }

    pub fn create_action_instance(
        &mut self,
        namespace: &str,
        command_id: &str,
        command_name: &str,
        package_uuid: &PackageUuid,
        block: &CodeBlock,
        _location: &FileLocation,
    ) -> Result<CommandInstanceOrParts, Diagnostic> {
        let ctx = self.find_or_create_context(namespace, package_uuid)?;
        let command_id = CommandId::Action(command_id.to_string());
        ctx.create_command_instance(&command_id, namespace, command_name, block, package_uuid)
    }

    pub fn create_wallet_instance(
        &mut self,
        namespaced_action: &str,
        wallet_name: &str,
        package_uuid: &PackageUuid,
        block: &CodeBlock,
        _location: &FileLocation,
    ) -> Result<WalletInstance, Diagnostic> {
        let Some((namespace, wallet_id)) = namespaced_action.split_once("::") else {
            todo!("return diagnostic")
        };
        let ctx = self.find_or_create_context(namespace, package_uuid)?;
        ctx.create_wallet_instance(wallet_id, namespace, wallet_name, block, package_uuid)
    }
}

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

pub async fn start_runbook_runloop(
    runbook: &mut Runbook,
    runtime_context: &mut RuntimeContext,
    progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
) -> Result<(), Vec<Diagnostic>> {
    let execution_context = CommandExecutionContext {
        review_input_default_values: false,
        review_input_values: false,
    };

    let mut action_item_requests = BTreeMap::new();
    let action_item_responses = BTreeMap::new();
    let _ = run_constructs_dependencies_indexing(runbook, runtime_context)?;

    let pass_result = run_wallets_evaluation(
        runbook,
        runtime_context,
        &execution_context,
        &mut action_item_requests,
        &action_item_responses,
        &progress_tx,
    )
    .await;

    if pass_result.actions.has_pending_actions() {
        return Err(vec![diagnosed_error!(
            "error: non interactive executions should not be generating actions"
        )]);
    }

    if !pass_result.diagnostics.is_empty() {
        println!("Diagnostics");
        for diag in pass_result.diagnostics.iter() {
            println!("- {}", diag);
        }
    }

    let mut uuid = Uuid::new_v4();
    let mut background_tasks_futures = vec![];
    let mut background_tasks_contructs_uuids = vec![];
    let mut runbook_completed = false;

    loop {
        let mut pass_results = run_constructs_evaluation(
            &uuid,
            runbook,
            runtime_context,
            None,
            &execution_context,
            &mut action_item_requests,
            &action_item_responses,
            &progress_tx,
        )
        .await;

        if !pass_results.diagnostics.is_empty() {
            println!("Diagnostics");
            for diag in pass_results.diagnostics.iter() {
                println!("- {}", diag);
            }
        }

        if !pass_results.actions.has_pending_actions()
            && background_tasks_contructs_uuids.is_empty()
        {
            let grouped_actions_items = collect_runbook_outputs(&uuid, &runbook, &runtime_context);
            for (group, items) in grouped_actions_items.iter() {
                println!("{}", group);
                for item in items.iter() {
                    if let ActionItemRequestType::DisplayOutput(ref output) = item.action_type {
                        println!("- {}: {}", output.name, output.value.to_string());
                    }
                }
            }
            runbook_completed = true;
        }

        if !pass_results
            .pending_background_tasks_constructs_uuids
            .is_empty()
        {
            background_tasks_futures.append(&mut pass_results.pending_background_tasks_futures);
            background_tasks_contructs_uuids
                .append(&mut pass_results.pending_background_tasks_constructs_uuids);
        }

        if background_tasks_futures.is_empty() {
            sleep(time::Duration::from_secs(3));
        } else {
            let results = kit::futures::future::join_all(background_tasks_futures).await;
            for (construct_uuid, result) in
                background_tasks_contructs_uuids.into_iter().zip(results)
            {
                match result {
                    Ok(result) => {
                        runbook
                            .constructs_execution_results
                            .insert(construct_uuid, result);
                    }
                    Err(diag) => {
                        let diags = vec![diag];
                        return Err(diags);
                    }
                }
            }
            background_tasks_futures = vec![];
            background_tasks_contructs_uuids = vec![];
        }

        uuid = Uuid::new_v4();
        if runbook_completed {
            break;
        }
    }
    println!("Terminating runbook");
    Ok(())
}

pub async fn start_interactive_runbook_runloop(
    runbook: &mut Runbook,
    runtime_context: &mut RuntimeContext,
    block_tx: Sender<BlockEvent>,
    _action_item_request_tx: Sender<ActionItemRequest>,
    action_item_responses_rx: Receiver<ActionItemResponse>,
    environments: BTreeMap<String, BTreeMap<String, String>>,
) -> Result<(), Vec<Diagnostic>> {
    // let mut runbook_state = BTreeMap::new();

    let mut runbook_initialized = false;
    let execution_context = CommandExecutionContext {
        review_input_default_values: true,
        review_input_values: true,
    };

    // Compute number of steps
    // A step is

    // store of action_item_ids and the associated action_item_request
    let mut action_item_requests: BTreeMap<BlockId, ActionItemRequest> = BTreeMap::new();
    // store of construct_uuids and its associated action_item_response_types
    let mut action_item_responses = BTreeMap::new();

    let mut background_tasks_futures = vec![];
    let mut background_tasks_contructs_uuids = vec![];
    let mut background_tasks_handle_uuid = Uuid::new_v4();
    let mut validated_blocks = 0;
    loop {
        let event_opt = match action_item_responses_rx.try_recv() {
            Ok(action) => Some(action),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => return Ok(()),
        };

        if !runbook_initialized {
            runbook_initialized = true;
            let _ = run_constructs_dependencies_indexing(runbook, runtime_context)?;
            let genesis_events = build_genesis_panel(
                &environments,
                &runtime_context.selected_env.clone(),
                runbook,
                runtime_context,
                &execution_context,
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
        println!("=> runloop received action item response");
        let ActionItemResponse {
            action_item_id,
            payload,
        } = action_item_response.clone();

        if action_item_id == SET_ENV_ACTION.id {
            reset_runbook_execution(
                &payload,
                runbook,
                runtime_context,
                &block_tx,
                &environments,
                &execution_context,
                &mut action_item_requests,
                &action_item_responses,
                &block_tx.clone(),
            )
            .await?;
            continue;
        }

        if let Some(action_item) = action_item_requests.get(&action_item_id) {
            let action_item = action_item.clone();
            if let Some(construct_uuid) = action_item.construct_uuid {
                if let Some(responses) = action_item_responses.get_mut(&construct_uuid) {
                    responses.push(action_item_response);
                } else {
                    action_item_responses.insert(construct_uuid, vec![action_item_response]);
                }
            }
        }

        match &payload {
            ActionItemResponseType::ValidateModal => {}
            ActionItemResponseType::ValidateBlock => {
                // Handle background tasks
                if !background_tasks_futures.is_empty() {
                    let _ = block_tx.send(BlockEvent::UpdateActionItems(vec![
                        NormalizedActionItemRequestUpdate {
                            id: action_item_id.clone(),
                            action_status: Some(ActionItemStatus::Success(None)),
                            action_type: None,
                        },
                    ]));
                    let _ = block_tx.send(BlockEvent::ProgressBar(Block::new(
                        &background_tasks_handle_uuid,
                        Panel::ProgressBar(vec![]),
                    )));

                    let results = kit::futures::future::join_all(background_tasks_futures).await;
                    for (construct_uuid, result) in
                        background_tasks_contructs_uuids.into_iter().zip(results)
                    {
                        match result {
                            Ok(result) => {
                                runbook
                                    .constructs_execution_results
                                    .insert(construct_uuid, result);
                            }
                            Err(diag) => {
                                let diags = vec![diag];
                                let _ = block_tx.send(BlockEvent::Error(Block {
                                    uuid: Uuid::new_v4(),
                                    visible: true,
                                    panel: Panel::ErrorPanel(ErrorPanelData::from_diagnostics(
                                        &diags,
                                    )),
                                }));
                            }
                        }
                    }

                    println!(
                        "setting visibility for progress bar: {}",
                        &background_tasks_handle_uuid.to_string()
                    );
                    let _ = block_tx.send(BlockEvent::UpdateProgressBarVisibility(
                        ProgressBarVisibilityUpdate::new(&background_tasks_handle_uuid, false),
                    ));
                    background_tasks_futures = vec![];
                    background_tasks_contructs_uuids = vec![];
                }

                background_tasks_handle_uuid = Uuid::new_v4();

                // Retrieve the previous requests sent and update their statuses.
                let mut runbook_completed = false;
                let mut map: BTreeMap<Uuid, _> = BTreeMap::new();

                let mut pass_results = run_constructs_evaluation(
                    &background_tasks_handle_uuid,
                    runbook,
                    runtime_context,
                    None,
                    &execution_context,
                    &mut map,
                    &action_item_responses,
                    &block_tx.clone(),
                )
                .await;

                let block_uuid = Uuid::new_v4();
                if let Some(error_event) = pass_results.compile_diagnostics_to_block() {
                    let _ = block_tx.send(BlockEvent::Error(error_event));
                    return Err(pass_results.diagnostics);
                } else if !pass_results.actions.has_pending_actions()
                    && background_tasks_contructs_uuids.is_empty()
                {
                    runbook_completed = true;
                    let grouped_actions_items =
                        collect_runbook_outputs(&block_uuid, &runbook, &runtime_context);
                    let mut actions = Actions::new_panel("output review", "");
                    for (key, action_items) in grouped_actions_items.into_iter() {
                        actions.push_group(key.as_str(), action_items);
                    }
                    pass_results.actions.append(&mut actions);
                } else if !pass_results.actions.store.is_empty() {
                    validated_blocks = validated_blocks + 1;
                    pass_results
                        .actions
                        .push_sub_group(vec![ActionItemRequest::new(
                            &None,
                            "Validate",
                            None,
                            ActionItemStatus::Todo,
                            ActionItemRequestType::ValidateBlock(ValidateBlockData::new(
                                validated_blocks,
                            )),
                            "validate_block",
                        )]);
                }

                if !pass_results
                    .pending_background_tasks_constructs_uuids
                    .is_empty()
                {
                    background_tasks_futures
                        .append(&mut pass_results.pending_background_tasks_futures);
                    background_tasks_contructs_uuids
                        .append(&mut pass_results.pending_background_tasks_constructs_uuids);
                }

                let update = ActionItemRequestUpdate::from_id(&action_item_id)
                    .set_status(ActionItemStatus::Success(None));
                pass_results.actions.push_action_item_update(update);
                for new_request in pass_results
                    .actions
                    .get_new_action_item_requests()
                    .into_iter()
                {
                    action_item_requests.insert(new_request.id.clone(), new_request.clone());
                }
                let block_events = pass_results
                    .actions
                    .compile_actions_to_block_events(&action_item_requests);

                for event in block_events.into_iter() {
                    let _ = block_tx.send(event);
                }
                if runbook_completed {
                    let _ = block_tx.send(BlockEvent::RunbookCompleted);
                }
            }
            ActionItemResponseType::PickInputOption(_response) => {
                // collected_responses.insert(k, v)
            }
            ActionItemResponseType::ProvideInput(_) => {
                let Some((provide_input_action_construct_uuid, scoped_requests)) =
                    retrieve_related_action_items_requests(
                        &action_item_id,
                        &mut action_item_requests,
                    )
                else {
                    continue;
                };
                let mut map: BTreeMap<Uuid, _> = BTreeMap::new();
                map.insert(provide_input_action_construct_uuid, scoped_requests);

                // todo: as of now, there won't actually be actions returned here from a pick input option response.
                // we need to return actions in this loop when the user provides inputs
                let mut pass_results = run_constructs_evaluation(
                    &background_tasks_handle_uuid,
                    runbook,
                    runtime_context,
                    None,
                    &execution_context,
                    &mut map,
                    &action_item_responses,
                    &block_tx.clone(),
                )
                .await;

                if let Some(error_event) = pass_results.compile_diagnostics_to_block() {
                    let _ = block_tx.send(BlockEvent::Error(error_event));
                } else {
                    let mut updated_actions = vec![];
                    for action in pass_results
                        .actions
                        .compile_actions_to_item_updates(&action_item_requests)
                        .into_iter()
                    {
                        if let Some(update) = action.normalize(&action_item_requests) {
                            updated_actions.push(update);
                        }
                    }
                    let _ = block_tx.send(BlockEvent::UpdateActionItems(updated_actions));

                    if !pass_results
                        .pending_background_tasks_constructs_uuids
                        .is_empty()
                    {
                        background_tasks_futures
                            .append(&mut pass_results.pending_background_tasks_futures);
                        background_tasks_contructs_uuids
                            .append(&mut pass_results.pending_background_tasks_constructs_uuids);
                    }
                }
            }
            ActionItemResponseType::ReviewInput(ReviewedInputResponse {
                value_checked, ..
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
            }
            ActionItemResponseType::ProvidePublicKey(_response) => {
                // Retrieve the previous requests sent and update their statuses.
                let Some((wallet_construct_uuid, scoped_requests)) =
                    retrieve_related_action_items_requests(
                        &action_item_id,
                        &mut action_item_requests,
                    )
                else {
                    continue;
                };

                let mut map = BTreeMap::new();
                map.insert(wallet_construct_uuid, scoped_requests);

                let pass_result = run_wallets_evaluation(
                    runbook,
                    runtime_context,
                    &execution_context,
                    &mut map,
                    &action_item_responses,
                    &block_tx.clone(),
                )
                .await;

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
            | ActionItemResponseType::ProvideSignedMessage(_) => {
                // Retrieve the previous requests sent and update their statuses.
                let Some((signing_action_construct_uuid, scoped_requests)) =
                    retrieve_related_action_items_requests(
                        &action_item_id,
                        &mut action_item_requests,
                    )
                else {
                    continue;
                };
                let mut map: BTreeMap<Uuid, _> = BTreeMap::new();
                map.insert(signing_action_construct_uuid, scoped_requests);

                let mut pass_results = run_constructs_evaluation(
                    &background_tasks_handle_uuid,
                    runbook,
                    runtime_context,
                    None,
                    &execution_context,
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

                if !pass_results
                    .pending_background_tasks_constructs_uuids
                    .is_empty()
                {
                    background_tasks_futures
                        .append(&mut pass_results.pending_background_tasks_futures);
                    background_tasks_contructs_uuids
                        .append(&mut pass_results.pending_background_tasks_constructs_uuids);
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
) -> Option<(Uuid, Vec<&'a mut ActionItemRequest>)> {
    let Some(wallet_construct_uuid) = action_item_requests
        .get(&action_item_id)
        .and_then(|a| a.construct_uuid)
    else {
        eprintln!("unable to retrieve {}", action_item_id);
        // todo: log error
        return None;
    };
    // // Retrieve the previous requests sent
    // // and update their statuses.
    let mut scoped_requests = vec![];
    for (_, request) in action_item_requests.iter_mut() {
        let Some(ref construct_uuid) = request.construct_uuid else {
            continue;
        };
        if construct_uuid.eq(&wallet_construct_uuid) {
            scoped_requests.push(request);
        }
    }
    Some((wallet_construct_uuid, scoped_requests))
}

pub async fn reset_runbook_execution(
    payload: &ActionItemResponseType,
    runbook: &mut Runbook,
    runtime_context: &mut RuntimeContext,
    block_tx: &Sender<BlockEvent>,
    environments: &BTreeMap<String, BTreeMap<String, String>>,
    execution_context: &CommandExecutionContext,
    action_item_requests: &mut BTreeMap<BlockId, ActionItemRequest>,
    action_item_responses: &BTreeMap<Uuid, Vec<ActionItemResponse>>,
    progress_tx: &Sender<BlockEvent>,
) -> Result<(), Vec<Diagnostic>> {
    let ActionItemResponseType::PickInputOption(environment_key) = payload else {
        unreachable!(
            "Action item event wih environment uuid sent with invalid payload {:?}",
            payload
        );
    };

    if environments.get(environment_key.as_str()).is_none() {
        unreachable!("Invalid environment variable was sent",);
    };

    let _ = block_tx.send(BlockEvent::Clear);

    runtime_context
        .set_active_environment(environment_key.into())
        .unwrap();

    let _ = run_constructs_dependencies_indexing(runbook, runtime_context)?;

    let genesis_events = build_genesis_panel(
        &environments,
        &Some(environment_key.clone()),
        runbook,
        runtime_context,
        &execution_context,
        action_item_requests,
        action_item_responses,
        &progress_tx,
    )
    .await?;
    for event in genesis_events {
        let _ = block_tx.send(event).unwrap();
    }
    Ok(())
}

pub async fn build_genesis_panel(
    environments: &BTreeMap<String, BTreeMap<String, String>>,
    selected_env: &Option<String>,
    runbook: &mut Runbook,
    runtime_context: &mut RuntimeContext,
    execution_context: &CommandExecutionContext,
    action_item_requests: &mut BTreeMap<BlockId, ActionItemRequest>,
    action_item_responses: &BTreeMap<Uuid, Vec<ActionItemResponse>>,
    progress_tx: &Sender<BlockEvent>,
) -> Result<Vec<BlockEvent>, Vec<Diagnostic>> {
    let mut actions = Actions::new_panel("runbook checklist", "");

    if environments.len() > 0 {
        let input_options: Vec<InputOption> = environments
            .keys()
            .map(|k| InputOption {
                value: k.to_string(),
                displayed_value: k.to_string(),
            })
            .collect();
        let selected_option: InputOption = selected_env
            .clone()
            .and_then(|e| {
                Some(InputOption {
                    value: e.clone(),
                    displayed_value: e.clone(),
                })
            })
            .unwrap_or({
                let k = environments.keys().next().unwrap();
                InputOption {
                    value: k.clone(),
                    displayed_value: k.clone(),
                }
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
        actions.push_sub_group(vec![action_request]);
    }

    let mut pass_result = run_wallets_evaluation(
        runbook,
        runtime_context,
        &execution_context,
        &mut BTreeMap::new(),
        &action_item_responses,
        &progress_tx,
    )
    .await;

    if !pass_result.diagnostics.is_empty() {
        println!("Diagnostics");
        for diag in pass_result.diagnostics.iter() {
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
    actions.push_sub_group(vec![validate_action]);

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
