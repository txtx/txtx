use crate::Context;
use juniper_codegen::graphql_object;

pub struct ManualDescription {
    pub identifier: String,
    pub name: Option<String>,
    pub description: Option<String>,
}

impl ManualDescription {
    pub fn new(identifier: &str, name: &Option<String>, description: &Option<String>) -> Self {
        Self {
            identifier: identifier.to_string(),
            name: name.clone(),
            description: description.clone(),
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
}
