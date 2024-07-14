use kit::types::diagnostics::Diagnostic;
use kit::types::{ConstructDid, PackageDid, PackageId, RunbookId};
use std::collections::{BTreeMap, HashMap};
use txtx_addon_kit::helpers::fs::FileLocation;
use workspace_context::ConstructInstanceType;

mod execution_context;
mod resolution_context;
mod workspace_context;

pub use execution_context::RunbookExecutionContext;
pub use resolution_context::RunbookResolutionContext;
pub use workspace_context::RunbookWorkspaceContext;

use crate::types::PreConstructData;

pub struct RunbookSources {
    /// Map of files required to construct the runbook
    pub tree: HashMap<FileLocation, (String, String)>,
}

impl RunbookSources {
    pub fn new() -> Self {
        Self {
            tree: HashMap::new(),
        }
    }

    pub fn add_source(&mut self, name: String, location: FileLocation, content: String) {
        self.tree.insert(location, (name, content));
    }
}

#[derive(Debug, Clone)]
pub struct Runbook {
    /// The resolution context contains all the data related to source code analysis and DAG construction
    pub resolution_context: RunbookResolutionContext,
    /// The execution context contains all the data related to the execution of the runbook
    pub execution_context: RunbookExecutionContext,
    /// The workspace context keeps track of packages and constructs reachable
    pub workspace_context: RunbookWorkspaceContext,
    /// Diagnostics collected over time, until hitting a fatal error
    pub diagnostics: Vec<Diagnostic>,
}

impl Runbook {
    pub fn new(runbook_id: RunbookId, description: Option<String>) -> Self {
        Self {
            workspace_context: RunbookWorkspaceContext::new(runbook_id, description),
            resolution_context: RunbookResolutionContext::new(),
            execution_context: RunbookExecutionContext::new(),
            diagnostics: vec![],
        }
    }

    pub fn runbook_id(&self) -> RunbookId {
        self.workspace_context.runbook_id.clone()
    }

    pub fn index_package(&mut self, package_id: &PackageId) -> PackageDid {
        self.workspace_context.index_package(package_id);
        self.resolution_context.index_package(package_id);
        package_id.did()
    }

    pub fn index_environment_variables(
        &mut self,
        environment_variables: &BTreeMap<String, String>,
    ) {
        for (key, value) in environment_variables.into_iter() {
            let construct_did = self
                .workspace_context
                .index_environment_variable(key, value);
            self.resolution_context
                .index_environment_variable(&construct_did);
        }
    }

    pub fn index_construct(
        &mut self,
        construct_name: String,
        construct_location: FileLocation,
        construct_data: PreConstructData,
        package_id: &PackageId,
    ) -> ConstructDid {
        let (construct_id, construct_instance_type) = self.workspace_context.index_construct(
            construct_name,
            construct_location,
            construct_data,
            package_id,
        );
        let construct_did = construct_id.did();
        self.resolution_context.index_construct(&construct_did);
        match construct_instance_type {
            ConstructInstanceType::Executable(instance) => {
                self.execution_context
                    .commands_instances
                    .insert(construct_did.clone(), instance);
            }
            ConstructInstanceType::Signing(instance) => {
                self.execution_context
                    .signing_commands_instances
                    .insert(construct_did.clone(), instance);
            }
            ConstructInstanceType::Import => {}
        }
        construct_did
    }
    // add attribute "tainting_change"

    // -> Order the nodes
    // -> Compute canonical id for each node
    //      -> traverse all the inputs, same order, only considering the one with "tainting_change" set to true
    // The edges should be slightly differently created: only if a tainting_change is at stake. 5
    // -> Compute canonical id for each edge (?)
    //      ->
}
