#[macro_use]
extern crate lazy_static;

#[macro_use]
pub extern crate txtx_addon_kit as kit;

pub extern crate crossbeam_channel as channel;

pub mod errors;
pub mod eval;
pub mod std;
pub mod types;
pub mod visitor;

use ::std::collections::BTreeMap;
use ::std::collections::HashMap;
use ::std::sync::mpsc::channel;
use ::std::thread::sleep;
use ::std::time::Duration;

use channel::Receiver;
use channel::Sender;
use channel::TryRecvError;
use eval::get_sorted_nodes;
use eval::prepare_constructs_reevaluation;
use eval::run_constructs_evaluation;
use kit::types::commands::CommandInstanceStateMachineInput;
use kit::types::frontend::ActionItemPayload;
use kit::types::frontend::BlockEvent;
use kit::types::frontend::Panel;
use kit::types::frontend::ProvidedInputData;
use kit::types::types::Type;
use kit::types::types::Value;
use kit::types::ConstructUuid;
use kit::uuid::Uuid;
use kit::AddonDefaults;
use txtx_addon_kit::hcl::structure::Block as CodeBlock;
use txtx_addon_kit::helpers::fs::FileLocation;
use txtx_addon_kit::types::commands::CommandId;
use txtx_addon_kit::types::commands::CommandInstanceOrParts;
use txtx_addon_kit::types::commands::EvalEvent;
use txtx_addon_kit::types::diagnostics::Diagnostic;
use txtx_addon_kit::types::frontend::ActionGroup;
use txtx_addon_kit::types::frontend::ActionItem;
use txtx_addon_kit::types::frontend::ActionItemEvent;
use txtx_addon_kit::types::frontend::ActionItemStatus;
use txtx_addon_kit::types::frontend::ActionItemType;
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

