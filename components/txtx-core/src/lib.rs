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
use eval::run_constructs_evaluation;
use kit::types::frontend::ActionItemPayload;
use kit::types::frontend::BlockEvent;
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

pub async fn start_runbook_runloop(
    runbook: &mut Runbook,
    runtime_context: &mut RuntimeContext,
    block_tx: Sender<Block>,
    action_item_updates_tx: Sender<ActionItem>,
    action_item_events_rx: Receiver<ActionItemEvent>,
    environments: BTreeMap<String, BTreeMap<String, String>>,
    interactive_by_default: bool,
) -> Result<(), Vec<Diagnostic>> {
    // let mut runbook_state = BTreeMap::new();

    let mut runbook_initialized = false;
    let mut current_block = None;

    // Compute number of steps
    // A step is

    loop {
        let event_opt = match action_item_events_rx.try_recv() {
            Ok(action) => Some(action),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                unimplemented!()
            }
        };

        if runbook_initialized {
            runbook_initialized = false;
            let genesis_panel = Block::ActionPanel(build_genesis_panel(&environments, &runbook));
            let _ = block_tx.send(genesis_panel.clone());
            current_block = Some(genesis_panel);
        }

        // Cooldown
        let Some(event) = event_opt else {
            sleep(Duration::from_millis(3000));
            continue;
        };





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

pub fn build_genesis_panel(environments: &BTreeMap<String, BTreeMap<String, String>>, runbook: &Runbook) -> ActionPanelData {
    let input_options: Vec<InputOption> = environments
        .keys()
        .map(|k| InputOption {
            value: k.to_string(),
            displayed_value: k.to_string(),
        })
        .collect();

    let env_selector = ActionItem {
        uuid: Uuid::new_v4(),
        index: 0,
        title: "SELECT ENVIRONMENT".into(),
        description: "".into(),
        action_status: ActionItemStatus::Todo,
        action_type: ActionItemType::PickInputOption(input_options),
    };

    let start_runbook_action = ActionItem {
        uuid: Uuid::new_v4(),
        index: 0,
        title: "START RUNBOOK".into(),
        description: "".into(),
        action_status: ActionItemStatus::Todo,
        action_type: ActionItemType::ValidatePanel,
    };

    let environment_selection_required: bool = environments.len() > 1;

    ActionPanelData {
        uuid: Uuid::new_v4(),
        title: "Runbook Checklist".into(),
        description: "".to_string(),
        groups: vec![ActionGroup {
            title: "Lorem ipsum".into(),
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
