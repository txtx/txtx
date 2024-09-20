use crate::Context;
use juniper_codegen::graphql_object;

#[derive(Clone)]
pub struct WorkspaceManifest {
    pub name: String,
    pub runbooks: Vec<RunbookMetadata>,
}

#[graphql_object(context = Context)]
impl WorkspaceManifest {
    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn runbooks(&self) -> Vec<RunbookMetadata> {
        self.runbooks.clone()
    }
}

#[derive(Clone)]
pub struct RunbookMetadata {
    pub name: String,
    pub description: Option<String>,
    pub registered_addons: Vec<String>,
}

impl RunbookMetadata {
    pub fn new(
        name: &String,
        registered_addons: &Vec<String>,
        description: &Option<String>,
    ) -> Self {
        Self {
            name: name.clone(),
            registered_addons: registered_addons.clone(),
            description: description.clone(),
        }
    }
}

#[graphql_object(context = Context)]
impl RunbookMetadata {
    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn description(&self) -> Option<String> {
        self.description.clone()
    }

    pub fn registered_addons(&self) -> Vec<String> {
        self.registered_addons.clone()
    }
}
