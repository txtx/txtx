use super::RunbookWorkspaceContext;
use kit::types::frontend::ActionItemRequest;
use kit::types::frontend::ActionItemRequestType;
use kit::types::frontend::ActionItemStatus;
use kit::types::frontend::DisplayOutputRequest;
use kit::types::types::Value;
use kit::types::wallets::SigningCommandsState;
use kit::types::ConstructDid;
use kit::types::PackageDid;
use kit::types::RunbookId;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::collections::HashMap;
use txtx_addon_kit::types::commands::CommandInputsEvaluationResult;
use txtx_addon_kit::types::commands::{CommandExecutionResult, CommandInstance};
use txtx_addon_kit::types::wallets::WalletInstance;

#[derive(Debug, Clone)]
pub struct RunbookExecutionContext {
    /// Map of executable commands (input, output, action)
    pub commands_instances: HashMap<ConstructDid, CommandInstance>,
    /// Map of signing commands (wallet)
    pub signing_commands_instances: HashMap<ConstructDid, WalletInstance>,
    /// State of the signing commands states (stateful)
    pub signing_commands_state: Option<SigningCommandsState>,
    /// Results of commands executions
    pub commands_execution_results: HashMap<ConstructDid, CommandExecutionResult>,
    /// Results of commands inputs evaluations
    pub commands_inputs_evaluations_results: HashMap<ConstructDid, CommandInputsEvaluationResult>,
    /// Constructs depending on a given Construct.
    pub commands_dependencies: BTreeMap<ConstructDid, Vec<ConstructDid>>,
    /// Constructs depending on a given Construct performing signing.
    pub signing_commands_dependencies: BTreeMap<ConstructDid, Vec<ConstructDid>>,
    /// Commands execution order.
    pub order_for_commands_execution: Vec<ConstructDid>,
    /// Signing commands initialization order.
    pub order_for_signing_commands_initialization: Vec<ConstructDid>,
}

use std::time::{SystemTime, UNIX_EPOCH};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RunbookExecutionSnapshot {
    org: Option<String>,
    project: Option<String>,
    name: String,
    ended_at: String,
    packages: Vec<PackageSnapshot>,
    signing_commands: Vec<SigningCommandSnapshot>,
    commands: Vec<CommandSnapshot>,
}

