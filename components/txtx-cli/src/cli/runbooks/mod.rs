use console::Style;
use convert_case::{Case, Casing};
use dialoguer::{theme::ColorfulTheme, Input, Select};
use std::{
    collections::{BTreeMap, HashMap, HashSet}, env, fmt::format, fs::{self, File}, path::PathBuf, sync::Arc
};
use tokio::sync::RwLock;
use txtx_core::{
    kit::{
        channel,
        helpers::fs::FileLocation,
        types::{
            commands::CommandInputsEvaluationResult,
            frontend::{ActionItemRequest, ActionItemResponse, BlockEvent},
            types::Value,
        },
    },
    pre_compute_runbook, start_interactive_runbook_runloop, start_runbook_runloop,
    types::{ProtocolManifest, Runbook, RunbookMetadata, RuntimeContext},
};

use txtx_gql::Context as GqlContext;

use crate::{
    cli::templates::{build_manifest_data, build_runbook_data},
    manifest::{read_manifest_at_path, read_runbook_from_location, read_runbooks_from_manifest},
    web_ui,
};

const DEFAULT_PORT_TXTX: u16 = 8488;

use super::{CheckRunbooks, Context, CreateRunbook, ExecuteRunbook, ListRunbooks};

pub async fn handle_check_command(cmd: &CheckRunbooks, _ctx: &Context) -> Result<(), String> {
    let manifest_file_path = match cmd.manifest_path {
        Some(ref path) => path.clone(),
        None => "txtx.yml".to_string(),
    };
    let manifest = read_manifest_at_path(&manifest_file_path)?;
    let _ = read_runbooks_from_manifest(&manifest, None)?;
    // let _ = txtx::check_plan(plan)?;
    Ok(())
}

pub async fn handle_new_command(cmd: &CreateRunbook, _ctx: &Context) -> Result<(), String> {
    let manifest_res = match &cmd.manifest_path {
        Some(manifest_path) => read_manifest_at_path(&manifest_path),
        None => read_manifest_at_path("txtx.yml"),
    };

    let theme = ColorfulTheme {
        values_style: Style::new().green(),
        ..ColorfulTheme::default()
    };

    let mut manifest = match manifest_res {
        Ok(manifest) => manifest,
        Err(_) => {
            // Ask for the name of the project
            let name: String = Input::new()
                .with_prompt("Enter the name of this project")
                .interact_text()
                .unwrap();

            ProtocolManifest {
                name,
                runbooks: vec![],
                environments: BTreeMap::new(),
                location: None,
            }
        }
    };

    // Choose between deploy, operate, pause, and other
    let choices = vec![
        "Maintenance: update settings, authorize new contracts, etc.",
        "Emergencies: pause contracts, authorization rotations, etc.",
        "Others",
    ];
    let folders = vec!["maintenance", "emergencies", "other"];
    let choice = Select::with_theme(&theme)
        .with_prompt("Choose a type of Runbook:")
        .default(0)
        .items(&choices)
        .interact()
        .unwrap();
    let mut action = folders[choice].to_string();

    // If 'other' is chosen, ask for a custom action name
    if action == "other" {
        action = Input::with_theme(&theme)
            .with_prompt("Enter a custom action name")
            .interact_text()
            .unwrap();
    }

    // Provide a name for the runbook
    let mut runbook_name: String = Input::with_theme(&theme)
        .with_prompt("Enter a name for the runbook (e.g., bns-multisig.tx)")
        .validate_with(|input: &String| {
            if input.ends_with(".tx") && input.chars().next().unwrap().is_lowercase() {
                Ok(())
            } else {
                Err("Runbook name must be in camelCase and end with .tx")
            }
        })
        .interact_text()
        .unwrap();

    // Normalize names
    runbook_name = runbook_name.to_case(Case::Kebab);
    if !runbook_name.ends_with(".tx") {
        runbook_name = format!("{}.tx", runbook_name);
    }

    // Provide a description (optional)
    let runbook_description: String = Input::with_theme(&theme)
        .with_prompt("Enter the description for the runbook (optional)")
        .allow_empty(true)
        .interact_text()
        .unwrap();

    manifest.runbooks.push(RunbookMetadata {
        location: format!("runbooks/{}/{}", action, runbook_name),
        name: runbook_name
            .strip_suffix(".tx")
            .unwrap()
            .to_ascii_lowercase(),
        description: Some(runbook_description),
    });

    // Initialize root location
    let root_location_path: PathBuf = env::current_dir().expect("Failed to get current directory");
    let root_location = FileLocation::from_path(root_location_path.clone());

    let mut runbook_file_path = root_location_path.clone();
    runbook_file_path.push("runbooks");

    if manifest.location.is_none() {
        // Create manifest file
        let manifest_name = "txtx.yml";
        let mut manifest_location = root_location.clone();
        let _ = manifest_location.append_path(manifest_name);

        let mut manifest_file =
            File::create(manifest_location.to_string()).expect("creation failed");
        let manifest_file_data = build_manifest_data(&manifest);
        let template = mustache::compile_str(include_str!("../templates/txtx.yml.mst"))
            .expect("Failed to compile template");
        template
            .render_data(&mut manifest_file, &manifest_file_data)
            .expect("Failed to render template");
        println!("{} {}", green!("Created manifest"), manifest_name);

        // Create runbooks directory
        fs::create_dir_all(&runbook_file_path).map_err(|e| {
            format!(
                "unable to create parent directory {}\n{}",
                runbook_file_path.display(),
                e
            )
        })?;
        println!("{} runbooks", green!("Created directory"));
    }

    let mut readme_file_path = runbook_file_path.clone();
    readme_file_path.push("README.md");
    let mut readme_file = File::create(readme_file_path).expect("creation failed");
    let readme_file_data = build_manifest_data(&manifest);
    let template = mustache::compile_str(include_str!("../templates/readme.md.mst"))
        .expect("Failed to compile template");
    template
        .render_data(&mut readme_file, &readme_file_data)
        .expect("Failed to render template");
    println!("{} runbooks/README.md", green!("Created file"));

    // Create runbooks subdirectory
    runbook_file_path.push(action);
    fs::create_dir_all(&runbook_file_path).map_err(|e| {
        format!(
            "unable to create parent directory {}\n{}",
            runbook_file_path.display(),
            e
        )
    })?;
    let runbook_location = FileLocation::from_path(runbook_file_path.clone());
    println!(
        "{} {}",
        green!("Created directory"),
        runbook_location
            .get_relative_path_from_base(&root_location)
            .unwrap()
    );

    // Create runbook
    runbook_file_path.push(runbook_name.clone());
    let mut runbook_file = File::create(runbook_file_path.clone()).expect("creation failed");
    let runbook_file_data = build_runbook_data(&runbook_name);
    let template = mustache::compile_str(include_str!("../templates/runbook.tx.mst"))
        .expect("Failed to compile template");
    template
        .render_data(&mut runbook_file, &runbook_file_data)
        .expect("Failed to render template");
    let runbook_location = FileLocation::from_path(runbook_file_path);
    println!(
        "{} {}",
        green!("Created runbook"),
        runbook_location
            .get_relative_path_from_base(&root_location)
            .unwrap()
    );

    Ok(())
}

