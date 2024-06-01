#[macro_use]
extern crate lazy_static;

#[macro_use]
pub extern crate txtx_addon_kit as kit;

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
use ::std::time::Duration;

use eval::collect_runbook_outputs;
use eval::run_constructs_evaluation;
use eval::run_wallets_evaluation;
use kit::types::commands::CommandExecutionContext;
use kit::types::frontend::ActionItemRequestType;
use kit::types::frontend::ActionItemResponse;
use kit::types::frontend::ActionItemResponseType;
use kit::types::frontend::BlockEvent;
use kit::types::frontend::Panel;
use kit::types::frontend::ReviewedInputResponse;
use kit::types::frontend::SetActionItemStatus;
use kit::types::wallets::WalletInstance;
use kit::uuid::Uuid;
use txtx_addon_kit::channel::{Receiver, Sender, TryRecvError};
use txtx_addon_kit::hcl::structure::Block as CodeBlock;
use txtx_addon_kit::helpers::fs::FileLocation;
use txtx_addon_kit::types::commands::CommandId;
use txtx_addon_kit::types::commands::CommandInstanceOrParts;
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::types::frontend::ActionGroup;
use txtx_addon_kit::types::frontend::ActionItemRequest;
use txtx_addon_kit::types::frontend::ActionItemStatus;
use txtx_addon_kit::types::frontend::ActionPanelData;
use txtx_addon_kit::types::frontend::ActionSubGroup;
use txtx_addon_kit::types::frontend::Block;
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
    let _ = run_constructs_indexing(runbook, runtime_context)?;
    let _ = run_constructs_checks(runbook, &mut runtime_context.addons_ctx)?;
    Ok(())
}

pub struct AddonsContext {
    addons: HashMap<String, Box<dyn Addon>>,
    contexts: HashMap<(PackageUuid, String), AddonContext>,
}

impl AddonsContext {
    pub fn new() -> Self {
        Self {
            addons: HashMap::new(),
            contexts: HashMap::new(),
        }
    }

    pub fn register(&mut self, addon: Box<dyn Addon>) {
        self.addons.insert(addon.get_namespace().to_string(), addon);
    }

