use serde::de::value::Error;

use crate::Context;
use juniper_codegen::graphql_object;
use serde::{de::IntoDeserializer, Deserialize};
use serde_json::json;
use txtx_core::{
    eval::{get_ordered_nodes, is_child_of_node},
    types::{ConstructUuid, Manual},
};

#[derive(Clone)]
pub struct ProtocolManifest {
    pub name: String,
    pub manuals: Vec<ManualDescription>,
}

#[graphql_object(context = Context)]
impl ProtocolManifest {
    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn manuals(&self) -> Vec<ManualDescription> {
        self.manuals.clone()
    }
}

#[derive(Clone)]
pub struct ManualDescription {
    pub identifier: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub construct_uuids: Vec<ConstructUuid>,
}

impl ManualDescription {
    pub fn new(
        identifier: &str,
        name: &Option<String>,
        description: &Option<String>,
        construct_uuids: &Vec<ConstructUuid>,
    ) -> Self {
        Self {
            identifier: identifier.to_string(),
            name: name.clone(),
            description: description.clone(),
            construct_uuids: construct_uuids.clone(),
        }
    }
}

#[graphql_object(context = Context)]
impl ManualDescription {
    pub fn uuid(&self) -> String {
        self.identifier.clone()
    }

    pub fn name(&self) -> Option<String> {
        self.name.clone()
    }

    pub fn description(&self) -> Option<String> {
        self.description.clone()
    }

    pub fn construct_uuids(&self) -> String {
        let json = json!(self.construct_uuids);
        json.to_string()
    }
}

pub struct GqlManual {
    pub name: String,
    pub data: Manual,
}

impl GqlManual {
    pub fn new(name: String, data: Manual) -> GqlManual {
        Self {
            name: name.clone(),
            data: data.clone(),
        }
    }
}

#[graphql_object(context = Context)]
impl GqlManual {
    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn description(&self) -> Option<String> {
        self.data.description.clone()
    }

    pub fn uuid(&self) -> String {
        self.name.clone()
    }

    pub fn data(&self) -> Result<String, String> {
        let mut data = vec![];
        let ordered_nodes =
            get_ordered_nodes(self.data.graph_root, self.data.constructs_graph.clone());
        let graph = self.data.constructs_graph.clone();

        println!("ordered nodes {:?}", ordered_nodes);
        for (i, node) in ordered_nodes.into_iter().enumerate() {
            let uuid = graph
                .node_weight(node)
                .expect("unable to retrieve construct");
            let construct_uuid = ConstructUuid::Local(uuid.clone());

            let Some(command_instance) = self.data.commands_instances.get(&construct_uuid) else {
                continue;
            };

            let is_child_of_root = is_child_of_node(self.data.graph_root, node, &graph);
            println!(
                "{} {} is editable: {}",
                i, command_instance.name, is_child_of_root
            );

            let constructs_execution_results =
                match self.data.constructs_execution_results.get(&construct_uuid) {
                    None => None,
                    Some(result) => match result {
                        Ok(result) => Some(
                            serde_json::to_value(result)
                                .map_err(|e| format!("failed to serialize manual data {e}"))?,
                        ),
                        Err(e) => Some(json!({"error": e})),
                    },
                };
            let command_inputs_evaluation_results = self
                .data
                .command_inputs_evaluation_results
                .get(&construct_uuid);

            data.push(json!({
                "readonly": !is_child_of_root,
                "index": i,
                "constructUuid": construct_uuid,
                "commandInstance": command_instance,
                "commandInputsEvaluationResult": command_inputs_evaluation_results,
                "constructsExecutionResult": constructs_execution_results
            }));
        }

        serde_json::to_string(&data).map_err(|e| format!("failed to serialize manual data {e}"))
    }

    pub fn command_instance_state(&self, construct_uuid_string: String) -> Result<String, String> {
        let construct_uuid =
            ConstructUuid::deserialize(construct_uuid_string.clone().into_deserializer())
                .map_err(|e: Error| e.to_string())?;

        let result = match self.data.commands_instances.get(&construct_uuid) {
            Some(command_instance) => {
                let state_machine = command_instance.state.lock().map_err(|e| e.to_string())?; // todo: handle error
                json!({"state": state_machine.state() })
            }
            None => json!({}),
        };

        serde_json::to_string(&result).map_err(|e| {
            format!(
                "failed to serialize command instance {} state {}",
                construct_uuid_string, e
            )
        })
    }
}
