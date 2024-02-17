use crate::Context;
use juniper_codegen::graphql_object;
use serde_json::json;
use txtx_core::types::{ConstructUuid, Manual};

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
    pub fn id(&self) -> String {
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

    pub fn data(&self) -> Result<String, String> {
        let mut data = vec![];
        for (construct_uuid, command_instance) in self.data.commands_instances.iter() {
            let constructs_execution_results =
                self.data.constructs_execution_results.get(&construct_uuid);
            let command_inputs_evaluation_results = self
                .data
                .command_inputs_evaluation_results
                .get(&construct_uuid);
            data.push(json!({
                "constructUuid": construct_uuid,
                "commandInstance": command_instance,
                "commandInputsEvaluationResult": command_inputs_evaluation_results,
                "constructsExecutionResult": constructs_execution_results
            }));
        }
        serde_json::to_string(&data).map_err(|e| format!("failed to serialize manual data {e}"))
    }
}
