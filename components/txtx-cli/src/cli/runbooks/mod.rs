use std::{
    collections::BTreeMap,
    sync::{mpsc::channel, Arc, RwLock},
};
use txtx_core::{
    channel::select,
    kit::types::frontend::{ActionItem, ActionItemEvent, ActionItemPayload, BlockEvent},
    pre_compute_runbook, start_runbook_runloop, SET_ENV_UUID,
};
use txtx_gql::Context as GqlContext;

use crate::{
    manifest::{read_manifest_at_path, read_runbooks_from_manifest},
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
    let manifest_file_path = match cmd.manifest_path {
        Some(ref path) => path.clone(),
        None => "txtx.json".to_string(),
    };

    let manifest = read_manifest_at_path(&manifest_file_path)?;
    let mut runbooks = read_runbooks_from_manifest(&manifest, None)?;

    println!(
        "\n{} Processing manifest '{}'",
        purple!("→"),
        manifest_file_path
    );

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
    let (runbook_name, (mut runbook, mut runtime_context)) = runbooks.into_iter().next().unwrap();

    println!("\n{} Starting runbook '{}'", purple!("→"), runbook_name);

    let (block_tx, block_rx) = txtx_core::channel::unbounded::<Block>();
    let (action_item_updates_tx, action_item_updates_rx) =
        txtx_core::channel::unbounded::<ActionItem>();
    let (action_item_events_tx, action_item_events_rx) =
        txtx_core::channel::unbounded::<ActionItemEvent>();

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

    let interactive_by_default = cmd.web_console;
    let environments = manifest.environments.clone();

    // Start runloop
    let _ = hiro_system_kit::thread_named("Runbook Runloop").spawn(move || {
        let runloop_future = start_runbook_runloop(
            &mut runbook,
            &mut runtime_context,
            block_tx,
            action_item_updates_tx,
            action_item_events_rx,
            environments,
            interactive_by_default,
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

    if cmd.web_console {
        // start web ui server
        let gql_context = GqlContext {
            protocol_name: manifest.name,
            block_store: block_store.clone(),
            action_item_events_tx,
        };

        let port = 8488;
        println!(
            "\n{} Running Web console\n{}",
            purple!("→"),
            green!(format!("http://127.0.0.1:{}", port))
        );

        let _ = web_ui::http::start_server(gql_context, port, ctx).await;
    }

    let _ = hiro_system_kit::thread_named("Blocks Store Update Runloop").spawn(move || loop {
        select! {
            recv(block_rx) -> msg => {
                if let Ok((new_block)) = msg {
                    // Add new_block to block_store
                }
            }
            recv(action_item_updates_rx) -> msg => {
                if let Ok(new_action_update) = msg {
                    // Retrieve item to update from store
                }
            }
        }
    });

    Ok(())
}
