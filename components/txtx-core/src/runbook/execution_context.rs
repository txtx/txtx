use std::collections::BTreeMap;
use std::collections::HashMap;

use kit::types::frontend::ActionItemRequest;
use kit::types::frontend::ActionItemRequestType;
use kit::types::frontend::ActionItemStatus;
use kit::types::frontend::DisplayOutputRequest;
use kit::types::types::Value;
use kit::types::wallets::SigningCommandsState;
use kit::types::ConstructDid;
use serde_json::json;
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

    pub fn serialize_execution(&self) -> serde_json::Value {
        let mut serialized_nodes = vec![];

        for construct_did in self.order_for_commands_execution.iter() {
            let Some(command_instance) = self.commands_instances.get(&construct_did) else {
                // runtime_ctx.addons.index_command_instance(namespace, package_did, block)
                continue;
            };

            let inputs_results = &self
                .commands_inputs_evaluations_results
                .get(&construct_did)
                .unwrap()
                .inputs;

            let outputs_results = &self
                .commands_execution_results
                .get(&construct_did)
                .unwrap()
                .outputs;

            let inputs = command_instance.specification.inputs.iter().map(|i| {
                let value = match (inputs_results.get_value(&i.name), i.optional) {
                    (Some(v), _) => {
                        v.clone()
                    },
                    (None, true) => {
                        Value::null()
                    },
                    _ => panic!("corrupted execution, required input {} missing post execution - investigation required", i.name)
                };
                json!({
                    "name": i.name,
                    "type": value.get_type(),
                    "value": value.to_string()
                })
            }).collect::<Vec<_>>();

            let outputs = command_instance.specification.outputs.iter().map(|o| {
                let output_result = match outputs_results.get(&o.name) {
                    Some(v) => v,
                    None => panic!("corrupted execution, required output {} missing post execution - investigation required", o.name)
                };
                json!({
                    "name": o.name,
                    "value_type": output_result.get_type(),
                    "value": output_result.to_string()
                })
            }).collect::<Vec<_>>();

            serialized_nodes.push(json!({
                "action": command_instance.specification.matcher,
                "inputs": inputs,
                "outputs": outputs,
            }));
        }

        json!({
            "nodes": serialized_nodes
        })
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
