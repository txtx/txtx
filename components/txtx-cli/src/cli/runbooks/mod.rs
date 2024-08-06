use ascii_table::AsciiTable;
use console::Style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use itertools::Itertools;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    env,
    fs::{self, File},
    io::Write,
    path::PathBuf,
    sync::Arc,
};
use tokio::sync::RwLock;
use txtx_addon_network_evm::EVMNetworkAddon;
use txtx_addon_network_stacks::StacksNetworkAddon;
use txtx_addon_telegram::TelegramAddon;
use txtx_core::{
    kit::{
        channel::{self, unbounded},
        hcl::{structure::Block, Ident},
        helpers::fs::FileLocation,
        indexmap::IndexMap,
        types::{
            commands::{CommandId, CommandInputsEvaluationResult},
            diagnostics::Diagnostic,
            frontend::{
                ActionItemRequest, ActionItemRequestType, ActionItemResponse, BlockEvent,
                ProgressBarStatusColor,
            },
            types::Value,
            AuthorizationContext, Did, PackageId, ValueStore,
        },
        Addon, AddonDefaults,
    },
    manifest::{
        file::{read_runbook_from_location, read_runbooks_from_manifest},
        RunbookMetadata, RunbookState, WorkspaceManifest,
    },
    runbook::{
        self, AddonConstructFactory, ConsolidatedChanges, RunbookExecutionMode,
        RunbookExecutionSnapshot, RunbookInputsMap, SynthesizedChange,
    },
    start_supervised_runbook_runloop, start_unsupervised_runbook_runloop,
    types::{ConstructDid, Runbook, RunbookSnapshotContext, RunbookSources},
};

use crate::{
    cli::templates::{build_manifest_data, build_runbook_data},
    web_ui::{
        self,
        cloud_relayer::{start_relayer_event_runloop, RelayerChannelEvent},
    },
};
use txtx_gql::Context as GqlContext;
use web_ui::cloud_relayer::RelayerContext;

pub const DEFAULT_BINDING_PORT: &str = "8488";
pub const DEFAULT_BINDING_ADDRESS: &str = "0.0.0.0";

use super::{CheckRunbook, Context, CreateRunbook, ExecuteRunbook, ListRunbooks};

pub fn load_runbook_execution_snapshot(
    state_file_location: &FileLocation,
) -> Result<RunbookExecutionSnapshot, String> {
    let snapshot_bytes = state_file_location.read_content()?;
    if snapshot_bytes.is_empty() {
        return Err(format!(
            "unable to read {}: file empty",
            state_file_location
        ));
    }
    let snapshot: RunbookExecutionSnapshot = serde_json::from_slice(&snapshot_bytes)
        .map_err(|e| format!("unable to read {}: {}", state_file_location, e.to_string()))?;
    Ok(snapshot)
}

pub fn display_snapshot_diffing(
    consolidated_changes: ConsolidatedChanges,
) -> Option<ConsolidatedChanges> {
    let synthesized_changes = consolidated_changes.get_synthesized_changes();

    if synthesized_changes.is_empty() && consolidated_changes.new_plans_to_add.is_empty() {
        println!(
            "{} Latest snapshot in sync with latest runbook updates",
            green!("✓")
        );
        return Some(consolidated_changes);
    }

    if !consolidated_changes.new_plans_to_add.is_empty() {
        println!("\n{}", yellow!("New chain to synchronize:"));
        println!("{}\n", consolidated_changes.new_plans_to_add.join(", "));
    }

    let has_critical_changes = synthesized_changes.iter().filter(|(c, _)| match c { SynthesizedChange::Edition(_, _) => true, _ => false }).count();
    if has_critical_changes > 0 {
        println!("\n{}\n", yellow!("Changes detected:"));
        for (i, (change, _impacted)) in synthesized_changes
            .into_iter()
            .enumerate()
        {
            match change {
                SynthesizedChange::Edition(change, _) => {
                    let formatted_change = change
                        .iter()
                        .map(|c| {
                            if c.starts_with("-") {
                                red!(c)
                            } else {
                                green!(c)
                            }
                        })
                        .join("");
                    println!("{}. The following edits:\n-------------------------\n{}\r-------------------------", i + 1, formatted_change);
                    println!("\rwill introduce consensus breaking changes.\n\n");
                }
                SynthesizedChange::Addition(_new_construct_did) => {}
            }
        }
    }
    Some(consolidated_changes)
}

