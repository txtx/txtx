use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::RwLock;
use std::{collections::HashMap, sync::Arc};
use txtx_addon_network_stacks::StacksNetworkAddon;
use txtx_core::kit::types::commands::{CommandInstanceStateMachineInput, EvalEvent};
use txtx_core::kit::types::diagnostics::Diagnostic;
use txtx_core::kit::uuid::Uuid;
use txtx_core::std::StdAddon;
use txtx_core::types::Runbook;
use txtx_core::{
    eval::prepare_constructs_reevaluation, eval::run_constructs_evaluation,
    kit::types::commands::CommandExecutionStatus, simulate_runbook, types::RuntimeContext,
    AddonsContext,
};
use txtx_gql::{Context as GqlContext, ContextData};

use crate::{
    manifest::{read_manifest_at_path, read_runbooks_from_manifest},
    term_ui, web_ui,
};

use super::{CheckRunbooks, Context, InspectRunbook, RunRunbook};

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

pub async fn handle_inspect_command(cmd: &InspectRunbook, _ctx: &Context) -> Result<(), String> {
    let manifest_file_path = match cmd.manifest_path {
        Some(ref path) => path.clone(),
        None => "txtx.json".to_string(),
    };
    let runbook_name = cmd.runbook.clone().unwrap();
    let manifest = read_manifest_at_path(&manifest_file_path)?;
    let runbook = read_runbooks_from_manifest(&manifest, Some(&vec![runbook_name.clone()]))
        .ok()
        .and_then(|mut m| m.remove(&runbook_name))
        .ok_or(format!(
            "unable to find entry '{}' in manifest {}",
            runbook_name, manifest_file_path
        ))?;

    let mut addons_ctx = AddonsContext::new();
    addons_ctx.register(Box::new(StdAddon::new()));
    addons_ctx.register(Box::new(StacksNetworkAddon::new()));

    let runtime_context = RuntimeContext::new(addons_ctx, manifest.environments.clone());

    let moved_man = runbook.clone();
    let mutable_runbook = Arc::new(RwLock::new(moved_man));
    let runtime_context_rw_lock = Arc::new(RwLock::new(runtime_context));
    let (eval_tx, _) = channel();
    simulate_runbook(&mutable_runbook, &runtime_context_rw_lock, eval_tx)?;

    if cmd.no_tui {
        // runbook.inspect_constructs();
    } else {
        let _ = term_ui::inspect::main(runbook);
    }
    Ok(())
}

pub async fn handle_run_command(cmd: &RunRunbook, ctx: &Context) -> Result<(), String> {
    let manifest_file_path = match cmd.manifest_path {
        Some(ref path) => path.clone(),
        None => "txtx.json".to_string(),
    };

    let manifest = read_manifest_at_path(&manifest_file_path)?;
    let runbooks = read_runbooks_from_manifest(&manifest, None)?;

    let mut gql_context = HashMap::new();
    let mut eval_event_ctx = HashMap::new();
    let (eval_event_tx, eval_event_rx) = channel();
    println!(
        "\n{} Processing manifest '{}'",
        purple!("→"),
        manifest_file_path
    );

    for (runbook_name, runbook) in runbooks.iter() {
        let mut addons_ctx = AddonsContext::new();
        addons_ctx.register(Box::new(StdAddon::new()));
        addons_ctx.register(Box::new(StacksNetworkAddon::new()));

        let runtime_context = RuntimeContext::new(addons_ctx, manifest.environments.clone());

        let runbook_rw_lock = Arc::new(RwLock::new(runbook.clone()));
        let runtime_ctx_rw_lock = Arc::new(RwLock::new(runtime_context));

        simulate_runbook(
            &runbook_rw_lock,
            &runtime_ctx_rw_lock,
            eval_event_tx.clone(),
        )?;

        println!(
            "{} Runbook '{}' successfully checked and loaded",
            green!("✓"),
            runbook_name
        );

        gql_context.insert(
            runbook_name.to_string(),
            ContextData {
                runbook: runbook_rw_lock.clone(),
                runtime_context: runtime_ctx_rw_lock.clone(),
            },
        );
        eval_event_ctx.insert(runbook.uuid.clone(), (runbook_rw_lock, runtime_ctx_rw_lock));
    }
    let (web_ui_tx, web_ui_rx) = channel();

    // start thread to listen for evaluation events
    let moved_eval_event_tx = eval_event_tx.clone();
    let _ = std::thread::spawn(move || {
        let res = eval_event_loop(eval_event_rx, moved_eval_event_tx, eval_event_ctx);
        if let Err(diags) = res {
            for diag in diags.iter() {
                println!(
                    "{} {}",
                    red!("x"),
                    diag
                );
            }
        }
        std::process::exit(1);
    });

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
    match web_ui_rx.recv() {
        Ok(_) => {}
        Err(_) => {}
    };
    let _ = web_ui_tx.send(true);

    Ok(())
}

fn eval_event_loop(
    eval_rx: Receiver<EvalEvent>,
    eval_tx: Sender<EvalEvent>,
    context: HashMap<Uuid, (Arc<RwLock<Runbook>>, Arc<RwLock<RuntimeContext>>)>,
) -> Result<(), Vec<Diagnostic>> {
    loop {
        match eval_rx.recv() {
            Ok(EvalEvent::AsyncRequestComplete {
                runbook_uuid,
                result,
                construct_uuid,
            }) => {
                let Some((runbook_rw_lock, runtime_ctx_rw_lock)) = context.get(&runbook_uuid)
                else {
                    unimplemented!(
                        "found no runbook associated with graph root {:?}",
                        runbook_uuid
                    );
                };

                let Ok(mut runbook) = runbook_rw_lock.write() else {
                    unimplemented!("unable to acquire lock");
                };

                let Some(command_instance) = runbook.commands_instances.get(&construct_uuid) else {
                    unimplemented!(
                        "found no construct_uuid {:?} associated with the runbook {:?}",
                        construct_uuid,
                        runbook_uuid
                    );
                };

                let result = match command_instance.state.lock() {
                    Ok(mut state_machine) => match result {
                        Ok(status) => match status {
                            CommandExecutionStatus::Complete(result) => match result {
                                Ok(result) => {
                                    state_machine
                                        .consume(&CommandInstanceStateMachineInput::Successful)
                                        .unwrap();
                                    Ok(result)
                                }
                                Err(e) => {
                                    state_machine
                                        .consume(&CommandInstanceStateMachineInput::Unsuccessful)
                                        .unwrap();
                                    Err(e)
                                }
                            },
                            CommandExecutionStatus::NeedsAsyncRequest => {
                                unreachable!()
                            }
                        },
                        Err(e) => {
                            state_machine
                                .consume(&CommandInstanceStateMachineInput::Unsuccessful)
                                .unwrap();
                            Err(e)
                        }
                    },
                    Err(e) => unimplemented!("failed to acquire lock {e}"),
                };

                runbook
                    .constructs_execution_results
                    .insert(construct_uuid.clone(), result); // todo

                let command_graph_node = runbook
                    .constructs_graph_nodes
                    .get(&construct_uuid.value())
                    .cloned()
                    .unwrap();
                drop(runbook);
                prepare_constructs_reevaluation(&runbook_rw_lock, command_graph_node);

                run_constructs_evaluation(
                    runbook_rw_lock,
                    runtime_ctx_rw_lock,
                    Some(command_graph_node),
                    eval_tx.clone(),
                )?;
            }
            Err(e) => unimplemented!("Channel failed {e}"),
        }
    }
}
