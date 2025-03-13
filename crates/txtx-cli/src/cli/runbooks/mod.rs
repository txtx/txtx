use super::{CheckRunbook, Context, CreateRunbook, ExecuteRunbook, ListRunbooks};
use crate::{get_addon_by_namespace, get_available_addons};
use ascii_table::AsciiTable;
use console::Style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use itertools::Itertools;
use serde_json::Value as JsonValue;
use std::{
    collections::{BTreeMap, HashSet},
    env,
    fs::{self, File},
    io::Write,
    path::PathBuf,
    sync::Arc,
};
use tokio::sync::RwLock;
use txtx_core::templates::{build_manifest_data, build_runbook_data};
use txtx_core::{
    kit::types::{commands::UnevaluatedInputsMap, stores::ValueStore},
    mustache,
    runbook::flow_context::FlowContext,
    templates::{TXTX_MANIFEST_TEMPLATE, TXTX_README_TEMPLATE},
};
use txtx_core::{
    kit::{
        channel::{self, unbounded},
        hcl::{structure::Block, Ident},
        helpers::fs::FileLocation,
        indexmap::IndexMap,
        types::{
            commands::{CommandId, CommandInputsEvaluationResult},
            diagnostics::Diagnostic,
            frontend::{ActionItemRequestType, BlockEvent, ProgressBarStatusColor},
            stores::AddonDefaults,
            types::Value,
            AuthorizationContext, Did, PackageId,
        },
        Addon,
    },
    manifest::{
        file::{read_runbook_from_location, read_runbooks_from_manifest},
        RunbookMetadata, RunbookStateLocation, WorkspaceManifest,
    },
    runbook::{
        AddonConstructFactory, ConsolidatedChanges, RunbookTopLevelInputsMap, SynthesizedChange,
    },
    start_supervised_runbook_runloop, start_unsupervised_runbook_runloop,
    types::{ConstructDid, Runbook, RunbookSnapshotContext, RunbookSources},
};

#[cfg(feature = "supervisor_ui")]
use actix_web::dev::ServerHandle;
#[cfg(feature = "ovm")]
use txtx_addon_network_ovm::OvmNetworkAddon;
#[cfg(feature = "sp1")]
use txtx_addon_sp1::Sp1Addon;
#[cfg(feature = "supervisor_ui")]
use txtx_supervisor_ui::{self, cloud_relayer::RelayerChannelEvent};

pub fn display_snapshot_diffing(
    consolidated_changes: ConsolidatedChanges,
) -> Option<ConsolidatedChanges> {
    let synthesized_changes = consolidated_changes.get_synthesized_changes();

    if synthesized_changes.is_empty() && consolidated_changes.new_plans_to_add.is_empty() {
        println!("{} Latest snapshot in sync with latest runbook updates\n", green!("✓"));
        return None;
    }

    if !consolidated_changes.new_plans_to_add.is_empty() {
        println!("\n{}", yellow!("New chain to synchronize:"));
        println!("{}\n", consolidated_changes.new_plans_to_add.join(", "));
    }

    let has_critical_changes = synthesized_changes
        .iter()
        .filter(|(c, _)| match c {
            SynthesizedChange::Edition(_, _) => true,
            SynthesizedChange::FormerFailure(_, _) => false,
            SynthesizedChange::Addition(_) => false,
        })
        .count();
    if has_critical_changes > 0 {
        println!("\n{}\n", yellow!("Changes detected:"));
        for (i, (change, _impacted)) in synthesized_changes.iter().enumerate() {
            match change {
                SynthesizedChange::Edition(change, _) => {
                    let formatted_change = change
                        .iter()
                        .map(|c| if c.starts_with("-") { red!(c) } else { green!(c) })
                        .join("");
                    println!("{}. The following edits:\n-------------------------\n{}\n-------------------------", i + 1, formatted_change);
                    println!("will introduce breaking changes.\n\n");
                }
                SynthesizedChange::FormerFailure(_construct_to_run, command_name) => {
                    println!("{}. The action error:\n-------------------------\n{}\n-------------------------", i + 1, command_name);
                    println!("will be re-executed.\n\n");
                }
                SynthesizedChange::Addition(_new_construct_did) => {}
            }
        }
    }

    let unexecuted = synthesized_changes
        .iter()
        .filter(|(c, _)| match c {
            SynthesizedChange::Edition(_, _) => false,
            SynthesizedChange::FormerFailure(_, _) => true,
            SynthesizedChange::Addition(_) => false,
        })
        .count();
    if unexecuted > 0 {
        println!("\n{}", yellow!("Runbook Recovery Plan"));
        println!("The previous runbook execution was interrupted before completion, causing the following actions to be aborted:");

        for (_i, (change, _impacted)) in synthesized_changes.iter().enumerate() {
            match change {
                SynthesizedChange::Edition(_, _) => {}
                SynthesizedChange::FormerFailure(_construct_to_run, command_name) => {
                    println!("- {}", command_name);
                }
                SynthesizedChange::Addition(_new_construct_did) => {}
            }
        }
        println!("These actions will be re-executed in the next run.\n");
    }

    Some(consolidated_changes)
}