pub async fn handle_check_command(
    cmd: &CheckRunbook,
    buffer_stdin: Option<String>,
    _ctx: &Context,
) -> Result<(), String> {
    let available_addons: Vec<Box<dyn Addon>> = vec![
        Box::new(StacksNetworkAddon::new()),
        Box::new(EVMNetworkAddon::new()),
        Box::new(TelegramAddon::new()),
    ];

    let (_manifest, _runbook_name, mut runbook, runbook_state) = load_runbook_from_manifest(
        &cmd.manifest_path,
        &cmd.runbook,
        &cmd.environment,
        &cmd.inputs,
        available_addons,
        buffer_stdin,
    )
    .await?;

    match &runbook_state {
        Some(RunbookState::File(state_file_location)) => {
            let ctx = RunbookSnapshotContext::new();
            let old = load_runbook_execution_snapshot(state_file_location)?;
            for run in runbook.running_contexts.iter_mut() {
                let frontier = HashSet::new();
                let _res = run
                    .execution_context
                    .simulate_execution(
                        &runbook.runtime_context,
                        &run.workspace_context,
                        &runbook.supervision_context,
                        &frontier,
                    )
                    .await;
            }
            runbook.enable_full_execution_mode();
            let new = ctx.snapshot_runbook_execution(
                &runbook.runbook_id,
                &runbook.running_contexts,
                None,
            );

            let consolidated_changes = ctx.diff(old, new);

            display_snapshot_diffing(consolidated_changes);
        }
        None => {}
    }
    Ok(())
}

