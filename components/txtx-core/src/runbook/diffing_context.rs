use super::{RunbookExecutionMode, RunningContext};
use kit::{
    helpers::fs::FileLocation,
    indexmap::IndexMap,
    types::{diagnostics::Diagnostic, types::Value, ConstructDid, PackageDid, RunbookId},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use similar::{capture_diff_slices, Algorithm, ChangeTag, DiffOp, TextDiff};
use std::{
    collections::HashSet,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunbookExecutionSnapshot {
    /// Organization authoring the workspace
    org: Option<String>,
    /// Workspace where the runbook
    workspace: Option<String>,
    /// Name of the runbook
    name: String,
    /// Keep track of the execution end date
    ended_at: String,
    ///
    runs: IndexMap<String, RunbookRunSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunbookRunSnapshot {
    ///
    pub inputs: IndexMap<String, Value>,
    /// Snapshot of the packages pulled by the runbook
    pub packages: IndexMap<PackageDid, PackageSnapshot>,
    /// Snapshot of the signing commands evaluations
    pub signing_commands: IndexMap<ConstructDid, SigningCommandSnapshot>,
    /// Snapshot of the commands evaluations
    pub commands: IndexMap<ConstructDid, CommandSnapshot>,
}

impl RunbookExecutionSnapshot {
    pub fn new(runbook_id: &RunbookId) -> Self {
        let ended_at = now_as_string();
        Self {
            org: runbook_id.org.clone(),
            workspace: runbook_id.workspace.clone(),
            name: runbook_id.name.clone(),
            ended_at,
            runs: IndexMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSnapshot {
    path: String,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningCommandSnapshot {
    package_did: PackageDid,
    construct_type: String,
    construct_name: String,
    construct_addon: Option<String>,
    construct_location: FileLocation,
    downstream_constructs_dids: Vec<ConstructDid>,
    inputs: IndexMap<String, CommandInputSnapshot>,
    outputs: IndexMap<String, CommandOutputSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandSnapshot {
    package_did: PackageDid,
    construct_type: String,
    construct_name: String,
    construct_location: FileLocation,
    construct_addon: Option<String>,
    upstream_constructs_dids: Vec<ConstructDid>,
    inputs: IndexMap<String, CommandInputSnapshot>,
    outputs: IndexMap<String, CommandOutputSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandInputSnapshot {
    pub value_pre_evaluation: Option<String>,
    pub value_post_evaluation: Value,
    pub critical: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandOutputSnapshot {
    pub value: Value,
    pub signed: bool,
}

#[derive(Debug, Clone)]
pub struct RunbookSnapshotContext {}

impl RunbookSnapshotContext {
    pub fn new() -> Self {
        Self {}
    }

    pub fn snapshot_runbook_execution(
        &self,
        runbook_id: &RunbookId,
        running_contexts: &Vec<RunningContext>,
        previous_snapshot: Option<RunbookExecutionSnapshot>,
    ) -> Result<RunbookExecutionSnapshot, Diagnostic> {
        // &runbook.workspace_context,
        // workspace_context: &RunbookWorkspaceContext,

        let mut snapshot = RunbookExecutionSnapshot::new(&runbook_id);

        for running_context in running_contexts.iter() {
            let run_id = running_context.inputs_set.name.clone();
            let (mut run, constructs_ids_to_consider) =
                match &running_context.execution_context.execution_mode {
                    RunbookExecutionMode::Ignored => {
                        // Runbook was fully executed, the source of truth is the snapshoted context
                        let previous_run = previous_snapshot
                            .as_ref()
                            .expect("unexpected error: former snapshot should have been provided")
                            .runs
                            .get(&run_id)
                            .expect("unexpected error: former snapshot corrupted")
                            .clone();
                        snapshot.runs.insert(run_id, previous_run);
                        continue;
                    }
                    RunbookExecutionMode::Full => {
                        // Runbook was fully executed, the source of truth is the new running context
                        let mut inputs = running_context.inputs_set.storage.clone();
                        inputs.sort_keys();
                        let constructs_ids_to_consider = vec![];
                        let run = RunbookRunSnapshot {
                            inputs,
                            packages: IndexMap::new(),
                            signing_commands: IndexMap::new(),
                            commands: IndexMap::new(),
                        };
                        (run, constructs_ids_to_consider)
                    }
                    RunbookExecutionMode::Partial(updated_constructs) => {
                        // Runbook was partially executed. We need to update the previous snapshot, only with the command that ran
                        let previous_run = previous_snapshot
                            .as_ref()
                            .ok_or(diagnosed_error!(
                                "former snapshot should have been provided"
                            ))?
                            .runs
                            .get(&run_id)
                            .ok_or(diagnosed_error!(
                                "unexpected error: former snapshot corrupted"
                            ))?
                            .clone();
                        let constructs_ids_to_consider = updated_constructs.clone();
                        (previous_run, constructs_ids_to_consider)
                    }
                };

            // Order packages
            let mut packages = running_context
                .workspace_context
                .packages
                .keys()
                .into_iter()
                .collect::<Vec<_>>();
            packages.sort_by_key(|k| k.did().0);

            for package_id in packages {
                let package_did = package_id.did();
                match run.packages.get_mut(&package_did) {
                    Some(package) => {
                        package.name = package_id.package_name.clone();
                        package.path = package_id.package_location.to_string();
                    }
                    None => {
                        run.packages.insert(
                            package_did,
                            PackageSnapshot {
                                name: package_id.package_name.clone(),
                                path: package_id.package_location.to_string(),
                            },
                        );
                    }
                }
            }

            // Signing commands
            for (signing_construct_did, downstream_constructs_dids) in running_context
                .execution_context
                .signing_commands_downstream_dependencies
                .iter()
            {
                let command_instance = running_context
                    .execution_context
                    .signing_commands_instances
                    .get(&signing_construct_did)
                    .unwrap();

                let signing_construct_id = running_context
                    .workspace_context
                    .constructs
                    .get(signing_construct_did)
                    .unwrap();

                let downstream_constructs_dids = downstream_constructs_dids
                    .iter()
                    .map(|c| c.clone())
                    .collect();

                let command_to_update = match run.signing_commands.get_mut(signing_construct_did) {
                    Some(signing_command) => signing_command,
                    None => {
                        let new_command = SigningCommandSnapshot {
                            package_did: command_instance.package_id.did(),
                            construct_type: signing_construct_id.construct_type.clone(),
                            construct_name: signing_construct_id.construct_name.clone(),
                            construct_location: signing_construct_id.construct_location.clone(),
                            construct_addon: None,
                            downstream_constructs_dids,
                            inputs: IndexMap::new(),
                            outputs: IndexMap::new(),
                        };
                        run.signing_commands
                            .insert(signing_construct_did.clone(), new_command);
                        run.signing_commands.get_mut(signing_construct_did).unwrap()
                    }
                };

                // Check if construct is sensitive
                let is_construct_sensitive = false;

                if let Some(inputs_evaluations) = running_context
                    .execution_context
                    .commands_inputs_evaluation_results
                    .get(signing_construct_did)
                {
                    for input in command_instance.specification.inputs.iter() {
                        let Some(value) = inputs_evaluations.inputs.get_value(&input.name) else {
                            continue;
                        };
                        let is_input_sensitive = input.sensitive;
                        let _should_values_be_hashed = is_construct_sensitive || is_input_sensitive;

                        match command_instance.get_expression_from_input(input) {
                            Ok(Some(entry)) => {
                                let input_name = &input.name;
                                match command_to_update.inputs.get_mut(input_name) {
                                    Some(input) => {
                                        input.value_pre_evaluation =
                                            Some(entry.to_string().trim().to_string());
                                        input.value_post_evaluation = value.clone();
                                        input.critical = true;
                                    }
                                    None => {
                                        command_to_update.inputs.insert(
                                            input_name.clone(),
                                            CommandInputSnapshot {
                                                value_pre_evaluation: Some(
                                                    entry.to_string().trim().to_string(),
                                                ),
                                                value_post_evaluation: value.clone(),
                                                critical: true,
                                            },
                                        );
                                    }
                                }
                            }
                            _ => {}
                        };
                    }
                }

                if let Some(outputs_results) = running_context
                    .execution_context
                    .commands_execution_results
                    .get(signing_construct_did)
                {
                    for output in command_instance.specification.outputs.iter() {
                        let Some(value) = outputs_results.outputs.get(&output.name) else {
                            continue;
                        };
                        let output_name = &output.name;
                        match command_to_update.outputs.get_mut(output_name) {
                            Some(output_to_update) => {
                                output_to_update.value = value.clone();
                                output_to_update.signed = false;
                            }
                            None => {
                                command_to_update.outputs.insert(
                                    output_name.clone(),
                                    CommandOutputSnapshot {
                                        value: value.clone(),
                                        signed: false,
                                    },
                                );
                            }
                        }
                    }
                }
            }

            for construct_did in running_context
                .execution_context
                .order_for_commands_execution
                .iter()
            {
                if !constructs_ids_to_consider.is_empty()
                    && !constructs_ids_to_consider.contains(construct_did)
                {
                    continue;
                }

                let command_instance = match running_context
                    .execution_context
                    .commands_instances
                    .get(&construct_did)
                {
                    Some(entry) => entry,
                    None => {
                        continue;
                    }
                };

                let construct_id = running_context
                    .workspace_context
                    .constructs
                    .get(construct_did)
                    .unwrap();

                let critical = running_context
                    .execution_context
                    .signed_commands
                    .contains(construct_did);

                let mut upstream_constructs_dids = vec![];
                if let Some(deps) = running_context
                    .execution_context
                    .signed_commands_upstream_dependencies
                    .get(construct_did)
                {
                    for construct_did in deps.iter() {
                        if running_context
                            .workspace_context
                            .constructs
                            .get(construct_did)
                            .is_some()
                        {
                            upstream_constructs_dids.push(construct_did.clone());
                        }
                    }
                }

                let command_to_update = match run.commands.get_mut(construct_did) {
                    Some(signing_command) => signing_command,
                    None => {
                        let new_command = CommandSnapshot {
                            package_did: command_instance.package_id.did(),
                            construct_type: construct_id.construct_type.clone(),
                            construct_name: construct_id.construct_name.clone(),
                            construct_location: construct_id.construct_location.clone(),
                            construct_addon: None,
                            upstream_constructs_dids,
                            inputs: IndexMap::new(),
                            outputs: IndexMap::new(),
                        };
                        run.commands.insert(construct_did.clone(), new_command);
                        run.commands.get_mut(construct_did).unwrap()
                    }
                };

                if let Some(inputs_evaluations) = running_context
                    .execution_context
                    .commands_inputs_evaluation_results
                    .get(construct_did)
                {
                    let mut sorted_inputs = command_instance.specification.inputs.clone();
                    sorted_inputs.sort_by(|a, b| a.name.cmp(&b.name));
                    for input in sorted_inputs.iter() {
                        let Some(value) = inputs_evaluations.inputs.get_value(&input.name) else {
                            continue;
                        };
                        let is_input_sensitive = input.sensitive;

                        match command_instance.get_expression_from_input(input) {
                            Ok(Some(entry)) => {
                                let input_name = &input.name;
                                match command_to_update.inputs.get_mut(input_name) {
                                    Some(input) => {
                                        input.value_pre_evaluation =
                                            Some(entry.to_string().trim().to_string());
                                        input.value_post_evaluation = value.clone();
                                        input.critical = critical;
                                    }
                                    None => {
                                        command_to_update.inputs.insert(
                                            input_name.clone(),
                                            CommandInputSnapshot {
                                                value_pre_evaluation: Some(
                                                    entry.to_string().trim().to_string(),
                                                ),
                                                value_post_evaluation: value.clone(),
                                                critical: critical,
                                            },
                                        );
                                    }
                                }
                            }
                            _ => {}
                        };
                    }
                }

                if let Some(ref critical_output) =
                    command_instance.specification.create_critical_output
                {
                    if let Some(outputs_results) = running_context
                        .execution_context
                        .commands_execution_results
                        .get(construct_did)
                    {
                        let mut sorted_outputs = command_instance.specification.outputs.clone();
                        sorted_outputs.sort_by(|a, b| a.name.cmp(&b.name));

                        for output in sorted_outputs {
                            let Some(value) = outputs_results.outputs.get(&output.name) else {
                                continue;
                            };
                            // This is a major shortcut, we should revisit this approach
                            let value = match value.as_object().map(|o| o.get(critical_output)) {
                                Some(Some(value)) => value.clone(),
                                _ => Value::null(),
                            };
                            let output_name = &output.name;
                            match command_to_update.outputs.get_mut(output_name) {
                                Some(output_to_update) => {
                                    output_to_update.value = value.clone();
                                    output_to_update.signed = false;
                                }
                                None => {
                                    command_to_update.outputs.insert(
                                        output_name.clone(),
                                        CommandOutputSnapshot {
                                            value: value.clone(),
                                            signed: false,
                                        },
                                    );
                                }
                            }
                        }
                    }
                }
            }

            snapshot.runs.insert(run_id, run);
        }

        let rountrip: RunbookExecutionSnapshot = serde_json::from_value(json!(snapshot)).unwrap();
        Ok(rountrip)
    }

    pub fn diff(
        &self,
        old: RunbookExecutionSnapshot,
        new: RunbookExecutionSnapshot,
    ) -> ConsolidatedChanges {
        let old_ref = old.clone();
        let new_ref = new.clone();

        let mut consolidated_changes = ConsolidatedChanges::new();

        let empty_string = "".to_string();

        // TextDiff::from_lines(&old.name, &new.name);
        // diffs.push(evaluated_diff(
        //     None,
        //     TextDiff::from_lines(&old.name, &new.name),
        //     format!("Runbook's name updated"),
        //     false,
        // ));
        // diffs.push(evaluated_diff(
        //     None,
        //     TextDiff::from_lines(
        //         old.org.as_ref().unwrap_or(&empty_string),
        //         &new.org.as_ref().unwrap_or(&empty_string),
        //     ),
        //     format!("Runbook's org updated"),
        //     false,
        // ));
        // diffs.push(evaluated_diff(
        //     None,
        //     TextDiff::from_lines(
        //         old.workspace.as_ref().unwrap_or(&empty_string),
        //         new.workspace.as_ref().unwrap_or(&empty_string),
        //     ),
        //     format!("Runbook's workspace updated"),
        //     false,
        // ));

        let old_runs_ids = old_ref
            .runs
            .iter()
            .map(|(c, _)| c.to_string())
            .collect::<Vec<_>>();
        let new_runs_ids = new_ref
            .runs
            .iter()
            .map(|(c, _)| c.to_string())
            .collect::<Vec<_>>();
        let runs_ids_sequence_changes =
            capture_diff_slices(Algorithm::Myers, &old_runs_ids, &new_runs_ids);
        // println!("Comparing \n{:?}\n{:?}", old_signing_commands_dids, new_signing_commands_dids);

        let mut comparable_runs_ids_list = vec![];
        // let mut runs_ids_to_add = vec![];
        // let mut runs_ids_to_remove = vec![];
        for change in runs_ids_sequence_changes.iter() {
            match change {
                DiffOp::Equal {
                    old_index,
                    new_index,
                    len,
                } => {
                    for i in 0..*len {
                        comparable_runs_ids_list.push((old_index + i, new_index + i));
                    }
                }
                DiffOp::Delete {
                    old_index,
                    old_len,
                    new_index: _,
                } => {
                    for i in 0..*old_len {
                        let entry = old_ref.runs.get_index(old_index + i).unwrap().0.clone();
                        consolidated_changes.old_plans_to_rem.push(entry)
                    }
                }
                DiffOp::Insert {
                    old_index: _,
                    new_index,
                    new_len,
                } => {
                    for i in 0..*new_len {
                        let entry = new_ref.runs.get_index(new_index + i).unwrap().0.clone();
                        consolidated_changes.new_plans_to_add.push(entry)
                    }
                }
                DiffOp::Replace {
                    old_index: _,
                    old_len: _,
                    new_index: _,
                    new_len: _,
                } => {
                    // for i in 0..*old_len {
                    //     for j in 0..*new_len {
                    //         comparable_runs_ids_list.push((old_index + i, new_index + j));
                    //     }
                    // }
                }
            }
        }

        for (old_index, new_index) in comparable_runs_ids_list.into_iter() {
            let mut plan_changes = ConsolidatedPlanChanges::new();

            let ((old_run_id, old_run), (new_run_id, new_run)) = match (
                old_ref.runs.get_index(old_index),
                new_ref.runs.get_index(new_index),
            ) {
                (Some(old), Some(new)) => (old, new),
                _ => continue,
            };

            // Construct name
            plan_changes.contructs_to_update.push(evaluated_diff(
                None,
                TextDiff::from_lines(old_run_id, new_run_id),
                format!("Chain id updated"),
                true,
            ));

            // if changes, we should recompute some temporary ids for packages and constructs
            let old_signing_commands = old_run.signing_commands.clone();
            let new_signing_commands = new_run.signing_commands.clone();

            let old_signing_commands_dids = old_signing_commands
                .iter()
                .map(|(c, _)| c.to_string())
                .collect::<Vec<_>>();
            let new_signing_commands_dids = new_signing_commands
                .iter()
                .map(|(c, _)| c.to_string())
                .collect::<Vec<_>>();
            let signing_command_sequence_changes = capture_diff_slices(
                Algorithm::Myers,
                &old_signing_commands_dids,
                &new_signing_commands_dids,
            );
            // println!("Comparing \n{:?}\n{:?}", old_signing_commands_dids, new_signing_commands_dids);

            let mut comparable_signing_constructs_list = vec![];
            for change in signing_command_sequence_changes.iter() {
                match change {
                    DiffOp::Equal {
                        old_index,
                        new_index,
                        len,
                    } => {
                        for i in 0..*len {
                            comparable_signing_constructs_list.push((old_index + i, new_index + i));
                        }
                    }
                    DiffOp::Delete {
                        old_index: _,
                        old_len: _,
                        new_index: _,
                    } => {
                        // comparable_signing_constructs_list.push((*old_index, *new_index));
                    }
                    DiffOp::Insert {
                        old_index: _,
                        new_index: _,
                        new_len: _,
                    } => {
                        // comparable_signing_constructs_list.push((*old_index, *new_index));
                    }
                    DiffOp::Replace {
                        old_index: _,
                        old_len: _,
                        new_index: _,
                        new_len: _,
                    } => {
                        // comparable_signing_constructs_list.push((*old_index, *new_index));
                    }
                }
            }

            for (old_index, new_index) in comparable_signing_constructs_list.into_iter() {
                let ((old_signing_command_id, old_signing_command), (_, new_signing_command)) =
                    match (
                        old_signing_commands.get_index(old_index),
                        new_signing_commands.get_index(new_index),
                    ) {
                        (Some(old), Some(new)) => (old, new),
                        _ => continue,
                    };

                // Construct name
                plan_changes.contructs_to_update.push(evaluated_diff(
                    Some(old_signing_command_id.clone()),
                    TextDiff::from_lines(
                        old_signing_command.construct_name.as_str(),
                        new_signing_command.construct_name.as_str(),
                    ),
                    format!("Signing command's name updated"),
                    false,
                ));

                // Construct path
                plan_changes.contructs_to_update.push(evaluated_diff(
                    Some(old_signing_command_id.clone()),
                    TextDiff::from_lines(
                        &old_signing_command.construct_location.to_string(),
                        &new_signing_command.construct_location.to_string(),
                    ),
                    format!("Signing command's path updated"),
                    false,
                ));

                // Construct driver
                plan_changes.contructs_to_update.push(evaluated_diff(
                    Some(old_signing_command_id.clone()),
                    TextDiff::from_lines(
                        old_signing_command
                            .construct_addon
                            .as_ref()
                            .unwrap_or(&empty_string),
                        new_signing_command
                            .construct_addon
                            .as_ref()
                            .unwrap_or(&empty_string),
                    ),
                    format!("Signing command's driver updated"),
                    true,
                ));

                // Let's look at the signed constructs
                let mut visited_constructs = HashSet::new();

                let mut inner_changes = diff_command_snapshots(
                    &old_run,
                    &old_signing_command.downstream_constructs_dids,
                    &new_run,
                    &new_signing_command.downstream_constructs_dids,
                    &mut visited_constructs,
                );

                plan_changes
                    .new_constructs_to_add
                    .append(&mut inner_changes.new_constructs_to_add);
                plan_changes
                    .old_constructs_to_rem
                    .append(&mut inner_changes.old_constructs_to_rem);
                plan_changes
                    .contructs_to_update
                    .append(&mut inner_changes.contructs_to_update);
            }
            consolidated_changes
                .plans_to_update
                .insert(new_run_id.into(), plan_changes);
        }

        consolidated_changes
    }
}

pub fn diff_command_snapshots(
    old_run: &RunbookRunSnapshot,
    old_construct_dids: &Vec<ConstructDid>,
    new_run: &RunbookRunSnapshot,
    new_construct_dids: &Vec<ConstructDid>,
    visited_constructs: &mut HashSet<ConstructDid>,
) -> ConsolidatedPlanChanges {
    let mut consolidated_changes = ConsolidatedPlanChanges::new();

    let empty_string = "".to_string();

    // Let's look at the signed constructs
    // First: Any new construct?
    let old_signed_commands_dids = old_construct_dids
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<_>>();
    let new_signed_commands_dids = new_construct_dids
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<_>>();
    let signed_command_sequence_changes = capture_diff_slices(
        Algorithm::Myers,
        &old_signed_commands_dids,
        &new_signed_commands_dids,
    );

    // println!("Comparing \n{:?}\n{:?}", old_signed_commands_dids, new_signed_commands_dids);

    let mut comparable_signed_constructs_list = vec![];
    for change in signed_command_sequence_changes.iter() {
        match change {
            DiffOp::Equal {
                old_index,
                new_index,
                len,
            } => {
                for i in 0..*len {
                    comparable_signed_constructs_list.push((old_index + i, new_index + i));
                }
            }
            DiffOp::Delete {
                old_index: _,
                old_len: _,
                new_index: _,
            } => {
                // Not true!
                // comparable_signed_constructs_list.push((*old_index, *new_index));
            }
            DiffOp::Insert {
                old_index: _,
                new_index,
                new_len,
            } => {
                for i in 0..*new_len {
                    let entry = new_construct_dids.get(new_index + i).unwrap().clone();
                    let command = match new_run.commands.get(&entry) {
                        Some(e) => Some(e.clone()),
                        None => None,
                    };
                    consolidated_changes
                        .new_constructs_to_add
                        .push((entry, command))
                }
            }
            DiffOp::Replace {
                old_index: _,
                old_len: _,
                new_index: _,
                new_len: _,
            } => {
                // comparable_signed_constructs_list.push((*old_index, *new_index));
            }
        }
    }

    for (old_index, new_index) in comparable_signed_constructs_list.into_iter() {
        let (old_construct_did, new_construct_did) = match (
            old_construct_dids.get(old_index),
            new_construct_dids.get(new_index),
        ) {
            (Some(old), Some(new)) => (old, new),
            _ => continue,
        };

        if visited_constructs.contains(old_construct_did) {
            continue;
        }
        visited_constructs.insert(old_construct_did.clone());

        let old_command = old_run.commands.get(old_construct_did).unwrap().clone();
        let new_command = new_run.commands.get(new_construct_did).unwrap().clone();

        // Construct name
        consolidated_changes
            .contructs_to_update
            .push(evaluated_diff(
                Some(old_construct_did.clone()),
                TextDiff::from_lines(
                    old_command.construct_name.as_str(),
                    new_command.construct_name.as_str(),
                ),
                format!("Non-signing command's name updated"),
                false,
            ));

        // Construct path
        consolidated_changes
            .contructs_to_update
            .push(evaluated_diff(
                Some(old_construct_did.clone()),
                TextDiff::from_lines(
                    &old_command.construct_location.to_string(),
                    &new_command.construct_location.to_string(),
                ),
                format!("Non-signing command's path updated"),
                false,
            ));

        // Construct driver
        consolidated_changes
            .contructs_to_update
            .push(evaluated_diff(
                Some(old_construct_did.clone()),
                TextDiff::from_lines(
                    old_command
                        .construct_addon
                        .as_ref()
                        .unwrap_or(&empty_string),
                    new_command
                        .construct_addon
                        .as_ref()
                        .unwrap_or(&empty_string),
                ),
                format!("Non-signing command's driver updated"),
                false,
            ));

        // old_command.inputs.sort_by(|a, b| a.name.cmp(&b.name));
        // new_command.inputs.sort_by(|a, b| a.name.cmp(&b.name));

        // Check inputs
        let old_inputs = old_command
            .inputs
            .iter()
            .map(|(i, _)| i.to_string())
            .collect::<Vec<_>>();
        let new_inputs = new_command
            .inputs
            .iter()
            .map(|(i, _)| i.to_string())
            .collect::<Vec<_>>();

        let inputs_sequence_changes = capture_diff_slices(Algorithm::Lcs, &old_inputs, &new_inputs);

        let mut comparable_inputs_list = vec![];
        for change in inputs_sequence_changes.iter() {
            // println!("{:?}", change);
            match change {
                DiffOp::Equal {
                    old_index,
                    new_index,
                    len,
                } => {
                    for i in 0..*len {
                        comparable_inputs_list.push((old_index + i, new_index + i));
                    }
                }
                DiffOp::Delete {
                    old_index: _,
                    old_len: _,
                    new_index: _,
                } => {
                    // comparable_inputs_list.push((*old_index, *new_index));
                }
                DiffOp::Insert {
                    old_index: _,
                    new_index: _,
                    new_len: _,
                } => {
                    // comparable_inputs_list.push((*old_index, *new_index));
                }
                DiffOp::Replace {
                    old_index: _,
                    old_len: _,
                    new_index: _,
                    new_len: _,
                } => {
                    // comparable_inputs_list.push((*old_index, *new_index));
                }
            }
        }
        // println!("{:?}", comparable_inputs_list);

        for (old_index, new_index) in comparable_inputs_list.into_iter() {
            let ((old_input_name, old_input), (new_input_name, new_input)) = match (
                old_command.inputs.get_index(old_index),
                new_command.inputs.get_index(new_index),
            ) {
                (Some(old), Some(new)) => (old, new),
                _ => continue,
            };

            // println!("{}:{}", old_input.name, new_input.name);

            // Input name
            consolidated_changes
                .contructs_to_update
                .push(evaluated_diff(
                    Some(old_construct_did.clone()),
                    TextDiff::from_lines(old_input_name.as_str(), new_input_name.as_str()),
                    format!("Non-signing command's input name updated"),
                    false,
                ));

            // Input value_pre_evaluation
            consolidated_changes
                .contructs_to_update
                .push(evaluated_diff(
                    Some(old_construct_did.clone()),
                    TextDiff::from_lines(
                        old_input
                            .value_pre_evaluation
                            .as_ref()
                            .unwrap_or(&empty_string),
                        new_input
                            .value_pre_evaluation
                            .as_ref()
                            .unwrap_or(&empty_string),
                    ),
                    format!("Non-signing command's input value_pre_evaluation updated"),
                    true,
                ));
            // Input value_post_evaluation
            if let Some(props) = new_input.value_post_evaluation.as_object() {
                for (prop, new_value) in props.iter() {
                    let Some(old_value) = old_input
                        .value_post_evaluation
                        .as_object()
                        .and_then(|o| o.get(prop))
                    else {
                        continue;
                    };
                    consolidated_changes
                        .contructs_to_update
                        .push(evaluated_diff(
                            Some(old_construct_did.clone()),
                            TextDiff::from_lines(&old_value.to_string(), &new_value.to_string()),
                            format!("Non-signing command's input value_post_evaluation updated"),
                            true,
                        ));
                }
            } else {
                consolidated_changes
                    .contructs_to_update
                    .push(evaluated_diff(
                        Some(old_construct_did.clone()),
                        TextDiff::from_lines(
                            &old_input.value_post_evaluation.to_string(),
                            &new_input.value_post_evaluation.to_string(),
                        ),
                        format!("Non-signing command's input value_post_evaluation updated"),
                        true,
                    ));
            }
        }

        // Checking the outputs
        let old_outputs = old_command
            .outputs
            .iter()
            .map(|(i, _)| i.to_string())
            .collect::<Vec<_>>();
        let new_outputs = new_command
            .outputs
            .iter()
            .map(|(i, _)| i.to_string())
            .collect::<Vec<_>>();

        let outputs_sequence_changes =
            capture_diff_slices(Algorithm::Patience, &old_outputs, &new_outputs);

        // println!("Comparing \n{:?}\n{:?}", old_inputs, new_inputs);

        let mut comparable_outputs_list = vec![];
        for change in outputs_sequence_changes.iter() {
            // println!("{:?}", change);
            match change {
                DiffOp::Equal {
                    old_index,
                    new_index,
                    len,
                } => {
                    for i in 0..*len {
                        comparable_outputs_list.push((old_index + i, new_index + i));
                    }
                }
                DiffOp::Delete {
                    old_index: _,
                    old_len: _,
                    new_index: _,
                } => {
                    // comparable_outputs_list.push((*old_index, *new_index));
                }
                DiffOp::Insert {
                    old_index: _,
                    new_index: _,
                    new_len: _,
                } => {
                    // comparable_outputs_list.push((*old_index, *new_index));
                }
                DiffOp::Replace {
                    old_index: _,
                    old_len: _,
                    new_index: _,
                    new_len: _,
                } => {
                    // comparable_outputs_list.push((*old_index, *new_index));
                }
            }
        }
        // println!("{:?}", comparable_inputs_list);

        for (old_index, new_index) in comparable_outputs_list.into_iter() {
            let ((old_output_name, old_output), (new_output_name, new_output)) = match (
                old_command.outputs.get_index(old_index),
                new_command.outputs.get_index(new_index),
            ) {
                (Some(old), Some(new)) => (old, new),
                _ => continue,
            };

            // println!("{}:{}", old_input.name, new_input.name);

            // Output name
            consolidated_changes
                .contructs_to_update
                .push(evaluated_diff(
                    Some(old_construct_did.clone()),
                    TextDiff::from_lines(old_output_name.as_str(), new_output_name.as_str()),
                    format!("Non-signing command's output name updated"),
                    false,
                ));

            // Input value_pre_evaluation
            consolidated_changes
                .contructs_to_update
                .push(evaluated_diff(
                    Some(new_construct_did.clone()),
                    TextDiff::from_lines(
                        old_output.value.to_string().as_str(),
                        new_output.value.to_string().as_str(),
                    ),
                    format!("Non-signing command's output value_pre_evaluation updated"),
                    true,
                ));
        }

        if old_command.upstream_constructs_dids.is_empty()
            && new_command.upstream_constructs_dids.is_empty()
        {
            continue;
        }

        let mut inner_changes = diff_command_snapshots(
            old_run,
            &old_command.upstream_constructs_dids,
            new_run,
            &new_command.upstream_constructs_dids,
            visited_constructs,
        );

        consolidated_changes
            .new_constructs_to_add
            .append(&mut inner_changes.new_constructs_to_add);
        consolidated_changes
            .old_constructs_to_rem
            .append(&mut inner_changes.old_constructs_to_rem);
        consolidated_changes
            .contructs_to_update
            .append(&mut inner_changes.contructs_to_update);
    }
    consolidated_changes
}

#[derive(Debug)]
pub struct ConsolidatedChanges {
    pub old_plans_to_rem: Vec<String>,
    pub new_plans_to_add: Vec<String>,
    pub plans_to_update: IndexMap<String, ConsolidatedPlanChanges>,
}

#[derive(Debug)]
pub struct ConsolidatedPlanChanges {
    pub old_constructs_to_rem: Vec<ConstructDid>,
    pub new_constructs_to_add: Vec<(ConstructDid, Option<CommandSnapshot>)>,
    pub contructs_to_update: Vec<Change>,
}

impl ConsolidatedPlanChanges {
    pub fn new() -> Self {
        Self {
            old_constructs_to_rem: vec![],
            new_constructs_to_add: vec![],
            contructs_to_update: vec![],
        }
    }
}

#[derive(Hash, PartialEq, Eq)]
pub enum SynthesizedChange {
    Edition(Vec<String>, bool),
    Addition(ConstructDid),
}

impl ConsolidatedChanges {
    pub fn new() -> Self {
        Self {
            old_plans_to_rem: vec![],
            new_plans_to_add: vec![],
            plans_to_update: IndexMap::new(),
        }
    }

    pub fn get_synthesized_changes(
        &self,
    ) -> IndexMap<SynthesizedChange, Vec<(String, Option<ConstructDid>)>> {
        let mut reverse_lookup: IndexMap<SynthesizedChange, Vec<(String, Option<ConstructDid>)>> =
            IndexMap::new();

        for (plan_id, plan_changes) in self.plans_to_update.iter() {
            for change in plan_changes.contructs_to_update.iter() {
                if change.description.is_empty() {
                    continue;
                }
                let key = SynthesizedChange::Edition(change.description.clone(), change.critical);
                let value = (plan_id.to_string(), change.construct_did.clone());
                if let Some(list) = reverse_lookup.get_mut(&key) {
                    list.push(value)
                } else {
                    reverse_lookup.insert(key, vec![value]);
                }
            }
            for (new_construct, _) in plan_changes.new_constructs_to_add.iter() {
                let key = SynthesizedChange::Addition(new_construct.clone());
                let value = (plan_id.to_string(), Some(new_construct.clone()));
                if let Some(list) = reverse_lookup.get_mut(&key) {
                    list.push(value)
                } else {
                    reverse_lookup.insert(key, vec![value]);
                }
            }
        }

        reverse_lookup
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    pub critical: bool,
    pub construct_did: Option<ConstructDid>,
    pub label: String,
    pub description: Vec<String>,
}

fn evaluated_diff<'a, 'b, 'c>(
    construct_did: Option<ConstructDid>,
    diff: TextDiff<'a, 'b, 'c, str>,
    label: String,
    critical: bool,
) -> Change
where
    'a: 'b,
    'b: 'a,
{
    let mut result = Change {
        critical,
        construct_did: construct_did.clone(),
        label,
        description: vec![],
    };

    let mut changes = vec![];
    for diff_result in diff.iter_all_changes() {
        if let ChangeTag::Equal = diff_result.tag() {
            continue;
        }
        changes.push((diff_result.tag(), diff_result.value().to_string()));
    }

    for (tag, change) in changes.into_iter() {
        let sign = match tag {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => unreachable!(),
        };
        result.description.push(format!("{}{}", sign, change))
    }

    result
}

fn now_as_string() -> String {
    // Get the current system time
    let now = SystemTime::now();
    // Calculate the duration since the Unix epoch
    let duration_since_epoch = now.duration_since(UNIX_EPOCH).expect("Time went backwards");
    // Convert the duration to seconds and nanoseconds
    let seconds = duration_since_epoch.as_secs() as i64;
    let nanoseconds = duration_since_epoch.subsec_nanos();
    let datetime = chrono::DateTime::from_timestamp(seconds, nanoseconds).unwrap();
    // Display the DateTime using the RFC 3339 format
    datetime.to_rfc3339()
}
// Shortcut:
// Support for constructs being removed / added / replaced
