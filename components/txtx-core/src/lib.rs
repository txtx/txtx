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
use kit::types::commands::CommandExecutionContext;
use kit::types::frontend::ActionItemRequestType;
use kit::types::frontend::ActionItemResponse;
use kit::types::frontend::ActionItemResponseType;
use kit::types::frontend::BlockEvent;
use kit::types::frontend::Panel;
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
    let _ = run_constructs_dependencies_indexing(runbook, runtime_context)?;
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

    pub fn create_prompt_instance(
        &mut self,
        namespaced_action: &str,
        command_name: &str,
        package_uuid: &PackageUuid,
        block: &CodeBlock,
        _location: &FileLocation,
    ) -> Result<CommandInstanceOrParts, Diagnostic> {
        let Some((namespace, command_id)) = namespaced_action.split_once("::") else {
            todo!("return diagnostic")
        };
        let ctx = self.find_or_create_context(namespace, package_uuid)?;
        let command_id = CommandId::Prompt(command_id.to_string());
        ctx.create_command_instance(&command_id, namespace, command_name, block, package_uuid)
    }
}

lazy_static! {
    pub static ref SET_ENV_UUID: Uuid = Uuid::new_v4();
}

pub async fn start_runbook_runloop(
    runbook: &mut Runbook,
    runtime_context: &mut RuntimeContext,
    block_tx: Sender<BlockEvent>,
    action_item_updates_tx: Sender<ActionItemRequest>,
    action_item_events_rx: Receiver<ActionItemResponse>,
    environments: BTreeMap<String, BTreeMap<String, String>>,
    interactive_by_default: bool,
) -> Result<(), Vec<Diagnostic>> {
    // let mut runbook_state = BTreeMap::new();

    let mut runbook_initialized = false;
    let mut current_node = None;

    let execution_context = CommandExecutionContext {
        review_input_default_values: interactive_by_default,
        review_input_values: interactive_by_default,
    };

    // Compute number of steps
    // A step is
    let (progress_tx, progress_rx) = txtx_addon_kit::channel::unbounded();

    let mut action_item_requests = HashMap::new();
    let mut action_item_responses = HashMap::new();

    loop {
        let event_opt = match action_item_events_rx.try_recv() {
            Ok(action) => Some(action),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                unimplemented!()
            }
        };

        if !runbook_initialized {
            runbook_initialized = true;
            let genesis_panel = Block::new(
                Uuid::new_v4(),
                Panel::ActionPanel(build_genesis_panel(
                    &environments,
                    &runtime_context.selected_env,
                    &runbook,
                )),
            );
            let _ = block_tx.send(BlockEvent::Append(genesis_panel.clone()));
        }

        // Cooldown
        let Some(ActionItemResponse {
            action_item_uuid,
            payload,
        }) = event_opt
        else {
            sleep(Duration::from_millis(3000));
            continue;
        };

        if action_item_uuid == SET_ENV_UUID.clone() {
            reset_runbook_execution(&payload, runbook, runtime_context, &block_tx, &environments);
            continue;
        }

        action_item_responses.insert(action_item_uuid.clone(), payload.clone());

        match &payload {
            ActionItemResponseType::ValidatePanel => {
                let mut runbook_completed = false;
                let mut groups = run_constructs_evaluation(
                    runbook,
                    runtime_context,
                    current_node,
                    &execution_context,
                    &action_item_responses,
                    &progress_tx,
                )
                .await?;

                if groups.is_empty() {
                    runbook_completed = true;
                    groups = collect_runbook_outputs(&runbook, &runtime_context);
                }
                let mut sub_groups = vec![];
                let Some((group, action_items)) = groups.pop_first() else {
                    continue;
                };

                for request in action_items.iter() {
                    action_item_requests.insert(request.uuid.clone(), request.action_type.clone());
                }

                let action_status = if action_items.is_empty() {
                    ActionItemStatus::Success
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
                            index: 0,
                            title: "Validate".into(),
                            description: "".into(),
                            action_status,
                            action_type: ActionItemRequestType::ValidatePanel,
                        }],
                        allow_batch_completion: true,
                    });
                }

                let panel = Block::new(
                    Uuid::new_v4(),
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
            ActionItemResponseType::PickInputOption(response) => {
                // collected_responses.insert(k, v)
            }
            ActionItemResponseType::ProvideInput(_) => {}
            ActionItemResponseType::ReviewInput(_) => {}
            ActionItemResponseType::ProvidePublicKey(_) => todo!(),
            ActionItemResponseType::ProvideSignedTransaction(_) => todo!(),
        };
    }

    Ok(())
}

pub fn reset_runbook_execution(
    payload: &ActionItemResponseType,
    runbook: &mut Runbook,
    runtime_context: &mut RuntimeContext,
    block_tx: &Sender<BlockEvent>,
    environments: &BTreeMap<String, BTreeMap<String, String>>,
) {
    let ActionItemResponseType::PickInputOption(environment_key) = payload else {
        unreachable!(
            "Action item event wih environment uuid sent with invalid payload {:?}",
            payload
        );
    };

    let Some(environment_values) = environments.get(environment_key.as_str()) else {
        unreachable!("Invalid environment variable was sent",);
    };

    let _ = block_tx.send(BlockEvent::Clear);

    runtime_context.set_active_environment(environment_key.into());

    let genesis_panel = Block::new(
        Uuid::new_v4(),
        Panel::ActionPanel(build_genesis_panel(
            &environments,
            &runtime_context.selected_env,
            &runbook,
        )),
    );
    let _ = block_tx.send(BlockEvent::Append(genesis_panel.clone()));
}

pub fn build_genesis_panel(
    environments: &BTreeMap<String, BTreeMap<String, String>>,
    selected_env: &Option<String>,
    runbook: &Runbook,
) -> ActionPanelData {
    let input_options: Vec<InputOption> = environments
        .keys()
        .map(|k| InputOption {
            value: k.to_string(),
            displayed_value: k.to_string(),
        })
        .collect();

    let mut action_items = vec![];

    if environments.len() > 0 {
        action_items.push(ActionItemRequest {
            uuid: SET_ENV_UUID.clone(),
            index: 0,
            title: "select environment".into(),
            description: selected_env.clone().unwrap_or("".to_string()),
            action_status: ActionItemStatus::Todo,
            action_type: ActionItemRequestType::PickInputOption(input_options),
        })
    }

    action_items.push(ActionItemRequest {
        uuid: Uuid::new_v4(),
        index: 0,
        title: "start runbook".into(),
        description: "".into(),
        action_status: if action_items.is_empty() {
            ActionItemStatus::Success
        } else {
            ActionItemStatus::Todo
        },
        action_type: ActionItemRequestType::ValidatePanel,
    });

    ActionPanelData {
        title: "runbook checklist".into(),
        description: "".to_string(),
        groups: vec![ActionGroup {
            title: "lorem ipsum".into(),
            sub_groups: vec![ActionSubGroup {
                action_items,
                allow_batch_completion: true,
            }],
        }],
    }
}