pub async fn handle_new_command(cmd: &CreateRunbook, _ctx: &Context) -> Result<(), String> {
    let manifest_location = FileLocation::from_path_string(&cmd.manifest_path)?;
    let manifest_res = WorkspaceManifest::from_location(&manifest_location);

    let theme = ColorfulTheme {
        values_style: Style::new().green(),
        ..ColorfulTheme::default()
    };

    let mut manifest = match manifest_res {
        Ok(manifest) => manifest,
        Err(_) => {
            // Ask for the name of the workspace
            let name: String = Input::new()
                .with_prompt("Enter the name of this workspace")
                .interact_text()
                .unwrap();
            WorkspaceManifest::new(name)
        }
    };

    // Choose between deploy, operate, pause, and other
    let choices = vec![
        "Maintenance: update settings, authorize new contracts, etc.",
        "Emergencies: pause contracts, authorization rotations, etc.",
        "Deployments: deploy contracts, upgrade contracts, etc.",
        "Other",
    ];
    let folders = vec!["maintenance", "emergencies", "deployments", "other"];
    let choice = Select::with_theme(&theme)
        .with_prompt("Choose a Runbook type:")
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
    let runbook_name: String = Input::with_theme(&theme)
        .with_prompt("Enter a name for the runbook (e.g., 'BNS Multisig')")
        .interact_text()
        .unwrap();

    // Provide a description (optional)
    let runbook_description: String = Input::with_theme(&theme)
        .with_prompt("Enter the description for the runbook (optional)")
        .allow_empty(true)
        .interact_text()
        .unwrap();

    let runbook = RunbookMetadata::new(
        &action,
        &runbook_name,
        if runbook_description.eq("") {
            None
        } else {
            Some(runbook_description)
        },
    );
    let runbook_id = &runbook.id.clone();
    manifest.runbooks.push(runbook);

    // Initialize root location
    let root_location_path: PathBuf = env::current_dir().expect("Failed to get current directory");
    let root_location = FileLocation::from_path(root_location_path.clone());

    let mut runbook_file_path = root_location_path.clone();
    runbook_file_path.push("runbooks");

    let manifest_location = if let Some(location) = manifest.location.clone() {
        location
    } else {
        let manifest_name = "txtx.yml";
        let mut manifest_location = root_location.clone();
        let _ = manifest_location.append_path(manifest_name);
        let _ = File::create(manifest_location.to_string()).expect("creation failed");
        println!("{} {}", green!("Created manifest"), manifest_name);
        manifest_location
    };
    let mut manifest_file = File::create(manifest_location.to_string()).expect("creation failed");

    let manifest_file_data = build_manifest_data(&manifest);
    let template = mustache::compile_str(include_str!("../templates/txtx.yml.mst"))
        .expect("Failed to compile template");
    template
        .render_data(&mut manifest_file, &manifest_file_data)
        .expect("Failed to render template");

    // Create runbooks directory
    match std::path::Path::exists(&runbook_file_path) {
        true => {}
        false => {
            fs::create_dir_all(&runbook_file_path).map_err(|e| {
                format!(
                    "unable to create parent directory {}\n{}",
                    runbook_file_path.display(),
                    e
                )
            })?;
            println!("{} runbooks", green!("Created directory"));
        }
    }

    let mut readme_file_path = runbook_file_path.clone();
    readme_file_path.push("README.md");
    match std::path::Path::exists(&readme_file_path) {
        true => {}
        false => {
            let mut readme_file = File::create(readme_file_path).expect("creation failed");
            let readme_file_data = build_manifest_data(&manifest);
            let template = mustache::compile_str(include_str!("../templates/readme.md.mst"))
                .expect("Failed to compile template");
            template
                .render_data(&mut readme_file, &readme_file_data)
                .expect("Failed to render template");
            println!("{} runbooks/README.md", green!("Created file"));
        }
    }

    // Create runbooks subdirectory
    runbook_file_path.push(action);
    match std::path::Path::exists(&runbook_file_path) {
        true => {}
        false => {
            fs::create_dir_all(&runbook_file_path.clone()).map_err(|e| {
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
        }
    }

    // Create runbook
    runbook_file_path.push(format!("{}.tx", runbook_id));

    match std::path::Path::exists(&runbook_file_path) {
        true => {
            return Err(format!(
            "file {} already exists. choose a different runbook name, or rename the existing file",
            runbook_file_path.to_str().unwrap()
        ))
        }
        false => {
            let mut runbook_file =
                File::create(runbook_file_path.clone()).expect("creation failed");
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
        }
    }
    Ok(())
}

pub async fn handle_list_command(cmd: &ListRunbooks, _ctx: &Context) -> Result<(), String> {
    let manifest_location = FileLocation::from_path_string(&cmd.manifest_path)?;
    let manifest = WorkspaceManifest::from_location(&manifest_location)?;
    if manifest.runbooks.is_empty() {
        println!("{}: no runbooks referenced in the txtx.yml manifest.\nRun the command `txtx new` to create a new runbook.", yellow!("warning"));
        std::process::exit(1);
    }
    println!("{:<35}\t{:<35}\t{}", "ID", "Name", yellow!("Description"));
    for runbook in manifest.runbooks {
        println!(
            "{:<35}\t{:<35}\t{}",
            runbook.id,
            runbook.name,
            yellow!(format!("{}", runbook.description.unwrap_or("".into())))
        );
    }
    Ok(())
}

pub async fn run_action(
    addon: &Box<dyn Addon>,
    command_name: &str,
    namespace: &str,
    raw_inputs: &Vec<String>,
) -> Result<(), Diagnostic> {
    let factory = AddonConstructFactory {
        functions: addon.build_function_lookup(),
        commands: addon.build_command_lookup(),
        signing_commands: addon.build_wallet_lookup(),
    };
    let block = Block::new(Ident::new("action"));
    let construct_did = Did::zero();
    let command_id = CommandId::Action(command_name.into());
    let package_id = PackageId::zero();
    let addon_defaults = AddonDefaults::new(command_name);
    let (tx, _rx) = unbounded();
    let command = factory
        .create_command_instance(&command_id, namespace, command_name, &block, &package_id)
        .unwrap();

    let mut inputs = ValueStore::new("inputs", &construct_did);

    for input in raw_inputs.iter() {
        let Some((input_name, input_value)) = input.split_once("=") else {
            return Err(Diagnostic::error_from_string(format!(
                "expected --input argument to be formatted as '{}', got '{}'",
                "key=value", input
            )));
        };
        let new_value = Value::parse_and_default_to_string(&input_value);
        inputs.insert(input_name, new_value);
    }
    let evaluated_inputs = CommandInputsEvaluationResult { inputs };

    let _res = command
        .perform_execution(
            &ConstructDid(construct_did),
            &evaluated_inputs,
            addon_defaults,
            &mut vec![],
            &None,
            &tx,
        )
        .await?;
    Ok(())
}

pub async fn handle_run_command(
    cmd: &ExecuteRunbook,
    buffer_stdin: Option<String>,
    ctx: &Context,
) -> Result<(), String> {
    let is_execution_unsupervised = cmd.unsupervised;
    let do_use_term_console = cmd.term_console;
    let start_web_ui = cmd.web_console || (!is_execution_unsupervised && !do_use_term_console);

    let available_addons: Vec<Box<dyn Addon>> = vec![
        Box::new(StacksNetworkAddon::new()),
        Box::new(EVMNetworkAddon::new()),
        Box::new(TelegramAddon::new()),
    ];

    if let Some((namespace, command_name)) = cmd.runbook.split_once("::") {
        for addon in available_addons.iter() {
            if namespace.starts_with(&format!("{}", addon.get_namespace())) {
                // Execute command
                run_action(addon, command_name, namespace, &cmd.inputs)
                    .await
                    .map_err(|e| e.to_string())?;
                return Ok(());
            }
        }
    }

    let (progress_tx, progress_rx) = txtx_core::kit::channel::unbounded();
    let res = load_runbook_from_manifest(
        &cmd.manifest_path,
        &cmd.runbook,
        &cmd.environment,
        &cmd.inputs,
        available_addons,
        buffer_stdin.clone(),
    )
    .await;
    let (runbook_name, mut runbook, runbook_state) = match res {
        Ok((_manifest, runbook_name, runbook, runbook_state)) => {
            (runbook_name, runbook, runbook_state)
        }
        Err(_) => {
            let (runbook_name, runbook) =
                load_runbook_from_file_path(&cmd.runbook, &cmd.inputs, buffer_stdin).await?;
            (runbook_name, runbook, None)
        }
    };

    let previous_state_opt =
        if let Some(RunbookState::File(state_file_location)) = runbook_state.clone() {
            match load_runbook_execution_snapshot(&state_file_location) {
                Ok(snapshot) => Some(snapshot),
                Err(e) => {
                    println!("{} {}", red!("x"), e);
                    None
                }
            }
        } else {
            None
        };

    runbook.enable_full_execution_mode();

    if !cmd.force_execution {
        if let Some(old) = previous_state_opt {
            // if !cmd.force_execution {
            let ctx = RunbookSnapshotContext::new();

            let mut execution_context_backups = HashMap::new();

            for run in runbook.running_contexts.iter_mut() {
                let frontier = HashSet::new();
                let execution_context_backup = run.execution_context.clone();
                let _res = run
                    .execution_context
                    .simulate_execution(
                        &runbook.runtime_context,
                        &run.workspace_context,
                        &runbook.supervision_context,
                        &frontier,
                    )
                    .await;
                execution_context_backups
                    .insert(run.inputs_set.name.clone(), execution_context_backup);
            }

            let new = ctx.snapshot_runbook_execution(
                &runbook.runbook_id,
                &runbook.running_contexts,
                None,
            );

            let consolidated_changes = ctx.diff(old, new);

            let Some(consolidated_changes) = display_snapshot_diffing(consolidated_changes) else {
                return Ok(());
            };

            let theme = ColorfulTheme {
                values_style: Style::new().green(),
                ..ColorfulTheme::default()
            };

            let mut actions_to_re_execute = IndexMap::new();
            let mut actions_to_execute = IndexMap::new();

            for (running_context_key, changes) in consolidated_changes.plans_to_update.iter() {

                let critical_edits = changes
                    .contructs_to_update
                    .iter()
                    .filter(|c| !c.description.is_empty() && c.critical)
                    .collect::<Vec<_>>();

                let additions = changes.new_constructs_to_add.iter().collect::<Vec<_>>();

                let running_context =
                    runbook.find_expected_running_context_mut(&running_context_key);

                let pristine_execution_context = execution_context_backups
                    .remove(running_context_key)
                    .unwrap();

                if consolidated_changes
                    .new_plans_to_add
                    .contains(running_context_key)
                {
                    // Restore a pristing execution context
                    running_context.execution_context.execution_mode = RunbookExecutionMode::Full;
                    running_context.execution_context = pristine_execution_context;
                    continue;
                }

                if critical_edits.is_empty() && additions.is_empty() {
                    running_context.execution_context.execution_mode =
                        RunbookExecutionMode::Ignored;
                    continue;
                }

                running_context.execution_context.execution_mode =
                    RunbookExecutionMode::Partial(vec![]);

                let descendants_of_critically_changed_commands = critical_edits
                    .iter()
                    .filter_map(|c| {
                        if let Some(construct_did) = &c.construct_did {
                            let mut segment = vec![];
                            segment.push(construct_did.clone());
                            let mut deps = running_context
                                .graph_context
                                .get_downstream_dependencies_for_construct_did(
                                    &construct_did,
                                    true,
                                );
                            segment.append(&mut deps);
                            Some(segment)
                        } else {
                            None
                        }
                    })
                    .flatten()
                    .collect::<Vec<_>>();

                let actions: Vec<(String, Option<String>)> =
                    descendants_of_critically_changed_commands
                        .iter()
                        .map(|construct_did| {
                            let documentation = running_context
                                .execution_context
                                .commands_inputs_simulation_results
                                .get(construct_did)
                                .and_then(|r| r.inputs.get_string("description"))
                                .and_then(|d| Some(d.to_string()));
                            let command = running_context
                                .execution_context
                                .commands_instances
                                .get(construct_did)
                                .unwrap();
                            (command.name.to_string(), documentation)
                        })
                        .collect();
                actions_to_re_execute.insert(running_context_key, actions);

                let mut descendants_of_added_commands = additions
                    .iter()
                    .map(|(construct_did, _)| {
                        let mut segment = vec![];
                        segment.push(construct_did.clone());
                        let mut deps = running_context
                            .graph_context
                            .get_downstream_dependencies_for_construct_did(&construct_did, true);
                        segment.append(&mut deps);
                        segment
                    })
                    .flatten()
                    .collect::<Vec<_>>();

                let added_actions: Vec<(String, Option<String>)> = descendants_of_added_commands
                    .iter()
                    .map(|construct_did| {
                        let documentation = running_context
                            .execution_context
                            .commands_inputs_simulation_results
                            .get(construct_did)
                            .and_then(|r| r.inputs.get_string("description"))
                            .and_then(|d| Some(d.to_string()));
                        let command = running_context
                            .execution_context
                            .commands_instances
                            .get(construct_did)
                            .unwrap();
                        (command.name.to_string(), documentation)
                    })
                    .collect();
                actions_to_execute.insert(running_context_key, added_actions);

                let mut great_filter = descendants_of_critically_changed_commands;
                great_filter.append(&mut descendants_of_added_commands);

                running_context
                    .execution_context
                    .order_for_commands_execution = running_context
                    .execution_context
                    .order_for_commands_execution
                    .clone()
                    .into_iter()
                    .filter(|c| great_filter.contains(&c))
                    .collect();
            }

            let has_actions = actions_to_re_execute.iter().filter(|(_, actions)| !actions.is_empty()).count();
            if has_actions > 0 {
                println!("The following actions will be re-executed:");
                for (context, actions) in actions_to_re_execute.iter() {
                    let documentation_missing = black!("<description field empty>");
                    println!("\n{}", yellow!(format!("{}", context)));
                    for (action_name, documentation) in actions.into_iter() {
                        println!(
                            "- {}: {}",
                            action_name,
                            documentation.as_ref().unwrap_or(&documentation_missing)
                        );
                    }
                }
                println!("\n");
            }

            let has_actions = actions_to_execute.iter().filter(|(_, actions)| !actions.is_empty()).count();
            if has_actions > 0 {
                println!("The following actions will be executed:");
                for (context, actions) in actions_to_execute.iter() {
                    let documentation_missing = black!("<description field empty>");
                    println!("\n{}", green!(format!("{}", context)));
                    for (action_name, documentation) in actions.into_iter() {
                        println!(
                            "- {}: {}",
                            action_name,
                            documentation.as_ref().unwrap_or(&documentation_missing)
                        );
                    }
                }
                println!("\n");
            }

            let confirm = Confirm::with_theme(&theme)
                .with_prompt("Do you want to continue?")
                .interact()
                .unwrap();

            if !confirm {
                return Ok(());
            }
        }
    } else {
        println!(
            "{} Executing Runbook with 'force' flag - ignoring previous execution state",
            yellow!("→"),
        );
    }

    // should not be generating actions
    if is_execution_unsupervised {
        let _ = hiro_system_kit::thread_named("Display background tasks logs").spawn(move || {
            while let Ok(msg) = progress_rx.recv() {
                match msg {
                    BlockEvent::UpdateProgressBarStatus(update) => {
                        match update.new_status.status_color {
                            ProgressBarStatusColor::Yellow => {
                                print!(
                                    "\r{} {} {:<100}",
                                    yellow!("→"),
                                    yellow!(format!("{}", update.new_status.status)),
                                    update.new_status.message,
                                );
                            }
                            ProgressBarStatusColor::Green => {
                                print!(
                                    "\r{} {} {:<100}\n",
                                    green!("✓"),
                                    green!(format!("{}", update.new_status.status)),
                                    update.new_status.message,
                                );
                            }
                            ProgressBarStatusColor::Red => {
                                print!(
                                    "\r{} {} {:<100}\n",
                                    red!("x"),
                                    red!(format!("{}", update.new_status.status)),
                                    update.new_status.message,
                                );
                            }
                        };
                        std::io::stdout().flush().unwrap();
                    }
                    _ => {}
                }
            }
        });

        println!(
            "{} Starting runbook '{}' execution in unsupervised mode",
            purple!("→"),
            runbook_name
        );

        let res = start_unsupervised_runbook_runloop(&mut runbook, &progress_tx).await;
        if let Err(diags) = res {
            println!("{} Execution aborted", red!("x"));
            for diag in diags.iter() {
                println!("{}", red!(format!("- {}", diag)));
            }
            return Ok(());
        }

        let ascii_table = AsciiTable::default();
        let mut collected_outputs: IndexMap<String, Vec<String>> = IndexMap::new();
        for running_context in runbook.running_contexts.iter() {
            let grouped_actions_items = running_context
                .execution_context
                .collect_outputs_constructs_results();

            for (_, items) in grouped_actions_items.iter() {
                for item in items.iter() {
                    if let ActionItemRequestType::DisplayOutput(ref output) = item.action_type {
                        match collected_outputs.get_mut(&output.name) {
                            Some(entries) => {
                                entries.push(output.value.to_string());
                            }
                            None => {
                                collected_outputs.insert(
                                    output.name.to_string(),
                                    vec![output.value.to_string()],
                                );
                            }
                        }
                    }
                }
            }
        }

        if !collected_outputs.is_empty() {
            let mut data = vec![];
            for (key, mut values) in collected_outputs.drain(..) {
                let mut row = vec![key];
                row.append(&mut values);
                data.push(row)
            }
            ascii_table.print(data);
        }

        if let Some(RunbookState::File(state_file_location)) = runbook_state {
            let previous_snapshot = match load_runbook_execution_snapshot(&state_file_location) {
                Ok(snapshot) => Some(snapshot),
                Err(_e) => None,
            };

            println!(
                "{} Saving execution state to {}",
                green!("✓"),
                state_file_location
            );
            let diff = RunbookSnapshotContext::new();
            let snapshot = diff.snapshot_runbook_execution(
                &runbook.runbook_id,
                &runbook.running_contexts,
                previous_snapshot,
            );
            state_file_location
                .write_content(serde_json::to_string_pretty(&snapshot).unwrap().as_bytes())
                .expect("unable to save state");
            ();
        }
        return Ok(());
    }

    let runbook_description = runbook.description.clone();
    let (block_tx, block_rx) = channel::unbounded::<BlockEvent>();
    let (block_broadcaster, _) = tokio::sync::broadcast::channel(5);
    let (action_item_updates_tx, _action_item_updates_rx) =
        channel::unbounded::<ActionItemRequest>();
    let (action_item_events_tx, action_item_events_rx) = channel::unbounded::<ActionItemResponse>();
    let block_store = Arc::new(RwLock::new(BTreeMap::new()));
    let (kill_loops_tx, kill_loops_rx) = channel::bounded(1);
    let (relayer_channel_tx, relayer_channel_rx) = channel::unbounded();

    let moved_block_tx = block_tx.clone();
    let moved_kill_loops_tx = kill_loops_tx.clone();
    let _ = hiro_system_kit::thread_named("Runbook Runloop").spawn(move || {
        let runloop_future = start_supervised_runbook_runloop(
            &mut runbook,
            moved_block_tx,
            action_item_updates_tx,
            action_item_events_rx,
        );
        if let Err(diags) = hiro_system_kit::nestable_block_on(runloop_future) {
            for diag in diags.iter() {
                println!("{} {}", red!("x"), diag);
            }
        }
        if let Err(_e) = moved_kill_loops_tx.send(true) {
            std::process::exit(1);
        }
    });

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

        let channel_data = Arc::new(RwLock::new(None));
        let relayer_context = RelayerContext {
            relayer_channel_tx: relayer_channel_tx.clone(),
            channel_data: channel_data.clone(),
        };

        let network_binding = format!(
            "{}:{}",
            cmd.network_binding_ip_address, cmd.network_binding_port
        );
        println!(
            "\n{} Starting the supervisor web console\n{}",
            purple!("→"),
            green!(format!("http://{}", network_binding))
        );

        let handle =
            web_ui::http::start_server(gql_context, relayer_context, &network_binding, ctx)
                .await
                .map_err(|e| format!("Failed to start web ui: {e}"))?;

        let moved_relayer_channel_tx = relayer_channel_tx.clone();
        let moved_kill_loops_tx = kill_loops_tx.clone();
        let moved_action_item_events_tx = action_item_events_tx.clone();
        let _ = hiro_system_kit::thread_named("Relayer Interaction").spawn(move || {
            let future = start_relayer_event_runloop(
                channel_data,
                relayer_channel_rx,
                moved_relayer_channel_tx,
                moved_action_item_events_tx,
                moved_kill_loops_tx,
            );
            hiro_system_kit::nestable_block_on(future)
        });

        Some(handle)
    } else {
        None
    };

    let moved_relayer_channel_tx = relayer_channel_tx.clone();
    let block_store_handle = tokio::spawn(async move {
        loop {
            if let Ok(mut block_event) = block_rx.try_recv() {
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
                            b.update_progress_bar_status(&update.construct_did, &update.new_status)
                        }),
                    BlockEvent::UpdateProgressBarVisibility(update) => block_store
                        .iter_mut()
                        .filter(|(_, b)| b.uuid == update.progress_bar_uuid)
                        .for_each(|(_, b)| b.visible = update.visible),
                    BlockEvent::RunbookCompleted => {
                        println!("{}", green!("Runbook complete!"));
                    }
                    BlockEvent::Error(new_block) => {
                        let len = block_store.len();
                        block_store.insert(len, new_block.clone());
                    }
                    BlockEvent::Exit => break,
                }

                if do_propagate_event {
                    let _ = block_broadcaster.send(block_event.clone());
                    let _ = moved_relayer_channel_tx.send(
                        RelayerChannelEvent::ForwardEventToRelayer(block_event.clone()),
                    );
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            // println!("waiting for next block event");
        }
    });

    let _ = hiro_system_kit::thread_named("Kill Runloops Thread")
        .spawn(move || {
            let future = async {
                match kill_loops_rx.recv() {
                    Ok(_) => {
                        if let Some(handle) = web_ui_handle {
                            let _ = handle.stop(true).await;
                        }
                        let _ = relayer_channel_tx.send(RelayerChannelEvent::Exit);
                        let _ = block_tx.send(BlockEvent::Exit);
                    }
                    Err(_) => {}
                };
            };

            hiro_system_kit::nestable_block_on(future)
        })
        .unwrap();

    ctrlc::set_handler(move || {
        if let Err(_e) = kill_loops_tx.send(true) {
            std::process::exit(1);
        }
    })
    .expect("Error setting Ctrl-C handler");
    let _ = tokio::join!(block_store_handle);
    Ok(())
}