    pub fn consolidate_functions_to_register(&mut self) -> Vec<FunctionSpecification> {
        let mut functions = vec![];
        for (_, addon) in self.addons.iter() {
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
            let Some(addon) = self.addons.get(namespace) else {
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
    pub static ref SET_ENV_UUID: Uuid = Uuid::new_v4();
}

pub async fn start_runbook_runloop(
    runbook: &mut Runbook,
    runtime_context: &mut RuntimeContext,
    block_tx: Sender<BlockEvent>,
    _action_item_request_tx: Sender<ActionItemRequest>,
    action_item_responses_rx: Receiver<ActionItemResponse>,
    environments: BTreeMap<String, BTreeMap<String, String>>,
    interactive_by_default: bool,
) -> Result<(), Vec<Diagnostic>> {
    // let mut runbook_state = BTreeMap::new();

    let mut runbook_initialized = false;

    let execution_context = CommandExecutionContext {
        review_input_default_values: interactive_by_default,
        review_input_values: interactive_by_default,
    };

    // Compute number of steps
    // A step is

    // store of action_item_uuids and the associated action_item_request
    let mut action_item_requests: BTreeMap<Uuid, ActionItemRequest> = BTreeMap::new();
    // store of construct_uuids and its associated action_item_response_types
    let mut action_item_responses = BTreeMap::new();

    loop {
        let event_opt = match action_item_responses_rx.try_recv() {
            Ok(action) => Some(action),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => return Ok(()),
        };

        if !runbook_initialized {
            runbook_initialized = true;
            let _ = run_constructs_dependencies_indexing(runbook, runtime_context)?;
            let genesis_panel = Block::new(
                Uuid::new_v4(),
                Panel::ActionPanel(
                    build_genesis_panel(
                        &environments,
                        &runtime_context.selected_env.clone(),
                        runbook,
                        runtime_context,
                        &execution_context,
                        &mut action_item_requests,
                        &action_item_responses,
                        &block_tx.clone(),
                    )
                    .await?,
                ),
            );
            let _ = block_tx.send(BlockEvent::Append(genesis_panel.clone()));
        }

        // Cooldown
        let Some(ActionItemResponse {
            action_item_uuid,
            payload,
        }) = event_opt
        else {
            sleep(Duration::from_millis(1000));
            continue;
        };

        if action_item_uuid == SET_ENV_UUID.clone() {
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

        if let Some(action_item) = action_item_requests.get(&action_item_uuid) {
            let action_item = action_item.clone();
            if let Some(construct_uuid) = action_item.construct_uuid {
                if let Some(responses) = action_item_responses.get_mut(&construct_uuid) {
                    responses.push(payload.clone());
                } else {
                    action_item_responses.insert(construct_uuid, vec![payload.clone()]);
                }
            }
        }

        match &payload {
            ActionItemResponseType::ValidatePanel => {
                // Retrieve the previous requests sent and update their statuses.
                let mut runbook_completed = false;
                let mut map: BTreeMap<Uuid, _> = BTreeMap::new();

                let mut groups = run_constructs_evaluation(
                    runbook,
                    runtime_context,
                    None,
                    &execution_context,
                    &mut map,
                    &action_item_responses,
                    &block_tx.clone(),
                )
                .await?;

                let block_uuid = Uuid::new_v4();
                if groups.is_empty() {
                    runbook_completed = true;
                    groups = collect_runbook_outputs(&block_uuid, &runbook, &runtime_context);
                }
                let mut sub_groups = vec![];
                let Some((group, action_items)) = groups.pop_first() else {
                    continue;
                };

                for request in action_items.iter() {
                    action_item_requests.insert(request.uuid.clone(), request.clone());
                }

                let action_status = if action_items.is_empty() {
                    ActionItemStatus::Success(None)
                } else {
                    ActionItemStatus::Todo
                };

                sub_groups.push(ActionSubGroup {
                    action_items,
                    allow_batch_completion: true,
                });

                if !runbook_completed {
                    sub_groups.push(ActionSubGroup {
                        action_items: vec![ActionItemRequest {
                            uuid: Uuid::new_v4(),
                            construct_uuid: None,
                            index: 0,
                            title: "Validate".into(),
                            description: "".into(),
                            action_status,
                            action_type: ActionItemRequestType::ValidatePanel,
                        }],
                        allow_batch_completion: false,
                    });
                }

                let panel = Block::new(
                    block_uuid,
                    Panel::ActionPanel(ActionPanelData {
                        title: group,
                        description: "".to_string(),
                        groups: vec![ActionGroup {
                            title: "".into(),
                            sub_groups,
                        }],
                    }),
                );

                let _ = block_tx.send(BlockEvent::Append(panel));
            }
            ActionItemResponseType::PickInputOption(_response) => {
                // collected_responses.insert(k, v)
            }
            ActionItemResponseType::ProvideInput(_) => {}
            ActionItemResponseType::ReviewInput(ReviewedInputResponse {
                value_checked, ..
            }) => {
                let _ = block_tx.send(BlockEvent::UpdateActionItems(vec![SetActionItemStatus {
                    action_item_uuid,
                    new_status: match value_checked {
                        true => ActionItemStatus::Success(None),
                        false => ActionItemStatus::Todo,
                    },
                }]));
            }
            ActionItemResponseType::ProvidePublicKey(_response) => {
                // Retrieve the previous requests sent and update their statuses.
                let Some((wallet_construct_uuid, scoped_requests)) =
                    retrieve_related_action_items_requests(
                        &action_item_uuid,
                        &mut action_item_requests,
                    )
                else {
                    continue;
                };
                let mut map: BTreeMap<Uuid, _> = BTreeMap::new();
                map.insert(wallet_construct_uuid, scoped_requests);
                let _ = run_wallets_evaluation(
                    runbook,
                    runtime_context,
                    &execution_context,
                    &mut map,
                    &action_item_responses,
                    &block_tx.clone(),
                )
                .await?;
                let scoped_updated_requests = map.get(&wallet_construct_uuid).unwrap();
                let updated_actions = scoped_updated_requests
                    .iter()
                    .map(|action| SetActionItemStatus {
                        action_item_uuid: action.uuid.clone(),
                        new_status: action.action_status.clone(),
                    })
                    .collect::<Vec<_>>();
                let _ = block_tx.send(BlockEvent::UpdateActionItems(updated_actions));
            }
            ActionItemResponseType::ProvideSignedTransaction(_response) => {
                // Retrieve the previous requests sent and update their statuses.
                let Some((signing_action_construct_uuid, scoped_requests)) =
                    retrieve_related_action_items_requests(
                        &action_item_uuid,
                        &mut action_item_requests,
                    )
                else {
                    continue;
                };
                let mut map: BTreeMap<Uuid, _> = BTreeMap::new();
                map.insert(signing_action_construct_uuid, scoped_requests);

                let _ = run_constructs_evaluation(
                    runbook,
                    runtime_context,
                    None,
                    &execution_context,
                    &mut map,
                    &action_item_responses,
                    &block_tx.clone(),
                )
                .await?;
                let scoped_updated_requests = map.get(&signing_action_construct_uuid).unwrap();
                let updated_actions = scoped_updated_requests
                    .iter()
                    .map(|action| SetActionItemStatus {
                        action_item_uuid: action.uuid.clone(),
                        new_status: action.action_status.clone(),
                    })
                    .collect::<Vec<_>>();
                let _ = block_tx.send(BlockEvent::UpdateActionItems(updated_actions));
            }
        };
    }
}

pub fn retrieve_related_action_items_requests<'a>(
    action_item_uuid: &Uuid,
    action_item_requests: &'a mut BTreeMap<Uuid, ActionItemRequest>,
) -> Option<(Uuid, Vec<&'a mut ActionItemRequest>)> {
    let Some(wallet_construct_uuid) = action_item_requests
        .get(&action_item_uuid)
        .and_then(|a| a.construct_uuid)
    else {
        eprintln!("unable to retrieve {}", action_item_uuid);
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
    action_item_requests: &mut BTreeMap<Uuid, ActionItemRequest>,
    action_item_responses: &BTreeMap<Uuid, Vec<ActionItemResponseType>>,
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

    runtime_context.set_active_environment(environment_key.into());

    let _ = run_constructs_dependencies_indexing(runbook, runtime_context)?;

    let genesis_panel = Block::new(
        Uuid::new_v4(),
        Panel::ActionPanel(
            build_genesis_panel(
                &environments,
                &Some(environment_key.clone()),
                runbook,
                runtime_context,
                &execution_context,
                action_item_requests,
                action_item_responses,
                &progress_tx,
            )
            .await?,
        ),
    );
    let _ = block_tx.send(BlockEvent::Append(genesis_panel.clone()));
    Ok(())
}

pub async fn build_genesis_panel(
    environments: &BTreeMap<String, BTreeMap<String, String>>,
    selected_env: &Option<String>,
    runbook: &mut Runbook,
    runtime_context: &mut RuntimeContext,
    execution_context: &CommandExecutionContext,
    action_item_requests: &mut BTreeMap<Uuid, ActionItemRequest>,
    action_item_responses: &BTreeMap<Uuid, Vec<ActionItemResponseType>>,
    progress_tx: &Sender<BlockEvent>,
) -> Result<ActionPanelData, Vec<Diagnostic>> {
    let input_options: Vec<InputOption> = environments
        .keys()
        .map(|k| InputOption {
            value: k.to_string(),
            displayed_value: k.to_string(),
        })
        .collect();

    let mut action_items = vec![];

    if environments.len() > 0 {
        let action_request = ActionItemRequest {
            uuid: SET_ENV_UUID.clone(),
            construct_uuid: None,
            index: 0,
            title: "select environment".into(),
            description: selected_env.clone().unwrap_or("".to_string()),
            action_status: ActionItemStatus::Todo,
            action_type: ActionItemRequestType::PickInputOption(input_options),
        };
        action_items.push(action_request.clone());
        action_item_requests.insert(SET_ENV_UUID.clone(), action_request);
    }

    let mut groups = if action_items.is_empty() {
        vec![]
    } else {
        vec![ActionGroup {
            title: "".into(),
            sub_groups: vec![ActionSubGroup {
                action_items: action_items.clone(),
                allow_batch_completion: true,
            }],
        }]
    };

    let mut empty_map = BTreeMap::new();

    let wallet_groups = run_wallets_evaluation(
        runbook,
        runtime_context,
        &execution_context,
        &mut empty_map,
        &action_item_responses,
        &progress_tx,
    )
    .await?;

    let mut actions_groups = vec![];
    for (group_title, sub_groups) in wallet_groups.into_iter() {
        for sub_group in sub_groups.iter() {
            for action in sub_group.action_items.iter() {
                action_item_requests.insert(action.uuid.clone(), action.clone());
            }
        }
        actions_groups.push(ActionGroup {
            title: group_title.clone(),
            sub_groups,
        });
    }
    groups.extend(actions_groups);

    let validate_action = ActionItemRequest {
        uuid: Uuid::new_v4(),
        construct_uuid: None,
        index: 0,
        title: "start runbook".into(),
        description: "".into(),
        action_status: if action_items.is_empty() {
            ActionItemStatus::Success(None)
        } else {
            ActionItemStatus::Todo
        },
        action_type: ActionItemRequestType::ValidatePanel,
    };

    action_item_requests.insert(validate_action.uuid.clone(), validate_action.clone());
    groups.push(ActionGroup {
        title: "".into(),
        sub_groups: vec![ActionSubGroup {
            action_items: vec![validate_action],
            allow_batch_completion: false,
        }],
    });
    Ok(ActionPanelData {
        title: "runbook checklist".into(),
        description: "".to_string(),
        groups,
    })
}
