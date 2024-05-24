use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::RwLock;
use std::{collections::HashMap, sync::Arc};
use txtx_addon_network_stacks::StacksNetworkAddon;
use txtx_core::kit::types::commands::{CommandInstanceStateMachineInput, EvalEvent};
use txtx_core::kit::types::diagnostics::Diagnostic;
use txtx_core::kit::uuid::Uuid;
use txtx_core::pre_compute_runbook;
use txtx_core::std::StdAddon;
use txtx_core::types::Runbook;
use txtx_core::{
    eval::prepare_constructs_reevaluation, eval::run_constructs_evaluation,
    kit::types::commands::CommandExecutionStatus, types::RuntimeContext,
    AddonsContext,
};
use txtx_gql::{Context as GqlContext, ContextData};

use crate::{
    manifest::{read_manifest_at_path, read_runbooks_from_manifest},
    term_ui, web_ui,
};

use super::{CheckRunbooks, Context, RunRunbook};

enum RunbookExecutionState {
    RunbookGenesis,
    RunbookGlobalsUpdated,
}

enum Block {
    Checklist(Checklist),
}

struct Checklist {
    uuid: Uuid,
    name: String,
    description: String,
    items: Vec<ChecklistAction>,
}

enum ChecklistActionResultProvider {
    TermConsole,
    LocalWebConsole,
    RemoteWebConsole,
}

enum ChecklistActionStatus {
    Todo,
    Success,
    InProgress(String),
    Error(Diagnostic),
    Warning(Diagnostic),
}

struct ChecklistAction {
    uuid: Uuid,
    name: String,
    description: String,
    status: ChecklistActionStatus,
    action_type: ChecklistActionType,
}

enum ChecklistActionType {
    ReviewInput,
    ProvideInput,
    ProvidePublicKey(ProvidePublicKeyData),
    ProvideSignedTransaction(ProvideSignedTransactionData),
    ValidateChecklist,
}

pub struct ProvidePublicKeyData {
    check_expectation_action_uuid: Option<Uuid>,
}

pub struct ProvideSignedTransactionData {
    check_expectation_action_uuid: Option<Uuid>,
}

struct ChecklistActionEvent {
    checklist_action_uuid: Uuid,
    payload: Vec<u8>,
}


pub async fn handle_check_command(cmd: &CheckRunbooks, _ctx: &Context) -> Result<(), String> {
    let manifest_file_path = match cmd.manifest_path {
        Some(ref path) => path.clone(),
        None => "txtx.json".to_string(),
    };
    let manifest = read_manifest_at_path(&manifest_file_path)?;
    let _ = read_runbooks_from_manifest(&manifest, None)?;
    // let _ = txtx::check_plan(plan)?;
    Ok(())
}

pub async fn handle_run_command(cmd: &RunRunbook, ctx: &Context) -> Result<(), String> {
    let manifest_file_path = match cmd.manifest_path {
        Some(ref path) => path.clone(),
        None => "txtx.json".to_string(),
    };

    let manifest = read_manifest_at_path(&manifest_file_path)?;
    let mut runbooks = read_runbooks_from_manifest(&manifest, None)?;

    let mut gql_context = HashMap::new();

    let (eval_event_tx, eval_event_rx) = channel();
    println!(
        "\n{} Processing manifest '{}'",
        purple!("→"),
        manifest_file_path
    );

    for (runbook_name, (runbook, runtime_context)) in runbooks.iter_mut() {
        let res = pre_compute_runbook(
            runbook,
            runtime_context,
        );
        if let Err(diags) = res {
            for diag in diags.iter() {
                println!("{} {}", red!("x"), diag);
            }
            std::process::exit(1);
        }

        println!(
            "{} Runbook '{}' successfully checked and loaded",
            green!("✓"),
            runbook_name
        );
    }
    let (runbook_name, (mut runbook, mut runtime_context)) = runbooks.into_iter().next().unwrap();

    println!(
        "\n{} Starting runbook '{}'",
        purple!("→"),
        runbook_name
    );

    let (block_tx, block_rx) = channel::<Block>();
    let (checklist_action_updates_tx, checklist_action_updates_rx) = channel::<ChecklistAction>();
    let (checklist_action_events_tx, checklist_action_events_rx) =
        channel::<ChecklistActionEvent>();

    // Frontend:
    // - block_rx
    // - checklist_action_updates_rx
    // - checklist_action_events_tx
    // Responsibility:
    // - listen to block_rx, checklist_action_updates_rx
    // - display UI elements
    // - dispatch `ChecklistActionEvent`

    // Backend:
    // - block_tx
    // - checklist_action_updates_tx
    // - checklist_action_events_rx
    // Responsibility:
    // - execute the graph
    // - build checklist, wait for its completion
    //   - listen to checklist_action_events_rx
    //   - update graph

    // start thread to listen for evaluation events
    let moved_eval_event_tx = eval_event_tx.clone();
    let _ = std::thread::spawn(move || {
        let res = start_runbook_runloop(&mut runbook, &mut runtime_context, block_tx, checklist_action_updates_tx, checklist_action_events_rx);
        if let Err(diags) = res {
            for diag in diags.iter() {
                println!("{} {}", red!("x"), diag);
            }
        }
        std::process::exit(1);
    });


    if cmd.web_console {
        // start web ui server
        let gql_context = GqlContext {
            protocol_name: manifest.name,
            data: gql_context,
            eval_tx: eval_event_tx.clone(),
        };

        let port = 8488;
        println!(
            "\n{} Running Web console\n{}",
            purple!("→"),
            green!(format!("http://127.0.0.1:{}", port))
        );

        let _ = web_ui::http::start_server(gql_context, port, ctx).await;
    }


    Ok(())
}