pub async fn load_runbooks_from_manifest(
    manifest_path: &str,
) -> Result<
    (
        WorkspaceManifest,
        IndexMap<String, (Runbook, RunbookSources, String, Option<RunbookState>)>,
    ),
    String,
> {
    let manifest_location = FileLocation::from_path_string(manifest_path)?;
    let manifest = WorkspaceManifest::from_location(&manifest_location)?;
    let runbooks = read_runbooks_from_manifest(&manifest, None)?;
    println!("\n{} Processing manifest '{}'", purple!("→"), manifest_path);
    Ok((manifest, runbooks))
}

pub async fn load_runbook_from_manifest(
    manifest_path: &str,
    desired_runbook_name: &str,
    environment_selector: &Option<String>,
    cli_inputs: &Vec<String>,
    available_addons: Vec<Box<dyn Addon>>,
    buffer_stdin: Option<String>,
) -> Result<(WorkspaceManifest, String, Runbook, Option<RunbookState>), String> {
    let (manifest, runbooks) = load_runbooks_from_manifest(manifest_path).await?;
    // Select first runbook by default
    for (runbook_id, (mut runbook, runbook_sources, runbook_name, runbook_state)) in
        runbooks.into_iter()
    {
        if runbook_name.eq(desired_runbook_name) || runbook_id.eq(desired_runbook_name) {
            let inputs_map =
                manifest.get_runbook_inputs(environment_selector, cli_inputs, buffer_stdin)?;
            let authorization_context =
                AuthorizationContext::new(manifest.location.clone().unwrap());
            let res = runbook.build_contexts_from_sources(
                runbook_sources,
                inputs_map,
                authorization_context,
                available_addons,
            );
            if let Err(diags) = res {
                for diag in diags.iter() {
                    println!("{} {}", red!("x"), diag);
                }
                std::process::exit(1);
            }
            return Ok((manifest, runbook_name, runbook, runbook_state));
        }
    }
    Err(format!(
        "unable to retrieve runbook '{}' in manifest",
        desired_runbook_name
    ))
}