impl RunbookExecutionSnapshot {
    pub fn new(runbook_id: &RunbookId) -> Self {
        let ended_at = now_as_string();
        Self {
            org: runbook_id.org.clone(),
            project: runbook_id.project.clone(),
            name: runbook_id.name.clone(),
            ended_at,
            packages: vec![],
            signing_commands: vec![],
            commands: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PackageSnapshot {
    did: PackageDid,
    path: String,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SigningCommandSnapshot {
    package_did: PackageDid,
    construct_did: ConstructDid,
    construct_type: String,
    construct_name: String,
    construct_addon: Option<String>,
    construct_path: String,
    signed_constructs_dids: Vec<ConstructDid>,
    inputs: Vec<CommandInputSnapshot>,
    outputs: Vec<CommandOutputSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CommandSnapshot {
    package_did: PackageDid,
    construct_did: ConstructDid,
    construct_type: String,
    construct_name: String,
    construct_path: String,
    construct_addon: Option<String>,
    inputs: Vec<CommandInputSnapshot>,
    outputs: Vec<CommandOutputSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CommandInputSnapshot {
    pub name: String,
    // pub value_pre_evaluation: Value,
    pub value_post_evaluation: Value,
    pub signed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CommandOutputSnapshot {
    pub name: String,
    pub value: Value,
    pub signed: bool,
}

impl RunbookExecutionContext {
    pub fn new() -> Self {
        Self {
            commands_instances: HashMap::new(),
            signing_commands_instances: HashMap::new(),
            signing_commands_state: Some(SigningCommandsState::new()),
            commands_execution_results: HashMap::new(),
            commands_inputs_evaluations_results: HashMap::new(),
            commands_dependencies: BTreeMap::new(),
            signing_commands_dependencies: BTreeMap::new(),
            order_for_commands_execution: vec![],
            order_for_signing_commands_initialization: vec![],
        }
    }

    pub fn canonize_runbook_execution(
        &self,
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
        for signing_construct_did in self.order_for_signing_commands_initialization.iter() {
            let command_instance = self
                .signing_commands_instances
                .get(&signing_construct_did)
                .unwrap();

            let signing_construct_id = workspace_context
                .constructs
                .get(signing_construct_did)
                .unwrap();

            let mut inputs = vec![];
            if let Some(inputs_evaluations) = self
                .commands_inputs_evaluations_results
                .get(signing_construct_did)
            {
                for input in command_instance.specification.inputs.iter() {
                    let Some(value) = inputs_evaluations.inputs.get_value(&input.name) else {
                        continue;
                    };
                    inputs.push(CommandInputSnapshot {
                        name: input.name.clone(),
                        value_post_evaluation: value.clone(),
                        signed: false, // todo
                    });
                }
            }

            let mut outputs: Vec<CommandOutputSnapshot> = vec![];
            if let Some(outputs_results) =
                self.commands_execution_results.get(signing_construct_did)
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

            let mut signed_constructs_dids = vec![];
            if let Some(deps) = self
                .signing_commands_dependencies
                .get(signing_construct_did)
            {
                for construct_did in deps.iter() {
                    signed_constructs_dids.push(construct_did.clone());
                }
            }

            snapshot.signing_commands.push(SigningCommandSnapshot {
                package_did: command_instance.package_id.did(),
                construct_did: signing_construct_did.clone(),
                construct_type: signing_construct_id.construct_type.clone(),
                construct_name: signing_construct_id.construct_name.clone(),
                construct_path: signing_construct_id.construct_location.to_string(),
                construct_addon: None,
                signed_constructs_dids,
                inputs,
                outputs,
            });
        }

        for construct_did in self.order_for_commands_execution.iter() {
            let command_instance = match self.commands_instances.get(&construct_did) {
                Some(entry) => entry,
                None => {
                    continue;
                }
            };

            let construct_id = workspace_context.constructs.get(construct_did).unwrap();

            // let mut constructs_dids = vec![];
            // if let Some(deps) = self.signing_commands_dependencies.get(construct_did) {
            //     for construct_did in deps.iter() {
            //         signed_constructs_dids.push(construct_did.clone());
            //     }
            // }

            let mut inputs = vec![];
            if let Some(inputs_evaluations) =
                self.commands_inputs_evaluations_results.get(construct_did)
            {
                for input in command_instance.specification.inputs.iter() {
                    let Some(value) = inputs_evaluations.inputs.get_value(&input.name) else {
                        continue;
                    };
                    inputs.push(CommandInputSnapshot {
                        name: input.name.clone(),
                        value_post_evaluation: value.clone(),
                        signed: false, // todo
                    });
                }
            }

            let mut outputs: Vec<CommandOutputSnapshot> = vec![];
            if let Some(outputs_results) = self.commands_execution_results.get(construct_did) {
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

            snapshot.commands.push(CommandSnapshot {
                package_did: command_instance.package_id.did(),
                construct_did: construct_did.clone(),
                construct_type: construct_id.construct_type.clone(),
                construct_name: construct_id.construct_name.clone(),
                construct_path: construct_id.construct_location.to_string(),
                construct_addon: None,
                inputs,
                outputs,
            });
        }

        json!(snapshot)
    }

    pub fn collect_outputs_constructs_results(&self) -> BTreeMap<String, Vec<ActionItemRequest>> {
        let mut action_items = BTreeMap::new();

        for construct_did in self.order_for_commands_execution.iter() {
            let Some(command_instance) = self.commands_instances.get(&construct_did) else {
                // runtime_ctx.addons.index_command_instance(namespace, package_did, block)
                continue;
            };

            if command_instance
                .specification
                .name
                .to_lowercase()
                .eq("output")
            {
                let Some(execution_result) = self.commands_execution_results.get(&construct_did)
                else {
                    return action_items;
                };

                let Some(value) = execution_result.outputs.get("value") else {
                    unreachable!()
                };

                action_items
                    .entry(command_instance.get_group())
                    .or_insert_with(Vec::new)
                    .push(ActionItemRequest::new(
                        &Some(construct_did.clone()),
                        &command_instance.name,
                        None,
                        ActionItemStatus::Todo,
                        ActionItemRequestType::DisplayOutput(DisplayOutputRequest {
                            name: command_instance.name.to_string(),
                            description: None,
                            value: value.clone(),
                        }),
                        "output".into(),
                    ));
            }
        }

        action_items
    }
}