fn start_runbook_runloop(runbook: &mut Runbook, runtime_context: &mut RuntimeContext, block_tx: Sender<Block>, checklist_action_updates_tx: Sender<ChecklistAction>, checklist_action_events_rx: Receiver<ChecklistActionEvent>) -> Result<(), Vec<Diagnostic>> {
    Ok(())
}

// fn eval_event_loop(
//     eval_rx: Receiver<EvalEvent>,
//     eval_tx: Sender<EvalEvent>,
//     context: HashMap<Uuid, (Arc<RwLock<Runbook>>, Arc<RwLock<RuntimeContext>>)>,
// ) -> Result<(), Vec<Diagnostic>> {
//     loop {
//         match eval_rx.recv() {
//             Ok(EvalEvent::AsyncRequestComplete {
//                 runbook_uuid,
//                 result,
//                 construct_uuid,
//             }) => {
//                 let Some((runbook_rw_lock, runtime_ctx_rw_lock)) = context.get(&runbook_uuid)
//                 else {
//                     unimplemented!(
//                         "found no runbook associated with graph root {:?}",
//                         runbook_uuid
//                     );
//                 };

//                 let Ok(mut runbook) = runbook_rw_lock.write() else {
//                     unimplemented!("unable to acquire lock");
//                 };

//                 let Some(command_instance) = runbook.commands_instances.get(&construct_uuid) else {
//                     unimplemented!(
//                         "found no construct_uuid {:?} associated with the runbook {:?}",
//                         construct_uuid,
//                         runbook_uuid
//                     );
//                 };

//                 let result = match command_instance.state.lock() {
//                     Ok(mut state_machine) => match result {
//                         Ok(status) => match status {
//                             CommandExecutionStatus::Complete(result) => match result {
//                                 Ok(result) => {
//                                     state_machine
//                                         .consume(&CommandInstanceStateMachineInput::Successful)
//                                         .unwrap();
//                                     Ok(result)
//                                 }
//                                 Err(e) => {
//                                     state_machine
//                                         .consume(&CommandInstanceStateMachineInput::Unsuccessful)
//                                         .unwrap();
//                                     Err(e)
//                                 }
//                             },
//                             CommandExecutionStatus::NeedsAsyncRequest => {
//                                 unreachable!()
//                             }
//                         },
//                         Err(e) => {
//                             state_machine
//                                 .consume(&CommandInstanceStateMachineInput::Unsuccessful)
//                                 .unwrap();
//                             Err(e)
//                         }
//                     },
//                     Err(e) => unimplemented!("failed to acquire lock {e}"),
//                 };

//                 runbook
//                     .constructs_execution_results
//                     .insert(construct_uuid.clone(), result); // todo

//                 let command_graph_node = runbook
//                     .constructs_graph_nodes
//                     .get(&construct_uuid.value())
//                     .cloned()
//                     .unwrap();
//                 drop(runbook);
//                 prepare_constructs_reevaluation(&runbook_rw_lock, command_graph_node);

//                 run_constructs_evaluation(
//                     runbook_rw_lock,
//                     runtime_ctx_rw_lock,
//                     Some(command_graph_node),
//                     eval_tx.clone(),
//                 )?;
//             }
//             Err(e) => unimplemented!("Channel failed {e}"),
//         }
//     }
// }