pub async fn handle_check_command(
    cmd: &CheckRunbook,
    buffer_stdin: Option<String>,
    _ctx: &Context,
) -> Result<(), String> {
    let (_manifest, _runbook_name, mut runbook, runbook_state) = load_runbook_from_manifest(
        &cmd.manifest_path,
        &cmd.runbook,
        &cmd.environment,
        &cmd.inputs,
        buffer_stdin,
    )
    .await?;

    match &runbook_state {
        Some(state_file_location) => {
            let ctx = RunbookSnapshotContext::new();
            let old = state_file_location.load_execution_snapshot(
                true,
                &runbook.runbook_id.name,
                &runbook.top_level_inputs_map.current_top_level_input_name(),
            )?;
            for run in runbook.flow_contexts.iter_mut() {
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
            let new = ctx
                .snapshot_runbook_execution(
                    &runbook.runbook_id,
                    &runbook.flow_contexts,
                    None,
                    &runbook.top_level_inputs_map,
                )
                .map_err(|e| e.message)?;

            let consolidated_changes = match ctx.diff(old, new) {
                Ok(changes) => changes,
                Err(e) => {
                    println!("{} Failed to process snapshot: {}", red!("x"), e);
                    return Ok(());
                }
            };

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
        hint_style: Style::new().cyan(),
        ..ColorfulTheme::default()
    };

    let mut manifest = match manifest_res {
        Ok(manifest) => manifest,
        Err(_) => {
            let current_dir = env::current_dir()
                .ok()
                .and_then(|d| d.file_name().map(|f| f.to_string_lossy().to_string()));
            let default = match current_dir {
                Some(dir) => dir,
                _ => "".to_string(),
            };

            // Ask for the name of the workspace
            let name: String = Input::with_theme(&theme)
                .with_prompt("Enter the name of this workspace")
                .default(default)
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
    // todo: validate runbook name
    let runbook_name: String = Input::with_theme(&theme)
        .with_prompt("Enter a name for the runbook (e.g., 'deploy-contract')")
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
        if runbook_description.eq("") { None } else { Some(runbook_description) },
    );
    let runbook_id = &runbook.name.clone();
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
    let template =
        mustache::compile_str(TXTX_MANIFEST_TEMPLATE).expect("Failed to compile template");
    template
        .render_data(&mut manifest_file, &manifest_file_data)
        .expect("Failed to render template");

    // Create runbooks directory
    match std::path::Path::exists(&runbook_file_path) {
        true => {}
        false => {
            fs::create_dir_all(&runbook_file_path).map_err(|e| {
                format!("unable to create parent directory {}\n{}", runbook_file_path.display(), e)
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
            let template =
                mustache::compile_str(TXTX_README_TEMPLATE).expect("Failed to compile template");
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
                format!("unable to create parent directory {}\n{}", runbook_file_path.display(), e)
            })?;
            let runbook_location = FileLocation::from_path(runbook_file_path.clone());
            println!(
                "{} {}",
                green!("Created directory"),
                runbook_location.get_relative_path_from_base(&root_location).unwrap()
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
            let template = mustache::compile_str(txtx_core::templates::TXTX_RUNBOOK_TEMPLATE)
                .expect("Failed to compile template");
            template
                .render_data(&mut runbook_file, &runbook_file_data)
                .expect("Failed to render template");
            let runbook_location = FileLocation::from_path(runbook_file_path);
            println!(
                "{} {}",
                green!("Created runbook"),
                runbook_location.get_relative_path_from_base(&root_location).unwrap()
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
    println!("{:<35}\t{}", "Name", yellow!("Description"));
    for runbook in manifest.runbooks {
        println!(
            "{:<35}\t{}",
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
        signers: addon.build_signer_lookup(),
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

    let mut inputs = ValueStore::new("inputs", &construct_did).with_defaults(&addon_defaults.store);

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
    let unevaluated_inputs = UnevaluatedInputsMap::new();
    let evaluated_inputs = CommandInputsEvaluationResult { inputs, unevaluated_inputs };

    let _res = command
        .perform_execution(
            &ConstructDid(construct_did.clone()),
            &ValueStore::new("inputs", &construct_did), // todo: we need to actually pass the nested evaluation values
            &evaluated_inputs,
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
    _ctx: &Context,
) -> Result<(), String> {
    let is_execution_unsupervised = cmd.unsupervised;

    let available_addons = get_available_addons();
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
        buffer_stdin.clone(),
    )
    .await;
    let (runbook_name, mut runbook, runbook_state_location) = match res {
        Ok((_manifest, runbook_name, runbook, state_file_location)) => {
            (runbook_name, runbook, state_file_location)
        }
        Err(_) => {
            let (runbook_name, runbook) =
                load_runbook_from_file_path(&cmd.runbook, &cmd.inputs, buffer_stdin).await?;
            (runbook_name, runbook, None)
        }
    };

    let previous_state_opt = if let Some(state_file_location) = runbook_state_location.clone() {
        match state_file_location.load_execution_snapshot(
            true,
            &runbook.runbook_id.name,
            &runbook.top_level_inputs_map.current_top_level_input_name(),
        ) {
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
            let ctx = RunbookSnapshotContext::new();

            let execution_context_backups = runbook.backup_execution_contexts();
            let new = runbook.simulate_and_snapshot_flows(&old).await?;

            for flow_context in runbook.flow_contexts.iter() {
                if old.flows.get(&flow_context.name).is_none() {
                    println!(
                        "{} Previous snapshot not found for flow {}",
                        yellow!("!"),
                        flow_context.name
                    );
                };
            }

            let consolidated_changes = match ctx.diff(old, new) {
                Ok(changes) => changes,
                Err(e) => {
                    println!("{} Failed to process snapshot: {}", red!("x"), e);
                    return Ok(());
                }
            };

            let Some(consolidated_changes) = display_snapshot_diffing(consolidated_changes) else {
                return Ok(());
            };

            runbook.prepare_flows_for_new_plans(
                &consolidated_changes.new_plans_to_add,
                execution_context_backups,
            );

            let (actions_to_re_execute, actions_to_execute) =
                runbook.prepared_flows_for_updated_plans(&consolidated_changes.plans_to_update);

            let has_actions =
                actions_to_re_execute.iter().filter(|(_, actions)| !actions.is_empty()).count();
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

            let has_actions =
                actions_to_execute.iter().filter(|(_, actions)| !actions.is_empty()).count();
            if has_actions > 0 {
                println!("The following actions have been added and will be executed for the first time:");
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

            let theme =
                ColorfulTheme { values_style: Style::new().green(), ..ColorfulTheme::default() };

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

    if cmd.explain {
        for (location, _) in runbook.sources.tree.iter() {
            println!("Loading {}", location);
        }
        for running_context in runbook.flow_contexts.iter_mut() {
            // running_context.execution_context.simulate_inputs_execution(&runbook.runtime_context, &running_context.workspace_context);
            let sorted_commands = &running_context.execution_context.order_for_commands_execution;
            for c in sorted_commands.iter() {
                let Some(command_instance) =
                    running_context.execution_context.commands_instances.get(c)
                else {
                    continue;
                };
                println!("{}::{}", command_instance.specification.matcher, command_instance.name);
            }
        }
        // return Ok(());
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
                                    "\r{} {} {:<150}",
                                    yellow!("→"),
                                    yellow!(format!("{}", update.new_status.status)),
                                    update.new_status.message,
                                );
                            }
                            ProgressBarStatusColor::Green => {
                                print!(
                                    "\r{} {} {:<150}\n",
                                    green!("✓"),
                                    green!(format!("{}", update.new_status.status)),
                                    update.new_status.message,
                                );
                            }
                            ProgressBarStatusColor::Red => {
                                print!(
                                    "\r{} {} {:<150}\n",
                                    red!("x"),
                                    red!(format!("{}", update.new_status.status)),
                                    update.new_status.message,
                                );
                            }
                            ProgressBarStatusColor::Purple => {
                                print!(
                                    "\r{} {} {:<150}\n",
                                    purple!("→"),
                                    purple!(format!("{}", update.new_status.status)),
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
            if let Some(location) =
                runbook.mark_failed_and_write_transient_state(runbook_state_location)?
            {
                println!("{} Saving transient state to {}", yellow!("!"), location);
            };
            return Ok(());
        }

        let (mut collected_outputs, json_outputs) =
            collect_formatted_outputs(&runbook.flow_contexts);

        if let Some(location) = runbook.write_runbook_state(runbook_state_location)? {
            println!("\n{} Saved execution state to {}", green!("✓"), location);
        }

        if !collected_outputs.is_empty() {
            if let Some(_) = cmd.output_json.as_ref() {
                try_display_json_output(
                    cmd.output_json.clone(),
                    json_outputs,
                    &runbook.runtime_context.authorization_context.workspace_location,
                    &runbook.runbook_id.name,
                    &runbook.top_level_inputs_map.current_top_level_input_name(),
                )
                .map_err(|e| format!("failed to print runbook outputs: {e}"))?;
            } else {
                for (flow_name, mut flow_outputs) in collected_outputs.drain(..) {
                    if !flow_outputs.is_empty() {
                        println!("{}", yellow!(format!("{} Outputs: ", flow_name)));
                        let mut data = vec![];
                        for (key, values) in flow_outputs.drain(..) {
                            if let Some(ref desired_output) = cmd.output {
                                if desired_output.eq(&key) && !values.is_empty() {
                                    println!("{}", values.first().unwrap());
                                    return Ok(());
                                }
                            }
                            let mut rows = vec![];

                            for (i, value) in values.into_iter().enumerate() {
                                let value = value.to_string();
                                let parts = value.split("\n");
                                for (j, part) in parts.into_iter().enumerate() {
                                    if i == 0 && j == 0 {
                                        rows.push(vec![key.clone(), part.to_string()]);
                                    } else {
                                        let row = vec!["".to_string(), part.to_string()];
                                        rows.push(row);
                                    }
                                }
                            }
                            data.append(&mut rows)
                        }
                        let mut ascii_table = AsciiTable::default();
                        ascii_table.set_max_width(150);
                        ascii_table.print(data);
                    }
                }
            }
        }

        return Ok(());
    }

    let (block_tx, block_rx) = channel::unbounded::<BlockEvent>();
    let (block_broadcaster, _) = tokio::sync::broadcast::channel(5);
    let block_store = Arc::new(RwLock::new(BTreeMap::new()));
    let (kill_loops_tx, kill_loops_rx) = channel::bounded(1);
    let (action_item_events_tx, action_item_events_rx) = tokio::sync::broadcast::channel(32);

    #[cfg(feature = "supervisor_ui")]
    let runbook_description = runbook.description.clone();
    #[cfg(feature = "supervisor_ui")]
    let registered_addons = runbook
        .runtime_context
        .addons_context
        .registered_addons
        .keys()
        .map(|k| k.clone())
        .collect::<Vec<_>>();

    let moved_block_tx = block_tx.clone();
    let moved_kill_loops_tx = kill_loops_tx.clone();
    let moved_runbook_state = runbook_state_location.clone();
    let output_json = cmd.output_json.clone();
    let _ = hiro_system_kit::thread_named("Runbook Runloop").spawn(move || {
        let runloop_future =
            start_supervised_runbook_runloop(&mut runbook, moved_block_tx, action_item_events_rx);
        if let Err(diags) = hiro_system_kit::nestable_block_on(runloop_future) {
            for diag in diags.iter() {
                println!("{} {}", red!("x"), diag);
            }
            match runbook.mark_failed_and_write_transient_state(moved_runbook_state) {
                Ok(Some(location)) => {
                    println!("{} Saving transient state to {}", yellow!("!"), location);
                }
                Ok(None) => {}
                Err(e) => {
                    println!("{} Failed to write transient runbook state: {}", red!("x"), e);
                }
            };
        } else {
            let (_, json_outputs) = collect_formatted_outputs(&runbook.flow_contexts);

            if !json_outputs.is_empty() {
                match try_display_json_output(
                    output_json,
                    json_outputs,
                    &runbook.runtime_context.authorization_context.workspace_location,
                    &runbook.runbook_id.name,
                    &runbook.top_level_inputs_map.current_top_level_input_name(),
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("{} failed to write runbook outputs{}", red!("x"), e);
                    }
                }
            }

            match runbook.write_runbook_state(moved_runbook_state) {
                Ok(Some(location)) => {
                    println!("\n{} Saved execution state to {}", green!("✓"), location);
                }
                Ok(None) => {}
                Err(e) => {
                    println!("{} Failed to write runbook state: {}", red!("x"), e);
                }
            };
        }
        if let Err(_e) = moved_kill_loops_tx.send(true) {
            std::process::exit(1);
        }
    });

    #[cfg(feature = "supervisor_ui")]
    let (relayer_channel_tx, relayer_channel_rx) = channel::unbounded();
    #[cfg(feature = "supervisor_ui")]
    let moved_relayer_channel_tx = relayer_channel_tx.clone();
    #[cfg(feature = "supervisor_ui")]
    let moved_kill_loops_tx = kill_loops_tx.clone();
    #[cfg(feature = "supervisor_ui")]
    let web_ui_handle: Option<ServerHandle> = if cmd.do_start_supervisor_ui() {
        use txtx_supervisor_ui::start_supervisor_ui;
        let (supervisor_events_tx, supervisor_events_rx) = channel::unbounded();
        let web_ui_handle = start_supervisor_ui(
            runbook_name,
            runbook_description,
            registered_addons,
            block_store.clone(),
            block_broadcaster.clone(),
            action_item_events_tx,
            moved_relayer_channel_tx,
            relayer_channel_rx,
            moved_kill_loops_tx.clone(),
            &cmd.network_binding_ip_address,
            cmd.network_binding_port,
            supervisor_events_tx,
        )
        .await
        .map_err(|e| format!("failed to start web console: {}", e))?;

        let _ = hiro_system_kit::thread_named("Supervisor UI Event Runloop").spawn(move || {
            while let Ok(msg) = supervisor_events_rx.recv() {
                match msg {
                    txtx_supervisor_ui::SupervisorEvents::Started(network_binding) => {
                        println!(
                            "\n{} Starting the supervisor web console\n{}",
                            purple!("→"),
                            green!(format!("http://{}", network_binding))
                        );
                    }
                }
            }
        });
        Some(web_ui_handle)
    } else {
        None
    };

    #[cfg(feature = "supervisor_ui")]
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
                        println!("\n{}", green!("Runbook complete!"));
                    }
                    BlockEvent::Error(new_block) => {
                        let len = block_store.len();
                        block_store.insert(len, new_block.clone());
                    }
                    BlockEvent::Exit => break,
                }

                if do_propagate_event {
                    let _ = block_broadcaster.send(block_event.clone());
                    #[cfg(feature = "supervisor_ui")]
                    let _ = moved_relayer_channel_tx
                        .send(RelayerChannelEvent::ForwardEventToRelayer(block_event.clone()));
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
                        let _ = block_tx.send(BlockEvent::Exit);
                        #[cfg(feature = "supervisor_ui")]
                        let _ = relayer_channel_tx.send(RelayerChannelEvent::Exit);
                        #[cfg(feature = "supervisor_ui")]
                        if let Some(handle) = web_ui_handle {
                            println!("{} Stopping web console", purple!("→"));
                            let _ = handle.stop(true).await;
                        }
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

pub fn load_workspace_manifest_from_manifest_path(
    manifest_path: &str,
) -> Result<WorkspaceManifest, String> {
    let manifest_location = FileLocation::from_path_string(manifest_path)?;
    WorkspaceManifest::from_location(&manifest_location)
}

pub async fn load_runbooks_from_manifest(
    manifest: &WorkspaceManifest,
    manifest_path: &str,
    environment_selector: &Option<String>,
) -> Result<IndexMap<String, (Runbook, RunbookSources, String, Option<RunbookStateLocation>)>, String>
{
    let runbooks = read_runbooks_from_manifest(&manifest, environment_selector, None)?;
    println!("\n{} Processing manifest '{}'", purple!("→"), manifest_path);
    Ok(runbooks)
}

pub async fn load_runbook_from_manifest(
    manifest_path: &str,
    desired_runbook_name: &str,
    environment_selector: &Option<String>,
    cli_inputs: &Vec<String>,
    buffer_stdin: Option<String>,
) -> Result<(WorkspaceManifest, String, Runbook, Option<RunbookStateLocation>), String> {
    let manifest = load_workspace_manifest_from_manifest_path(manifest_path)?;
    let top_level_inputs_map =
        manifest.get_runbook_inputs(environment_selector, cli_inputs, buffer_stdin)?;

    let environment_selector =
        environment_selector.clone().or(manifest.environments.first().map(|(k, _)| k.clone()));

    let runbooks =
        load_runbooks_from_manifest(&manifest, manifest_path, &environment_selector).await?;
    // Select first runbook by default
    for (runbook_id, (mut runbook, runbook_sources, runbook_name, runbook_state)) in
        runbooks.into_iter()
    {
        if runbook_name.eq(desired_runbook_name) || runbook_id.eq(desired_runbook_name) {
            let authorization_context =
                AuthorizationContext::new(manifest.location.clone().unwrap());
            let res = runbook
                .build_contexts_from_sources(
                    runbook_sources,
                    top_level_inputs_map,
                    authorization_context,
                    get_addon_by_namespace,
                )
                .await;
            if let Err(diags) = res {
                for diag in diags.iter() {
                    println!("{} {}", red!("x"), diag);
                }
                std::process::exit(1);
            }
            return Ok((manifest, runbook_name, runbook, runbook_state));
        }
    }
    Err(format!("unable to retrieve runbook '{}' in manifest", desired_runbook_name))
}

pub async fn load_runbook_from_file_path(
    file_path: &str,
    cli_inputs: &Vec<String>,
    buffer_stdin: Option<String>,
) -> Result<(String, Runbook), String> {
    let location = FileLocation::from_path_string(file_path)?;
    let (runbook_name, mut runbook, runbook_sources) =
        read_runbook_from_location(&location, &None, &None, None)?;

    println!("\n{} Processing file '{}'", purple!("→"), file_path);
    let mut inputs_map = RunbookTopLevelInputsMap::new();
    inputs_map.override_values_with_cli_inputs(cli_inputs, buffer_stdin)?;
    let authorization_context = AuthorizationContext::new(location);
    let res = runbook
        .build_contexts_from_sources(
            runbook_sources,
            inputs_map,
            authorization_context,
            get_addon_by_namespace,
        )
        .await;
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

#[derive(serde::Serialize)]
pub struct DisplayableRunbookOutput {
    pub title: String,
    pub description: Option<String>,
    pub value: String,
}

pub fn collect_formatted_outputs(
    flow_contexts: &Vec<FlowContext>,
) -> (IndexMap<String, IndexMap<String, Vec<JsonValue>>>, Vec<DisplayableRunbookOutput>) {
    let mut json_outputs = vec![];
    let mut collected_outputs: IndexMap<String, IndexMap<String, Vec<JsonValue>>> = IndexMap::new();
    for flow_context in flow_contexts.iter() {
        let mut running_context_outputs: IndexMap<String, Vec<JsonValue>> = IndexMap::new();
        let grouped_actions_items =
            flow_context.execution_context.collect_outputs_constructs_results();

        for (_, items) in grouped_actions_items.iter() {
            for item in items.iter() {
                if let ActionItemRequestType::DisplayOutput(ref output) = item.action_type {
                    let value = output.value.to_json();

                    let output_name = output.name.to_string();
                    let display_name = output
                        .description
                        .as_ref()
                        .and_then(|d| Some(d.as_str()))
                        .unwrap_or(output_name.as_str());

                    json_outputs.push(DisplayableRunbookOutput {
                        title: output_name.to_string(),
                        description: output.description.clone(),
                        value: output.value.to_string(),
                    });
                    match running_context_outputs.get_mut(&output.name) {
                        Some(entries) => {
                            entries.push(value);
                        }
                        None => {
                            running_context_outputs.insert(display_name.to_string(), vec![value]);
                        }
                    }
                }
            }
        }
        collected_outputs.insert(flow_context.name.clone(), running_context_outputs);
    }
    (collected_outputs, json_outputs)
}

pub fn try_display_json_output(
    output_json: Option<Option<String>>,
    json_outputs: Vec<DisplayableRunbookOutput>,
    workspace_location: &FileLocation,
    runbook_id: &str,
    environment: &str,
) -> Result<(), String> {
    if let Some(output_loc) = output_json.as_ref() {
        let output = serde_json::to_string_pretty(&json_outputs)
            .map_err(|e| format!("failed to serialize outputs: {e}"))?;
        if let Some(output_loc) = output_loc {
            let mut output_location = workspace_location
                .get_parent_location()
                .map_err(|e| format!("failed to write to output file: {e}"))?;
            output_location
                .append_path(output_loc)
                .map_err(|e| format!("invalid output directory: {e}"))?;
            output_location
                .append_path(environment)
                .map_err(|e| format!("invalid output directory: {e}"))?;

            let now = chrono::Local::now();
            let formatted = now.format("%Y-%m-%d--%H-%M-%S").to_string();
            output_location
                .append_path(&format!("{}_{}.output.json", runbook_id, formatted))
                .map_err(|e| format!("invalid output file path: {e}"))?;

            output_location
                .write_content(output.as_bytes())
                .map_err(|e| format!("failed to write to output file: {e}"))?;
            println!(
                "{} {}",
                green!("✓"),
                format!("Outputs written to {}", output_location.to_string())
            );
        } else {
            println!("{}", output);
        }
    }
    Ok(())
}
