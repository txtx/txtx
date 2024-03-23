use crate::Context;
use juniper_codegen::graphql_object;
use serde_json::json;
use txtx_core::{
    kit::types::{commands::CommandExecutionResult, diagnostics::Diagnostic},
    types::{CommandInstance, ConstructUuid},
};

pub struct Construct {
    pub uuid: ConstructUuid,
    pub data: CommandInstance,
    pub result: Option<Result<CommandExecutionResult, Diagnostic>>,
}

impl Construct {
    pub fn new(
        uuid: &ConstructUuid,
        data: &CommandInstance,
        result: Option<Result<CommandExecutionResult, Diagnostic>>,
    ) -> Self {
        Self {
            uuid: uuid.clone(),
            data: data.clone(),
            result: result.clone(),
        }
    }
}

#[graphql_object(context = Context)]
impl Construct {
    pub fn uuid(&self) -> String {
        self.uuid.value().to_string()
    }

    pub fn id(&self) -> String {
        self.data.name.to_string()
    }

    pub fn output(&self) -> String {
        let result = self.result.clone().unwrap();
        let result = json!(result);
        result.to_string()
    }
}
