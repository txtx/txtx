use crate::Context;
use juniper_codegen::graphql_object;

#[derive(Clone)]
pub struct WorkspaceManifest {
    pub name: String,
    pub runbooks: Vec<RunbookDescription>,
}

#[graphql_object(context = Context)]
impl WorkspaceManifest {
    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn runbooks(&self) -> Vec<RunbookDescription> {
        self.runbooks.clone()
    }
}

#[derive(Clone)]
pub struct RunbookDescription {
    pub name: String,
    pub description: Option<String>,
}

impl RunbookDescription {
    pub fn new(name: &String, description: &Option<String>) -> Self {
        Self {
            name: name.clone(),
            description: description.clone(),
        }
    }
}

#[graphql_object(context = Context)]
impl RunbookDescription {
    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn description(&self) -> Option<String> {
        self.description.clone()
    }
}
