use crate::Context;
use juniper_codegen::graphql_object;
use serde_json::json;
use txtx_core::types::ConstructUuid;

#[derive(Clone)]
pub struct ProtocolManifest {
    pub name: String,
    pub runbooks: Vec<RunbookDescription>,
}

#[graphql_object(context = Context)]
impl ProtocolManifest {
    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn runbooks(&self) -> Vec<RunbookDescription> {
        self.runbooks.clone()
    }
}

#[derive(Clone)]
pub struct RunbookDescription {
    pub identifier: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub construct_uuids: Vec<ConstructUuid>,
}

impl RunbookDescription {
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
impl RunbookDescription {
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
