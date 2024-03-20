use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::RwLock;
use std::{collections::HashMap, sync::Arc};
use txtx_addon_network_stacks::StacksNetworkAddon;
use txtx_core::kit::types::commands::{CommandInstanceStateMachineInput, EvalEvent};
use txtx_core::kit::uuid::Uuid;
use txtx_core::types::Manual;
use txtx_core::{
    eval::run_constructs_evaluation, kit::types::commands::CommandExecutionStatus, simulate_manual,
    types::RuntimeContext, AddonsContext,
};
use txtx_gql::{Context as GqlContext, ContextData};

use crate::{
    manifest::{read_manifest_at_path, read_manuals_from_manifest},
    term_ui, web_ui,
};

use super::{CheckManuals, Context, InspectManual, RunManual};

pub async fn handle_check_command(cmd: &CheckManuals, _ctx: &Context) -> Result<(), String> {
    let manifest_file_path = match cmd.manifest_path {
        Some(ref path) => path.clone(),
        None => "protocol.json".to_string(),
    };
    let manifest = read_manifest_at_path(&manifest_file_path)?;
    let _ = read_manuals_from_manifest(&manifest, None)?;
    // let _ = txtx::check_plan(plan)?;
    Ok(())
}

pub async fn handle_inspect_command(cmd: &InspectManual, _ctx: &Context) -> Result<(), String> {
    let manifest_file_path = match cmd.manifest_path {
        Some(ref path) => path.clone(),
        None => "protocol.json".to_string(),
    };
    let manual_name = cmd.manual.clone().unwrap();
    let manifest = read_manifest_at_path(&manifest_file_path)?;
    let manual = read_manuals_from_manifest(&manifest, Some(&vec![manual_name.clone()]))
        .ok()
        .and_then(|mut m| m.remove(&manual_name))
        .ok_or(format!(
            "unable to find entry '{}' in manifest {}",
            manual_name, manifest_file_path
        ))?;
    let stacks_addon = StacksNetworkAddon::new();
    let mut addons_ctx = AddonsContext::new();
    addons_ctx.register(Box::new(stacks_addon));

    let runtime_context = RuntimeContext::new(addons_ctx);

    let moved_man = manual.clone();
    let mutable_manual = Arc::new(RwLock::new(moved_man));
    let runtime_context_rw_lock = Arc::new(RwLock::new(runtime_context));
    let (eval_tx, _) = channel();
    simulate_manual(&mutable_manual, &runtime_context_rw_lock, eval_tx)?;

    if cmd.no_tui {
        // manual.inspect_constructs();
    } else {
        let _ = term_ui::inspect::main(manual);
    }
    Ok(())
}

pub async fn handle_run_command(cmd: &RunManual, ctx: &Context) -> Result<(), String> {
    let manifest_file_path = match cmd.manifest_path {
        Some(ref path) => path.clone(),
        None => "protocol.json".to_string(),
    };

    let manifest = read_manifest_at_path(&manifest_file_path)?;
    let manuals = read_manuals_from_manifest(&manifest, None)?;

    let mut gql_context = HashMap::new();
    let mut eval_event_ctx = HashMap::new();
    let (eval_event_tx, eval_event_rx) = channel();

    for (manual_name, manual) in manuals.iter() {
        let stacks_addon = StacksNetworkAddon::new();
        let mut addons_ctx = AddonsContext::new();
        addons_ctx.register(Box::new(stacks_addon));

        let runtime_context = RuntimeContext::new(addons_ctx);

        let manual_rw_lock = Arc::new(RwLock::new(manual.clone()));
        let runtime_ctx_rw_lock = Arc::new(RwLock::new(runtime_context));

        simulate_manual(&manual_rw_lock, &runtime_ctx_rw_lock, eval_event_tx.clone())?;

        gql_context.insert(
            manual_name.to_string(),
            ContextData {
                manual: manual_rw_lock.clone(),
                runtime_context: runtime_ctx_rw_lock.clone(),
            },
        );
        eval_event_ctx.insert(manual.uuid.clone(), (manual_rw_lock, runtime_ctx_rw_lock));
    }
    let (web_ui_tx, web_ui_rx) = channel();

    // start thread to listen for evaluation events
    let moved_eval_event_tx = eval_event_tx.clone();
    let _ = std::thread::spawn(move || {
        eval_event_loop(eval_event_rx, moved_eval_event_tx, eval_event_ctx);
    });

    // start web ui server
    let gql_context = GqlContext {
        data: gql_context,
        eval_tx: eval_event_tx.clone(),
    };
    let _ = web_ui::http::start_server(gql_context, ctx).await;
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
    context: HashMap<Uuid, (Arc<RwLock<Manual>>, Arc<RwLock<RuntimeContext>>)>,
) {
    loop {
        match eval_rx.recv() {
            Ok(EvalEvent::AsyncRequestComplete {
                manual_uuid,
                result,
                construct_uuid,
            }) => {
                let Some((manual_rw_lock, runtime_ctx_rw_lock)) = context.get(&manual_uuid) else {
                    unimplemented!(
                        "found no manual associated with graph root {:?}",
                        manual_uuid
                    );
                };

                let Ok(mut manual) = manual_rw_lock.write() else {
                    unimplemented!("unable to acquire lock");
                };

                let Some(command_instance) = manual.commands_instances.get(&construct_uuid) else {
                    unimplemented!(
                        "found no construct_uuid {:?} associated with the manual {:?}",
                        construct_uuid,
                        manual_uuid
                    );
                };

                let result = match command_instance.state.lock() {
                    Ok(mut state_machine) => match result {
                        Ok(status) => match status {
                            CommandExecutionStatus::Complete(result) => {
                                state_machine
                                    .consume(&CommandInstanceStateMachineInput::Successful)
                                    .unwrap();

                                Ok(result)
                            }
                            CommandExecutionStatus::NeedsAsyncRequest => {
                                unimplemented!()
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
                manual
                    .constructs_execution_results
                    .insert(construct_uuid, result.unwrap()); // todo

                println!("rerunning constructs evaluation");
                run_constructs_evaluation(
                    manual_rw_lock,
                    runtime_ctx_rw_lock,
                    None,
                    eval_tx.clone(),
                )
                .unwrap()
            }
            Err(e) => unimplemented!("Channel failed {e}"),
        }
    }
}
