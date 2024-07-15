use std::time::{SystemTime, UNIX_EPOCH};

use kit::types::{types::Value, ConstructDid, PackageDid, RunbookId};
use serde::{Deserialize, Serialize};
use serde_json::json;
use similar::{capture_diff_slices, Algorithm, ChangeTag, DiffOp, TextDiff};

use super::{RunbookExecutionContext, RunbookWorkspaceContext};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunbookExecutionSnapshot {
    org: Option<String>,
    workspace: Option<String>,
    name: String,
    ended_at: String,
    packages: Vec<PackageSnapshot>,
    signing_commands: Vec<SigningCommandSnapshot>,
    commands: Vec<CommandSnapshot>,
}

impl RunbookExecutionSnapshot {
    pub fn get_command_with_construct_did(&self, construct_did: &ConstructDid) -> &CommandSnapshot {
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

    pub fn canonize_runbook_execution(
        &self,
        execution_context: &RunbookExecutionContext,
        workspace_context: &RunbookWorkspaceContext,
    ) -> serde_json::Value {
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
        json!(snapshot)
    }

    pub fn diff(&self, old: &RunbookExecutionSnapshot, new: &RunbookExecutionSnapshot) {
        let mut diffs = vec![];
        let empty_string = "".to_string();
        TextDiff::from_lines(&old.name, &new.name);
        diffs.push(ChangesType::Cosmectic(
            TextDiff::from_lines(&old.name, &new.name),
            format!("Runbook's name updated"),
        ));
        diffs.push(ChangesType::Cosmectic(
            TextDiff::from_lines(
                old.org.as_ref().unwrap_or(&empty_string),
                &new.org.as_ref().unwrap_or(&empty_string),
            ),
            format!("Org's name updated"),
        ));
        diffs.push(ChangesType::Cosmectic(
            TextDiff::from_lines(
                old.workspace.as_ref().unwrap_or(&empty_string),
                &new.workspace.as_ref().unwrap_or(&empty_string),
            ),
            format!("Workspace's name updated"),
        ));

        // if changes, we should recompute some temporary ids for packages and constructs
        let old_signing_commands_dids = old
            .signing_commands
            .iter()
            .map(|c| c.construct_did.to_string())
            .collect::<Vec<_>>();
        let new_signing_commands_dids = new
            .signing_commands
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
            let old_signing_command = &old.signing_commands[old_index];
            let new_signing_command = &new.signing_commands[new_index];

            // Construct name
            diffs.push(ChangesType::Cosmectic(
                TextDiff::from_lines(
                    old_signing_command.construct_name.as_str(),
                    new_signing_command.construct_name.as_str(),
                ),
                format!("Signing command's name updated"),
            ));
            // Construct path
            diffs.push(ChangesType::Cosmectic(
                TextDiff::from_lines(
                    old_signing_command.construct_path.as_str(),
                    new_signing_command.construct_path.as_str(),
                ),
                format!("Signing command's path updated"),
            ));
            // Construct driver
            diffs.push(ChangesType::Critical(
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
                let old_construct_did = &old_signing_command.downstream_constructs_dids[old_index];
                let new_construct_did = &new_signing_command.downstream_constructs_dids[new_index];
                let old_construct = old.get_command_with_construct_did(old_construct_did);
                let new_construct = new.get_command_with_construct_did(new_construct_did);

                // Construct name
                diffs.push(ChangesType::Cosmectic(
                    TextDiff::from_lines(
                        old_construct.construct_name.as_str(),
                        new_construct.construct_name.as_str(),
                    ),
                    format!("Signing command's name updated"),
                ));
                // Construct path
                diffs.push(ChangesType::Cosmectic(
                    TextDiff::from_lines(
                        old_construct.construct_path.as_str(),
                        new_construct.construct_path.as_str(),
                    ),
                    format!("Signing command's path updated"),
                ));
                // Construct driver
                diffs.push(ChangesType::Critical(
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
                ));
                // Value pre-evaluation

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
                    let old_input = &old_construct.inputs[old_index];
                    let new_input = &new_construct.inputs[new_index];

                    // Input name
                    diffs.push(ChangesType::Cosmectic(
                        TextDiff::from_lines(old_input.name.as_str(), new_input.name.as_str()),
                        format!("Signing command's input name updated"),
                    ));
                    // Input value_pre_evaluation
                    diffs.push(ChangesType::Cosmectic(
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
                    ));

                    // Input value_post_evaluation
                }
            }
        }

        for diff_type in diffs.into_iter() {
            let (_critical, fmt_critical, diff_results, label) = match diff_type {
                ChangesType::Cosmectic(value, label) => (false, "[cosmetic]", value, label),
                ChangesType::Critical(value, label) => (true, "[critical]", value, label),
            };

            let mut changes = vec![];
            for diff_result in diff_results.iter_all_changes() {
                if let ChangeTag::Equal = diff_result.tag() {
                    continue;
                }
                changes.push(diff_result);
            }

            if changes.is_empty() {
                continue;
            }

            println!("{}: {}", fmt_critical, label);
            for change in changes.into_iter() {
                let sign = match change.tag() {
                    ChangeTag::Delete => "-",
                    ChangeTag::Insert => "+",
                    ChangeTag::Equal => unreachable!(),
                };
                print!("{}{}", sign, change);
            }
        }
    }
}

enum ChangesType<'a, 'b, 'c> {
    Cosmectic(TextDiff<'a, 'b, 'c, str>, String),
    Critical(TextDiff<'a, 'b, 'c, str>, String),
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
