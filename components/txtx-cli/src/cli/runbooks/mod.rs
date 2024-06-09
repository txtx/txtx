use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::RwLock;
use txtx_core::{
    kit::{
        channel::{self},
        helpers::fs::FileLocation,
        types::frontend::{ActionItemRequest, ActionItemResponse, BlockEvent},
    },
    pre_compute_runbook, start_interactive_runbook_runloop, start_runbook_runloop,
    types::{Runbook, RuntimeContext},
};

use txtx_gql::Context as GqlContext;

use crate::{
    manifest::{
        read_manifest_at_path, read_runbook_from_location, read_runbooks_from_manifest,
        ProtocolManifest,
    },
    web_ui,
};

use super::{CheckRunbooks, Context, RunRunbook};

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
    let (runbook_name, mut runbook, mut runtime_context, environments) =
        match (&cmd.manifest_path, &cmd.runbook_path) {
            (Some(manifest_path), None) => {
                let (manifest, runbook_name, runbook, runtime_context) =
                    load_runbook_from_manifest(&manifest_path).await?;
                (
                    runbook_name,
                    runbook,
                    runtime_context,
                    manifest.environments,
                )
            }
            (None, None) => {
                let (manifest, runbook_name, runbook, runtime_context) =
                    load_runbook_from_manifest("txtx.yml").await?;
                (
                    runbook_name,
                    runbook,
                    runtime_context,
                    manifest.environments,
                )
            }
            (None, Some(runbook_path)) => {
                let (runbook_name, runbook, runtime_context) =
                    load_runbook_from_file_path(&runbook_path).await?;
                (runbook_name, runbook, runtime_context, BTreeMap::new())
            }
            _ => unreachable!(),
        };

    println!("\n{} Starting runbook '{}'", purple!("→"), runbook_name);

    let (block_tx, block_rx) = channel::unbounded::<BlockEvent>();
    let (block_broadcaster, _) = tokio::sync::broadcast::channel(5);
    let (action_item_updates_tx, _action_item_updates_rx) =
        channel::unbounded::<ActionItemRequest>();
    let (action_item_events_tx, action_item_events_rx) = channel::unbounded::<ActionItemResponse>();

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

    let is_execution_interactive = cmd.web_console || cmd.term_console;
    let runbook_description = runbook.description.clone();
    let moved_block_tx = block_tx.clone();
    // Start runloop

    if !is_execution_interactive {
        let res = start_runbook_runloop(&mut runbook, &mut runtime_context, environments).await;
        if let Err(diags) = res {
            for diag in diags.iter() {
                println!("{} {}", red!("x"), diag);
            }
        }
        return Ok(());
    }

    let _ = hiro_system_kit::thread_named("Runbook Runloop").spawn(move || {
        let runloop_future = start_interactive_runbook_runloop(
            &mut runbook,
            &mut runtime_context,
            moved_block_tx,
            action_item_updates_tx,
            action_item_events_rx,
            environments,
        );
        if let Err(diags) = hiro_system_kit::nestable_block_on(runloop_future) {
            for diag in diags.iter() {
                println!("{} {}", red!("x"), diag);
            }
        }
        std::process::exit(1);
    });

    // Start runloop
    let block_store = Arc::new(RwLock::new(BTreeMap::new()));
    let (kill_loops_tx, kill_loops_rx) = std::sync::mpsc::channel();

    let web_ui_handle = if cmd.web_console {
        // start web ui server
        let gql_context = GqlContext {
            protocol_name: runbook_name.clone(),
            runbook_name: runbook_name.clone(),
            runbook_description,
            block_store: block_store.clone(),
            block_broadcaster: block_broadcaster.clone(),
            action_item_events_tx: action_item_events_tx.clone(),
        };

        let port = 8488;
        println!(
            "\n{} Running Web console\n{}",
            purple!("→"),
            green!(format!("http://127.0.0.1:{}", port))
        );

        let handle = web_ui::http::start_server(gql_context, port, ctx)
            .await
            .map_err(|e| format!("Failed to start web ui: {e}"))?;

        Some(handle)
    } else {
        None
    };

    let _ = tokio::spawn(async move {
        loop {
            if let Ok(block_event) = block_rx.recv() {
                let mut block_store = block_store.write().await;
                match block_event.clone() {
                    BlockEvent::Action(new_block) => {
                        let len = block_store.len();
                        block_store.insert(len, new_block.clone());
                    }
                    BlockEvent::Clear => {
                        *block_store = BTreeMap::new();
                    }
                    BlockEvent::UpdateActionItems(updates) => {
                        for update in updates.iter() {
                            for (_, block) in block_store.iter_mut() {
                                block.update_action_item(update.clone());
                            }
                        }
                    }
                    BlockEvent::Modal(new_block) => {
                        let len = block_store.len();
                        block_store.insert(len, new_block.clone());
                    }
                    BlockEvent::ProgressBar(new_block) => {
                        let len = block_store.len();
                        block_store.insert(len, new_block.clone());
                    }
                    BlockEvent::RunbookCompleted => unimplemented!("Runbook completed!"),
                    BlockEvent::Exit => break,
                }
                let _ = block_broadcaster.send(block_event.clone());
            }
        }
    });

    let _ = tokio::spawn(async move {
        match kill_loops_rx.recv() {
            Ok(_) => {
                if let Some(handle) = web_ui_handle {
                    let _ = handle.stop(true).await;
                }
                let _ = block_tx.send(BlockEvent::Exit);
            }
            Err(_) => {}
        };
    });
    ctrlc::set_handler(move || {
        kill_loops_tx
            .send(true)
            .expect("Could not send signal on channel to kill web ui.")
    })
    .expect("Error setting Ctrl-C handler");

    Ok(())
}

pub async fn load_runbook_from_manifest(
    manifest_path: &str,
) -> Result<(ProtocolManifest, String, Runbook, RuntimeContext), String> {
    let manifest = read_manifest_at_path(&manifest_path)?;
    let mut runbooks = read_runbooks_from_manifest(&manifest, None)?;

    println!("\n{} Processing manifest '{}'", purple!("→"), manifest_path);

    for (runbook_name, (runbook, runtime_context)) in runbooks.iter_mut() {
        let res = pre_compute_runbook(runbook, runtime_context);
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

    // Select first runbook by default
    let (runbook_name, (runbook, runtime_context)) = runbooks.into_iter().next().unwrap();
    Ok((manifest, runbook_name, runbook, runtime_context))
}

pub async fn load_runbook_from_file_path(
    file_path: &str,
) -> Result<(String, Runbook, RuntimeContext), String> {
    let location = FileLocation::from_path_string(file_path)?;

    let (runbook_name, mut runbook, mut runtime_context) =
        read_runbook_from_location(&location, &None, &BTreeMap::new())?;

    println!("\n{} Processing file '{}'", purple!("→"), file_path);

    let res = pre_compute_runbook(&mut runbook, &mut runtime_context);
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

    // Select first runbook by default
    Ok((runbook_name, runbook, runtime_context))
}
