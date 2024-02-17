use crate::Context;
use juniper_codegen::graphql_object;
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

}