pub async fn load_runbook_from_file_path(
    file_path: &str,
    cli_inputs: &Vec<String>,
    buffer_stdin: Option<String>,
) -> Result<(String, Runbook), String> {
    let location = FileLocation::from_path_string(file_path)?;
    let (runbook_name, mut runbook, runbook_sources) =
        read_runbook_from_location(&location, &None)?;

    println!("\n{} Processing file '{}'", purple!("→"), file_path);
    let mut inputs_map = RunbookInputsMap::new();
    inputs_map.override_values_with_cli_inputs(cli_inputs, buffer_stdin)?;
    let available_addons: Vec<Box<dyn Addon>> = vec![
        Box::new(StacksNetworkAddon::new()),
        Box::new(EVMNetworkAddon::new()),
        Box::new(TelegramAddon::new()),
    ];
    let authorization_context = AuthorizationContext::new(location);
    let res = runbook.build_contexts_from_sources(
        runbook_sources,
        inputs_map,
        authorization_context,
        available_addons,
    );
    if let Err(diags) = res {
        for diag in diags.iter() {
            println!("{} {}", red!("x"), diag);
        }
        std::process::exit(1);
    }

    println!("{} '{}' successfully checked", green!("✓"), runbook_name);

    // Select first runbook by default
    Ok((runbook_name, runbook))
}