pub async fn handle_list_command(cmd: &ListRunbooks, _ctx: &Context) -> Result<(), String> {
    let manifest = match &cmd.manifest_path {
        Some(manifest_path) => read_manifest_at_path(&manifest_path)?,
        None => read_manifest_at_path("txtx.yml")?,
    };
    if manifest.runbooks.is_empty() {
        println!("{}: no runbooks referenced in the txtx.yml manifest.\nRun the command `txtx new` to create a new runbook.", yellow!("warning"));
        std::process::exit(1);
    }
    for runbook in manifest.runbooks {
        println!(
            "{}\t\t{}",
            runbook.name,
            yellow!(format!("{}", runbook.description.unwrap_or("".into())))
        );
    }
    Ok(())
}

pub async fn handle_run_command(cmd: &ExecuteRunbook, ctx: &Context) -> Result<(), String> {
    let res = match &cmd.manifest_path {
        Some(manifest_path) => load_runbook_from_manifest(&manifest_path, &cmd.runbook).await,
        None => load_runbook_from_manifest("txtx.yml", &cmd.runbook).await,
    };
    let (runbook_name, mut runbook, mut runtime_context, environments) = match res {
        Ok((m, a, b, c)) => (a, b, c, m.environments),
        Err(_) => {
            let (runbook_name, runbook, runtime_context) =
                load_runbook_from_file_path(&cmd.runbook).await?;
            (runbook_name, runbook, runtime_context, BTreeMap::new())
        }
    };

    if let Some(ref selected_env) = cmd.environment {
        runtime_context.set_active_environment(selected_env.clone())?;
    }

    let mut batch_inputs = vec![];

    for input in cmd.inputs.iter() {
        let (input_name, input_value) = input.split_once("=").unwrap();
        for (construct_uuid, command_instance) in runbook.commands_instances.iter_mut() {
            if command_instance.specification.matcher.eq("input")
                && input_name.eq(&command_instance.name)
            {
                let mut result = CommandInputsEvaluationResult::new(input_name);

                if input_value.starts_with("[") {
                    batch_inputs = input_value[1..(input_value.len() - 2)]
                        .split(",")
                        .map(|v| Value::uint(v.parse::<u64>().unwrap()))
                        .collect::<Vec<Value>>();
                    let v = batch_inputs.remove(0);
                    result.inputs.insert("value", v);
                    runbook
                        .command_inputs_evaluation_results
                        .insert(construct_uuid.clone(), result);
                } else {
                    result
                        .inputs
                        .insert("value", Value::uint(input_value.parse::<u64>().unwrap()));
                    runbook
                        .command_inputs_evaluation_results
                        .insert(construct_uuid.clone(), result);
                }
            }
        }
    }

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
    let start_web_ui = cmd.web_console || cmd.port.is_some();
    let is_execution_interactive = start_web_ui || cmd.term_console;
    let runbook_description = runbook.description.clone();
    let moved_block_tx = block_tx.clone();
    let (progress_tx, progress_rx) = txtx_core::kit::channel::unbounded();
    // Start runloop

    // should not be generating actions
    if !is_execution_interactive {
        let _ = hiro_system_kit::thread_named("Display background tasks logs").spawn(move || {
            while let Ok(msg) = progress_rx.recv() {
                match msg {
                    BlockEvent::UpdateProgressBarStatus(update) => {
                        if update.new_status.message.len() == 64 {
                            // Hacky - update message in place
                            print!("\r{} 0x{}", yellow!(format!("{}", update.new_status.status)), update.new_status.message);
                        } else {
                            println!("{}", update.new_status.message)
                        }
                    }
                    _ => {}
                }
            }
        });

        let res = start_runbook_runloop(&mut runbook, &mut runtime_context, &progress_tx).await;
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

    let web_ui_handle = if start_web_ui {
        // start web ui server
        let gql_context = GqlContext {
            protocol_name: runbook_name.clone(),
            runbook_name: runbook_name.clone(),
            runbook_description,
            block_store: block_store.clone(),
            block_broadcaster: block_broadcaster.clone(),
            action_item_events_tx: action_item_events_tx.clone(),
        };

        let port = cmd.port.unwrap_or(DEFAULT_PORT_TXTX);
        println!(
            "\n{} Running Web console\n{}",
            purple!("→"),
            green!(format!("http://localhost:{}", port))
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
            if let Ok(mut block_event) = block_rx.recv() {
                let mut block_store = block_store.write().await;
                let mut do_propagate_event = true;
                match block_event.clone() {
                    BlockEvent::Action(new_block) => {
                        let len = block_store.len();
                        block_store.insert(len, new_block.clone());
                    }
                    BlockEvent::Clear => {
                        *block_store = BTreeMap::new();
                    }
                    BlockEvent::UpdateActionItems(updates) => {
                        // for action item updates, track if we actually changed anything before propagating the event
                        do_propagate_event = false;
                        let mut filtered_updates = vec![];
                        for update in updates.iter() {
                            for (_, block) in block_store.iter_mut() {
                                let did_update = block.apply_action_item_updates(update.clone());
                                if did_update {
                                    do_propagate_event = true;
                                    filtered_updates.push(update.clone());
                                }
                            }
                        }
                        block_event = BlockEvent::UpdateActionItems(filtered_updates);
                    }
                    BlockEvent::Modal(new_block) => {
                        let len = block_store.len();
                        block_store.insert(len, new_block.clone());
                    }
                    BlockEvent::ProgressBar(new_block) => {
                        let len = block_store.len();
                        block_store.insert(len, new_block.clone());
                    }
                    BlockEvent::UpdateProgressBarStatus(update) => block_store
                        .iter_mut()
                        .filter(|(_, b)| b.uuid == update.progress_bar_uuid)
                        .for_each(|(_, b)| {
                            b.update_progress_bar_status(&update.construct_uuid, &update.new_status)
                        }),
                    BlockEvent::UpdateProgressBarVisibility(update) => block_store
                        .iter_mut()
                        .filter(|(_, b)| b.uuid == update.progress_bar_uuid)
                        .for_each(|(_, b)| b.visible = update.visible),
                    BlockEvent::RunbookCompleted => {
                        println!("Runbook completed!")
                    }
                    BlockEvent::Error(new_block) => {
                        let len = block_store.len();
                        block_store.insert(len, new_block.clone());
                    }
                    BlockEvent::Exit => break,
                }
                // only propagate the event if there are actually changes to the block store
                if do_propagate_event {
                    let _ = block_broadcaster.send(block_event.clone());
                }
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
        if let Err(_e) = kill_loops_tx.send(true) {
            std::process::exit(1);
        }
    })
    .expect("Error setting Ctrl-C handler");

    Ok(())
}

pub async fn load_runbooks_from_manifest(
    manifest_path: &str,
) -> Result<(ProtocolManifest, HashMap<String, (Runbook, RuntimeContext)>), String> {
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

        println!("{} '{}' successfully checked", green!("✓"), runbook_name);
    }
    Ok((manifest, runbooks))
}

pub async fn load_runbook_from_manifest(
    manifest_path: &str,
    desired_runbook_name: &str,
) -> Result<(ProtocolManifest, String, Runbook, RuntimeContext), String> {
    let (manifest, runbooks) = load_runbooks_from_manifest(manifest_path).await?;
    // Select first runbook by default
    for (runbook_name, (runbook, runtime_context)) in runbooks.into_iter() {
        if runbook_name.eq(desired_runbook_name) {
            return Ok((manifest, runbook_name, runbook, runtime_context));
        }
    }

    Err(format!(
        "unable to retrieve runbook '{}' in manifst",
        desired_runbook_name
    ))
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

    println!("{} '{}' successfully checked", green!("✓"), runbook_name);

    // Select first runbook by default
    Ok((runbook_name, runbook, runtime_context))
}