// pub fn simulate_runbook(
//     runbook: &Arc<RwLock<Runbook>>,
//     runtime_context: &Arc<RwLock<RuntimeContext>>,
//     eval_tx: Sender<EvalEvent>,
// ) -> Result<(), Vec<Diagnostic>> {
//     match runtime_context.write() {
//         Ok(mut runtime_context) => {
//             let _ = run_constructs_indexing(runbook, &mut runtime_context)?;
//             let _ = run_constructs_checks(runbook, &mut runtime_context.addons_ctx)?;
//             let _ = run_constructs_dependencies_indexing(runbook, &mut runtime_context)?;
//         }
//         Err(e) => unimplemented!("could not acquire lock: {e}"),
//     }
//     let _ = run_constructs_evaluation(runbook, runtime_context, None, eval_tx)?;
//     Ok(())
// }

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
    action_item_updates_tx: Sender<ActionItem>,
    action_item_events_rx: Receiver<ActionItemEvent>,
    environments: BTreeMap<String, BTreeMap<String, String>>,
    interactive_by_default: bool,
) -> Result<(), Vec<Diagnostic>> {
    // let mut runbook_state = BTreeMap::new();

    let mut runbook_initialized = false;
    let mut current_block = None;
    let (tx, rx) = channel();

    // Compute number of steps
    // A step is

    loop {
        let event_opt = match action_item_events_rx.try_recv() {
            Ok(action) => {
                println!("received action");
                Some(action)
            }
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
            current_block = Some(genesis_panel);
        }

        // Cooldown
        let Some(event) = event_opt else {
            sleep(Duration::from_millis(3000));
            continue;
        };

        let ActionItemEvent {
            action_item_uuid,
            payload,
        } = event;

        if action_item_uuid == SET_ENV_UUID.clone() {
            let ActionItemPayload::PickInputOption(env) = payload else {
                // todo: we can probably just continue here, but for now we'll panic to see if this is ever a problem
                unreachable!(
                    "Action item event wih environment uuid sent with invalid payload {:?}",
                    payload
                );
            };

            let env = match environments.get(&env) {
                Some(_) => env,
                None => {
                    // an invalid environment variable was sent
                    continue;
                }
            };

            let _ = block_tx.send(BlockEvent::Clear);
            runtime_context.set_active_environment(env);

            let genesis_panel = Block::new(
                Uuid::new_v4(),
                Panel::ActionPanel(build_genesis_panel(
                    &environments,
                    &runtime_context.selected_env,
                    &runbook,
                )),
            );
            let _ = block_tx.send(BlockEvent::Append(genesis_panel.clone()));

            let _ = run_constructs_dependencies_indexing(runbook, runtime_context)?;
            let _ = run_constructs_evaluation(runbook, runtime_context, None, tx.clone()).await?;

            let ordered_nodes = get_sorted_nodes(runbook.constructs_graph.clone());
            let graph = runbook.constructs_graph.clone();

            let mut provide_input_actions = vec![];
            let mut review_input_actions = vec![];
            let mut review_output_actions = vec![];
            let mut panels = vec![];
            for (_, node) in ordered_nodes.into_iter().enumerate() {
                let uuid = graph
                    .node_weight(node)
                    .expect("unable to retrieve construct");
                let construct_uuid = ConstructUuid::Local(uuid.clone());

                let Some(command_instance) = runbook.commands_instances.get(&construct_uuid) else {
                    continue;
                };

                let evaluated_inputs = runbook
                    .command_inputs_evaluation_results
                    .get(&construct_uuid)
                    .unwrap();

                let Some(mut action) = command_instance.get_action(
                    evaluated_inputs,
                    construct_uuid,
                    AddonDefaults::new(), // todo
                    0,
                ) else {
                    continue;
                };
                match command_instance.specification.matcher.as_str() {
                    "input" => match action.action_type {
                        ActionItemType::ProvideInput(_) => {
                            action.index = provide_input_actions.len() as u16;
                            provide_input_actions.push(action);
                        }
                        ActionItemType::ReviewInput => {
                            action.index = review_input_actions.len() as u16;
                            review_input_actions.push(action);
                        }
                        _ => unreachable!(),
                    },
                    "output" => {
                        action.index = review_output_actions.len() as u16;
                        review_output_actions.push(action);
                    }
                    _ => {
                        panels.push(Block::new(
                            Uuid::new_v4(),
                            Panel::new_action_panel(
                                &command_instance.specification.name,
                                "",
                                vec![ActionGroup::new(
                                    "lorem ipsum",
                                    vec![ActionSubGroup::new(vec![action], false)],
                                )],
                            ),
                        ));
                    }
                };
            }

            // send the input block
            let input_panel = Block::new(
                Uuid::new_v4(),
                Panel::new_action_panel(
                    "inputs review",
                    "",
                    vec![
                        ActionGroup::new(
                            "provide inputs",
                            vec![ActionSubGroup::new(provide_input_actions, true)],
                        ),
                        ActionGroup::new(
                            "review inputs",
                            vec![ActionSubGroup::new(review_input_actions, true)],
                        ),
                    ],
                ),
            );
            let _ = block_tx.send(BlockEvent::Append(input_panel)).unwrap();

            // then send each of the other blocks
            for panel in panels {
                let _ = block_tx.send(BlockEvent::Append(panel)).unwrap();
            }
            // finally, send our output block
            let output_panel = Block::new(
                Uuid::new_v4(),
                Panel::new_action_panel(
                    "outputs review",
                    "",
                    vec![ActionGroup::new(
                        "review outputs",
                        vec![ActionSubGroup::new(review_output_actions, true)],
                    )],
                ),
            );

            let _ = block_tx.send(BlockEvent::Append(output_panel)).unwrap();
            continue;
        } else {
            let Some(current_block) = current_block.clone() else {
                println!("not found in current block");
                continue;
            };
            let Some(action_item) = current_block.find_action(action_item_uuid) else {
                continue;
            };

            let new_status_payload = match payload {
                ActionItemPayload::ReviewInput => (
                    current_block.uuid,
                    action_item.uuid,
                    ActionItemStatus::Success,
                ),
                ActionItemPayload::ProvideInput(ProvidedInputData {
                    input_name,
                    value,
                    typing,
                }) => {
                    println!("got provide input event!");
                    let value: Result<Value, Diagnostic> =
                        Value::from_string(value, Type::Primitive(typing), None);

                    let construct_uuid = ConstructUuid::Local(action_item_uuid);

                    let input_eval_results = runbook
                        .command_inputs_evaluation_results
                        .get_mut(&construct_uuid);
                    match input_eval_results {
                        Some(input_eval_results) => {
                            input_eval_results.insert(&input_name, value.clone())
                        }
                        None => {}
                    }

                    let Some(command_instance) =
                        runbook.commands_instances.get_mut(&construct_uuid)
                    else {
                        println!("couldn't find command instance");
                        continue;
                    };
                    match command_instance.state.lock() {
                        Ok(mut state_machine) => {
                            state_machine
                                .consume(&CommandInstanceStateMachineInput::ReEvaluate)
                                .unwrap();
                        }
                        Err(e) => panic!("unable to acquire lock {e}"),
                    };

                    let Some(command_graph_node) = runbook
                        .constructs_graph_nodes
                        .get(&construct_uuid.value())
                        .cloned()
                    else {
                        println!("missing from graph?");
                        // if somehow this construct is not part of the graph, we don't need to reevaluate it
                        continue;
                    };

                    prepare_constructs_reevaluation(runbook, command_graph_node.clone());
                    match run_constructs_evaluation(
                        runbook,
                        runtime_context,
                        Some(command_graph_node.clone()),
                        tx.clone(),
                    )
                    .await
                    {
                        Ok(()) => println!("successfully reevaluated constructs after mutation"),
                        Err(e) => println!("error reevaluating constructs after mutation: {:?}", e),
                    }

                    match value {
                        Ok(_) => (
                            current_block.uuid,
                            action_item.uuid,
                            ActionItemStatus::Success,
                        ),
                        Err(e) => (
                            current_block.uuid,
                            action_item.uuid,
                            ActionItemStatus::Error(e),
                        ),
                    }
                }
                ActionItemPayload::PickInputOption(_) => todo!(),
                ActionItemPayload::ProvidePublicKey(_) => todo!(),
                ActionItemPayload::ProvideSignedTransaction(_) => todo!(),
                ActionItemPayload::ValidatePanel => todo!(),
            };
            let _ = block_tx.send(BlockEvent::SetActionItemStatus(new_status_payload));
        }

        // Retrieve action via its UUID
        // event.checklist_action_uuid

        // the action is pointing to the construct
        // "send" the payload to the construct, it will know what to do with it?
        // the action can also have a "next action"

        // do we have an ongoing block?
        // retrieve all the actions of the checklist

        // recompute the graph

        // while promises are being returned
        // collect the promises

        // Runbook Execution returns
        // - 1 result
        // - 1 action
    }

    Ok(())
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

    let env_selector = ActionItem {
        uuid: SET_ENV_UUID.clone(),
        index: 0,
        title: "select environment".into(),
        description: selected_env.clone().unwrap_or("".to_string()),
        action_status: ActionItemStatus::Todo,
        action_type: ActionItemType::PickInputOption(input_options),
    };

    let start_runbook_action = ActionItem {
        uuid: Uuid::new_v4(),
        index: 0,
        title: "start runbook".into(),
        description: "".into(),
        action_status: ActionItemStatus::Todo,
        action_type: ActionItemType::ValidatePanel,
    };

    let environment_selection_required: bool = environments.len() > 1;

    ActionPanelData {
        title: "runbook checklist".into(),
        description: "".to_string(),
        groups: vec![ActionGroup {
            title: "lorem ipsum".into(),
            sub_groups: vec![ActionSubGroup {
                action_items: vec![env_selector, start_runbook_action],
                allow_batch_completion: true,
            }],
        }],
    }
}

#[test]
fn test_genesis_panel() {
    // build_genesis_panel()
}
