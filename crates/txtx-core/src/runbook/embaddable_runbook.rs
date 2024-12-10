use kit::types::stores::ValueStore;
use kit::types::RunbookId;

use super::{RunbookExecutionContext, RunbookGraphContext, RunbookWorkspaceContext};

pub struct EmbeddableRunbook {
    pub runbook_id: RunbookId,
    pub description: Option<String>,
    /// The resolution context contains all the data related to source code analysis and DAG construction
    pub graph_context: RunbookGraphContext,
    /// The execution context contains all the data related to the execution of the runbook
    pub execution_context: RunbookExecutionContext,
    /// The workspace context keeps track of packages and constructs reachable
    pub workspace_context: RunbookWorkspaceContext,
    /// The set of environment variables used during the execution
    pub top_level_inputs: ValueStore,
    /// The evaluated inputs to this flow
    pub evaluated_inputs: ValueStore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddableRunbookSpecification {
    runbook_id: RunbookId,
    inputs: IndexMap<String, EmbeddableRunbookInput>,
    /// Schema version
    version: u32,
    /// Snapshot of the evaluated runbook defaults, indexed by package did and addon id
    pub addon_defaults_fingerprints: IndexMap<PackageDid, IndexMap<String, IndexMap<String, Did>>>,
    /// Snapshot of the packages pulled by the runbook
    pub packages: IndexMap<PackageDid, PackageSnapshot>,
    /// Snapshot of the signing commands evaluations
    pub signers: IndexMap<ConstructDid, SigningCommandSnapshot>,
    /// Snapshot of the commands evaluations
    pub commands: IndexMap<ConstructDid, CommandSnapshot>,
    /// Snapshot of the inputs provided by the manifest and CLI
    top_level_inputs_fingerprints: IndexMap<String, Did>,
}

pub enum EmbeddableRunbookInput {
    Value(Value),
    Signer,
}
