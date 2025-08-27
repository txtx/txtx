use crate::Context;
use juniper_codegen::graphql_object;
use txtx_addon_kit::types::frontend::SupervisorAddonData;

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
    pub supervisor_addon_data: Vec<SupervisorAddonData>,
}

impl RunbookMetadata {
    pub fn new(
        name: &String,
        supervisor_addon_data: &Vec<SupervisorAddonData>,
        description: &Option<String>,
    ) -> Self {
        Self {
            name: name.clone(),
            supervisor_addon_data: supervisor_addon_data.clone(),
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

    pub fn addon_data(&self) -> Vec<GqlSupervisorAddonData> {
        self.supervisor_addon_data.iter().map(|data| GqlSupervisorAddonData(data.clone())).collect()
    }
}

#[derive(Clone)]
pub struct GqlSupervisorAddonData(pub SupervisorAddonData);
#[graphql_object(context = Context)]
impl GqlSupervisorAddonData {
    pub fn addon_name(&self) -> String {
        self.0.addon_name.clone()
    }

    pub fn rpc_api_url(&self) -> Option<String> {
        self.0.rpc_api_url.clone()
    }
}
