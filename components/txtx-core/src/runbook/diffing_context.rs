use kit::types::{types::Value, ConstructDid, PackageDid, RunbookId};
use serde::{Deserialize, Serialize};
use similar::{capture_diff_slices, Algorithm, ChangeTag, DiffOp, TextDiff};
use std::time::{SystemTime, UNIX_EPOCH};

use super::{RunbookExecutionContext, RunbookWorkspaceContext};

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
    /// Snapshot of the packages pulled by the runbook
    packages: Vec<PackageSnapshot>,
    /// Snapshot of the signing commands evaluations
    signing_commands: Vec<SigningCommandSnapshot>,
    /// Snapshot of the commands evaluations
    commands: Vec<CommandSnapshot>,
}

impl RunbookExecutionSnapshot {
    pub fn get_command_with_construct_did(
        &mut self,
        construct_did: &ConstructDid,
    ) -> &CommandSnapshot {
        for command in self.commands.iter() {
            if command.construct_did.eq(construct_did) {
                return command;
            }
        }
        unreachable!()
    }
}

impl RunbookExecutionSnapshot {
    pub fn new(runbook_id: &RunbookId) -> Self {
        let ended_at = now_as_string();
        Self {
            org: runbook_id.org.clone(),
            workspace: runbook_id.workspace.clone(),
            name: runbook_id.name.clone(),
            ended_at,
            packages: vec![],
            signing_commands: vec![],
            commands: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSnapshot {
    did: PackageDid,
    path: String,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningCommandSnapshot {
    package_did: PackageDid,
    construct_did: ConstructDid,
    construct_type: String,
    construct_name: String,
    construct_addon: Option<String>,
    construct_path: String,
    downstream_constructs_dids: Vec<ConstructDid>,
    inputs: Vec<CommandInputSnapshot>,
    outputs: Vec<CommandOutputSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandSnapshot {
    package_did: PackageDid,
    construct_did: ConstructDid,
    construct_type: String,
    construct_name: String,
    construct_path: String,
    construct_addon: Option<String>,
    upstream_constructs_dids: Vec<ConstructDid>,
    inputs: Vec<CommandInputSnapshot>,
    outputs: Vec<CommandOutputSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandInputSnapshot {
    pub name: String,
    pub value_pre_evaluation: Option<String>,
    pub value_post_evaluation: Value,
    pub critical: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandOutputSnapshot {
    pub name: String,
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
        execution_context: &RunbookExecutionContext,
        workspace_context: &RunbookWorkspaceContext,
    ) -> RunbookExecutionSnapshot {
        let mut snapshot = RunbookExecutionSnapshot::new(&workspace_context.runbook_id);

        for (package_id, _) in workspace_context.packages.iter() {
            snapshot.packages.push(PackageSnapshot {
                did: package_id.did(),
                name: package_id.package_name.clone(),
                path: package_id.package_location.to_string(),
            })
        }

        // Signing commands
        for (signing_construct_did, downstream_constructs_dids) in execution_context
            .signing_commands_downstream_dependencies
            .iter()
        {
            let command_instance = execution_context
                .signing_commands_instances
                .get(&signing_construct_did)
                .unwrap();

            let signing_construct_id = workspace_context
                .constructs
                .get(signing_construct_did)
                .unwrap();

            let mut inputs: Vec<CommandInputSnapshot> = vec![];

            // Check if construct is sensitive
            let is_construct_sensitive = false;

            if let Some(inputs_evaluations) = execution_context
                .commands_inputs_evaluations_results
                .get(signing_construct_did)
            {
                for input in command_instance.specification.inputs.iter() {
                    let Some(value) = inputs_evaluations.inputs.get_value(&input.name) else {
                        continue;
                    };
                    let is_input_sensitive = input.sensitive;

                    let _should_values_be_hashed = is_construct_sensitive || is_input_sensitive;

                    let value_pre_evaluation = command_instance
                        .get_expression_from_input(input)
                        .unwrap()
                        .map(|e| e.to_string().trim().to_string());

                    inputs.push(CommandInputSnapshot {
                        name: input.name.clone(),
                        value_pre_evaluation,
                        value_post_evaluation: value.clone(),
                        critical: true,
                    });
                }
            }

            let mut outputs: Vec<CommandOutputSnapshot> = vec![];
            if let Some(outputs_results) = execution_context
                .commands_execution_results
                .get(signing_construct_did)
            {
                for output in command_instance.specification.outputs.iter() {
                    let Some(value) = outputs_results.outputs.get(&output.name) else {
                        continue;
                    };
                    outputs.push(CommandOutputSnapshot {
                        name: output.name.clone(),
                        value: value.clone(),
                        signed: false, // todo
                    });
                }
            }

            let downstream_constructs_dids = downstream_constructs_dids
                .iter()
                .map(|c| c.clone())
                .collect();

            snapshot.signing_commands.push(SigningCommandSnapshot {
                package_did: command_instance.package_id.did(),
                construct_did: signing_construct_did.clone(),
                construct_type: signing_construct_id.construct_type.clone(),
                construct_name: signing_construct_id.construct_name.clone(),
                construct_path: signing_construct_id.construct_location.to_string(),
                construct_addon: None,
                downstream_constructs_dids,
                inputs,
                outputs,
            });
        }

        for construct_did in execution_context.order_for_commands_execution.iter() {
            let command_instance = match execution_context.commands_instances.get(&construct_did) {
                Some(entry) => entry,
                None => {
                    continue;
                }
            };

            let construct_id = workspace_context.constructs.get(construct_did).unwrap();

            let critical = execution_context.signed_commands.contains(construct_did);

            let mut inputs = vec![];
            if let Some(inputs_evaluations) = execution_context
                .commands_inputs_evaluations_results
                .get(construct_did)
            {
                for input in command_instance.specification.inputs.iter() {
                    let Some(value) = inputs_evaluations.inputs.get_value(&input.name) else {
                        continue;
                    };
                    let value_pre_evaluation = command_instance
                        .get_expression_from_input(input)
                        .unwrap()
                        .map(|e| e.to_string().trim().to_string());

                    inputs.push(CommandInputSnapshot {
                        name: input.name.clone(),
                        value_pre_evaluation,
                        value_post_evaluation: value.clone(),
                        critical,
                    });
                }
            }

            let mut outputs: Vec<CommandOutputSnapshot> = vec![];
            if let Some(outputs_results) = execution_context
                .commands_execution_results
                .get(construct_did)
            {
                for output in command_instance.specification.outputs.iter() {
                    let Some(value) = outputs_results.outputs.get(&output.name) else {
                        continue;
                    };
                    outputs.push(CommandOutputSnapshot {
                        name: output.name.clone(),
                        value: value.clone(),
                        signed: false, // todo
                    });
                }
            }

            let mut upstream_constructs_dids = vec![];
            if let Some(deps) = execution_context
                .signed_commands_upstream_dependencies
                .get(construct_did)
            {
                for construct_did in deps.iter() {
                    upstream_constructs_dids.push(construct_did.clone());
                }
            }

            snapshot.commands.push(CommandSnapshot {
                package_did: command_instance.package_id.did(),
                construct_did: construct_did.clone(),
                construct_type: construct_id.construct_type.clone(),
                construct_name: construct_id.construct_name.clone(),
                construct_path: construct_id.construct_location.to_string(),
                construct_addon: None,
                upstream_constructs_dids,
                inputs,
                outputs,
            });
        }
        snapshot
    }

    pub fn diff(&self, mut old: RunbookExecutionSnapshot, mut new: RunbookExecutionSnapshot) {
        let mut old_ref = old.clone();
        let mut new_ref = new.clone();
        // let mut refs = vec![];

        let mut diffs = vec![];
        let empty_string = "".to_string();
        TextDiff::from_lines(&old.name, &new.name);
        diffs.push(evaluated_diff(
            None,
            TextDiff::from_lines(&old.name, &new.name),
            format!("Runbook's name updated"),
            false,
        ));
        diffs.push(evaluated_diff(
            None,
            TextDiff::from_lines(
                old.org.as_ref().unwrap_or(&empty_string),
                &new.org.as_ref().unwrap_or(&empty_string),
            ),
            format!("Runbook's name updated"),
            false,
        ));
        diffs.push(evaluated_diff(
            None,
            TextDiff::from_lines(
                old.workspace.as_ref().unwrap_or(&empty_string),
                new.workspace.as_ref().unwrap_or(&empty_string),
            ),
            format!("Runbook's name updated"),
            false,
        ));

        // if changes, we should recompute some temporary ids for packages and constructs
        let mut old_signing_commands = old_ref.signing_commands.clone();
        let mut new_signing_commands = new_ref.signing_commands.clone();

        old_signing_commands.sort_by(|a, b| a.construct_did.cmp(&b.construct_did));
        new_signing_commands.sort_by(|a, b| a.construct_did.cmp(&b.construct_did));

        let old_signing_commands_dids = old_signing_commands
            .iter()
            .map(|c| c.construct_did.to_string())
            .collect::<Vec<_>>();
        let new_signing_commands_dids = new_signing_commands
            .iter()
            .map(|c| c.construct_did.to_string())
            .collect::<Vec<_>>();
        let signing_command_sequence_changes = capture_diff_slices(
            Algorithm::Myers,
            &old_signing_commands_dids,
            &new_signing_commands_dids,
        );

        let mut comparable_signing_constructs_list = vec![];
        for change in signing_command_sequence_changes.iter() {
            match change {
                DiffOp::Equal {
                    old_index,
                    new_index,
                    len: _,
                } => {
                    for i in 0..old_signing_commands_dids.len() {
                        comparable_signing_constructs_list.push((i, i));
                    }
                }
                DiffOp::Delete {
                    old_index,
                    old_len: _,
                    new_index,
                } => {
                    comparable_signing_constructs_list.push((*old_index, *new_index));
                }
                DiffOp::Insert {
                    old_index,
                    new_index,
                    new_len: _,
                } => {
                    comparable_signing_constructs_list.push((*old_index, *new_index));
                }
                DiffOp::Replace {
                    old_index,
                    old_len: _,
                    new_index,
                    new_len: _,
                } => {
                    comparable_signing_constructs_list.push((*old_index, *new_index));
                }
            }
        }

        for (old_index, new_index) in comparable_signing_constructs_list.into_iter() {
            let (old_signing_command, new_signing_command) = match (
                old_signing_commands.get(old_index),
                new_signing_commands.get(new_index),
            ) {
                (Some(old), Some(new)) => (old, new),
                _ => continue,
            };

            // Construct name
            diffs.push(evaluated_diff(
                Some(old_signing_command.construct_did.clone()),
                TextDiff::from_lines(
                    old_signing_command.construct_name.as_str(),
                    new_signing_command.construct_name.as_str(),
                ),
                format!("Signing command's name updated"),
                false,
            ));

            // Construct path
            diffs.push(evaluated_diff(
                Some(old_signing_command.construct_did.clone()),
                TextDiff::from_lines(
                    old_signing_command.construct_path.as_str(),
                    new_signing_command.construct_path.as_str(),
                ),
                format!("Signing command's path updated"),
                false,
            ));

            // Construct driver
            diffs.push(evaluated_diff(
                Some(old_signing_command.construct_did.clone()),
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
            // First: Any new construct?
            let old_signed_commands_dids = old_signing_command
                .downstream_constructs_dids
                .iter()
                .map(|c| c.0.to_string())
                .collect::<Vec<_>>();
            let new_signed_commands_dids = new_signing_command
                .downstream_constructs_dids
                .iter()
                .map(|c| c.0.to_string())
                .collect::<Vec<_>>();
            let signed_command_sequence_changes = capture_diff_slices(
                Algorithm::Myers,
                &old_signed_commands_dids,
                &new_signed_commands_dids,
            );

            let mut comparable_signed_constructs_list = vec![];
            for change in signed_command_sequence_changes.iter() {
                match change {
                    DiffOp::Equal {
                        old_index,
                        new_index,
                        len: _,
                    } => {
                        for i in 0..old_signed_commands_dids.len() {
                            comparable_signed_constructs_list.push((i, i));
                        }
                    }
                    DiffOp::Delete {
                        old_index,
                        old_len: _,
                        new_index,
                    } => {
                        comparable_signed_constructs_list.push((*old_index, *new_index));
                    }
                    DiffOp::Insert {
                        old_index,
                        new_index,
                        new_len: _,
                    } => {
                        comparable_signed_constructs_list.push((*old_index, *new_index));
                    }
                    DiffOp::Replace {
                        old_index,
                        old_len: _,
                        new_index,
                        new_len: _,
                    } => {
                        comparable_signed_constructs_list.push((*old_index, *new_index));
                    }
                }
            }

            for (old_index, new_index) in comparable_signed_constructs_list.into_iter() {
                let (old_construct_did, new_construct_did) = match (
                    old_signing_command
                        .downstream_constructs_dids
                        .get(old_index),
                    new_signing_command
                        .downstream_constructs_dids
                        .get(new_index),
                ) {
                    (Some(old), Some(new)) => (old, new),
                    _ => continue,
                };

                let mut old_construct = old_ref.get_command_with_construct_did(old_construct_did).clone();
                let mut new_construct = new_ref.get_command_with_construct_did(new_construct_did).clone();

                // Construct name
                diffs.push(evaluated_diff(
                    Some(old_signing_command.construct_did.clone()),
                    TextDiff::from_lines(
                        old_construct.construct_name.as_str(),
                        new_construct.construct_name.as_str(),
                    ),
                    format!("Signing command's name updated"),
                    false,
                ));

                // Construct path
                diffs.push(evaluated_diff(
                    Some(old_signing_command.construct_did.clone()),
                    TextDiff::from_lines(
                        old_construct.construct_path.as_str(),
                        new_construct.construct_path.as_str(),
                    ),
                    format!("Signing command's path updated"),
                    false,
                ));

                // Construct driver
                diffs.push(evaluated_diff(
                    Some(old_signing_command.construct_did.clone()),
                    TextDiff::from_lines(
                        old_construct
                            .construct_addon
                            .as_ref()
                            .unwrap_or(&empty_string),
                        new_construct
                            .construct_addon
                            .as_ref()
                            .unwrap_or(&empty_string),
                    ),
                    format!("Signing command's driver updated"),
                    false,
                ));

                old_construct.inputs.sort_by(|a, b| a.name.cmp(&b.name));
                new_construct.inputs.sort_by(|a, b| a.name.cmp(&b.name));

                // Check inputs
                let old_inputs = old_construct
                    .inputs
                    .iter()
                    .map(|i| i.name.to_string())
                    .collect::<Vec<_>>();
                let new_inputs = new_construct
                    .inputs
                    .iter()
                    .map(|i| i.name.to_string())
                    .collect::<Vec<_>>();

                let inputs_sequence_changes =
                    capture_diff_slices(Algorithm::Patience, &old_inputs, &new_inputs);

                let mut comparable_inputs_list = vec![];
                for change in inputs_sequence_changes.iter() {
                    match change {
                        DiffOp::Equal {
                            old_index,
                            new_index,
                            len: _,
                        } => {
                            for i in 0..old_inputs.len() {
                                comparable_inputs_list.push((i, i));
                            }
                        }
                        DiffOp::Delete {
                            old_index,
                            old_len: _,
                            new_index,
                        } => {
                            comparable_inputs_list.push((*old_index, *new_index));
                        }
                        DiffOp::Insert {
                            old_index,
                            new_index,
                            new_len: _,
                        } => {
                            comparable_inputs_list.push((*old_index, *new_index));
                        }
                        DiffOp::Replace {
                            old_index,
                            old_len: _,
                            new_index,
                            new_len: _,
                        } => {
                            comparable_inputs_list.push((*old_index, *new_index));
                        }
                    }
                }

                for (old_index, new_index) in comparable_inputs_list.into_iter() {
                    let (old_input, new_input) = match (
                        old_construct.inputs.get(old_index),
                        new_construct.inputs.get(new_index),
                    ) {
                        (Some(old), Some(new)) => (old, new),
                        _ => continue,
                    };

                    // Input name
                    diffs.push(evaluated_diff(
                        Some(old_signing_command.construct_did.clone()),
                        TextDiff::from_lines(old_input.name.as_str(), new_input.name.as_str()),
                        format!("Signing command's input name updated"),
                        false,
                    ));

                    // Input value_pre_evaluation
                    diffs.push(evaluated_diff(
                        Some(old_signing_command.construct_did.clone()),
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
                        format!("Signing command's input value_pre_evaluation updated"),
                        true,
                    ));
                }
            }
        }

        for change in diffs.iter() {
            if !change.description.is_empty() {
                let fmt_critical = match change.critical {
                    true => "[critical]",
                    false => "[cosmetic]",
                };
    
                println!(
                    "{}: {}\n{}",
                    fmt_critical,
                    change.label,
                    change.description.join("\n")
                );    
            }
        }
    }
}

struct Change {
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
) -> Change where 'a: 'b, 'b: 'a {
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
