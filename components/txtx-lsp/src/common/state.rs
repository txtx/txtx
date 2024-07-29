use lsp_types::{
    CompletionItem, DocumentSymbol, Hover, MessageType, Position, Range, SignatureHelp,
};
use std::borrow::BorrowMut;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::vec;
use txtx_core::kit::helpers::fs::{FileAccessor, FileLocation};
use txtx_core::kit::types::diagnostics::{Diagnostic as TxtxDiagnostic, DiagnosticLevel};
use txtx_core::kit::types::RunbookId;
use txtx_core::manifest::WorkspaceManifest;

use super::requests::capabilities::InitializationOptions;

#[derive(Debug, Clone, PartialEq)]
pub struct ActiveRunbookData {
    source: String,
}

impl ActiveRunbookData {
    pub fn new(source: &str) -> Self {
        ActiveRunbookData {
            source: source.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunbookState {
    runbook_id: RunbookId,
    errors: Vec<TxtxDiagnostic>,
    warnings: Vec<TxtxDiagnostic>,
    notes: Vec<TxtxDiagnostic>,
    location: FileLocation,
}

impl RunbookState {
    pub fn new(
        runbook_id: RunbookId,
        mut diags: Vec<TxtxDiagnostic>,
        location: FileLocation,
    ) -> RunbookState {
        let mut errors = vec![];
        let mut warnings = vec![];
        let mut notes = vec![];

        for diag in diags.drain(..) {
            match diag.level {
                DiagnosticLevel::Error => {
                    errors.push(diag);
                }
                DiagnosticLevel::Warning => {
                    warnings.push(diag);
                }
                DiagnosticLevel::Note => {
                    notes.push(diag);
                }
            }
        }

        RunbookState {
            runbook_id,
            errors,
            warnings,
            notes,
            location,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RunbookMetadata {
    pub base_location: FileLocation,
    pub manifest_location: FileLocation,
    pub relative_path: String,
}

#[derive(Clone, Default, Debug)]
pub struct EditorState {
    pub workspaces: HashMap<FileLocation, WorkspaceState>,
    pub runbooks_lookup: HashMap<FileLocation, RunbookMetadata>,
    pub active_runbooks: HashMap<FileLocation, ActiveRunbookData>,
    pub settings: InitializationOptions,
}

impl EditorState {
    pub fn new() -> EditorState {
        EditorState {
            workspaces: HashMap::new(),
            runbooks_lookup: HashMap::new(),
            active_runbooks: HashMap::new(),
            settings: InitializationOptions::default(),
        }
    }

    pub fn index_workspace(&mut self, manifest_location: FileLocation, workspace: WorkspaceState) {
        let mut base_location = manifest_location.clone();

        match base_location.borrow_mut() {
            FileLocation::FileSystem { path } => {
                let mut parent = path.clone();
                parent.pop();
                parent.pop();
            }
            FileLocation::Url { url } => {
                let mut segments = url
                    .path_segments_mut()
                    .expect("could not find root location");
                segments.pop();
                segments.pop();
            }
        };

        for (runbook_location, runbook_state) in workspace.runbooks.iter() {
            let relative_path = runbook_location
                .get_relative_path_from_base(&base_location)
                .expect("could not find relative location");

            self.runbooks_lookup.insert(
                runbook_location.clone(),
                RunbookMetadata {
                    base_location: base_location.clone(),
                    manifest_location: manifest_location.clone(),
                    relative_path,
                },
            );
        }
        self.workspaces.insert(manifest_location, workspace);
    }

    pub fn clear_workspace(&mut self, manifest_location: &FileLocation) {
        if let Some(workspace) = self.workspaces.remove(manifest_location) {
            for (runbook_location, _) in workspace.runbooks.iter() {
                self.runbooks_lookup.remove(runbook_location);
            }
        }
    }

    pub fn clear_workspace_associated_with_runbook(
        &mut self,
        runbook_location: &FileLocation,
    ) -> Option<FileLocation> {
        match self.runbooks_lookup.get(runbook_location) {
            Some(runbook_metadata) => {
                let manifest_location = runbook_metadata.manifest_location.clone();
                self.clear_workspace(&manifest_location);
                Some(manifest_location)
            }
            None => None,
        }
    }

    pub fn get_completion_items_for_runbook(
        &self,
        runbook_location: &FileLocation,
        position: &Position,
    ) -> Vec<lsp_types::CompletionItem> {
        // let active_runbook = match self.active_runbooks.get(runbook_location) {
        //     Some(contract) => contract,
        //     None => return vec![],
        // };


        // let modules = self
        //     .runbooks_lookup
        //     .get(runbook_location)
        //     .and_then(|d| self.workspaces.get(&d.manifest_location))
        //     .map(|p| p.get_contract_calls_for_contract(runbook_location))
        //     .unwrap_or_default();

        // let expressions = active_runbook.expressions.as_ref();
        // let active_contract_defined_data =
        // ContractDefinedData::new(expressions.unwrap_or(&vec![]), position);

        // build_completion_item_list(
        //     &active_runbook.clarity_version,
        //     expressions.unwrap_or(&vec![]),
        //     &Position {
        //         line: position.line + 1,
        //         character: position.character + 1,
        //     },
        //     &active_contract_defined_data,
        //     contract_calls,
        //     should_wrap,
        //     self.settings.completion_include_native_placeholders,
        // )
        vec![lsp_types::CompletionItem {
            label: "completion".into(),
            ..Default::default()
        }]
    }

    pub fn get_document_symbols_for_runbook(
        &self,
        runbook_location: &FileLocation,
    ) -> Vec<DocumentSymbol> {
        vec![]
    }

    pub fn get_definition_location(
        &self,
        runbook_location: &FileLocation,
        position: &Position,
    ) -> Option<lsp_types::Location> {
        // let runbook = self.active_runbooks.get(runbook_location)?;
        // let _position = Position {
        //     line: position.line + 1,
        //     character: position.character + 1,
        // };
        None
    }

    pub fn get_hover_data(
        &self,
        runbook_location: &FileLocation,
        position: &lsp_types::Position,
    ) -> Option<Hover> {
        // let runbook = self.active_runbooks.get(runbook_location)?;
        // let position = Position {
        //     line: position.line + 1,
        //     character: position.character + 1,
        // };
        // let documentation = get_expression_documentation(
        //     &position,
        //     contract.clarity_version,
        //     contract.expressions.as_ref()?,
        // )?;

        Some(Hover {
            contents: lsp_types::HoverContents::Markup(lsp_types::MarkupContent {
                kind: lsp_types::MarkupKind::Markdown,
                value: "hover".to_string(),
            }),
            range: None,
        })
    }

    pub fn get_signature_help(
        &self,
        runbook_location: &FileLocation,
        position: &lsp_types::Position,
        active_signature: Option<u32>,
    ) -> Option<SignatureHelp> {
        let runbook = self.active_runbooks.get(runbook_location)?;
        let position = Position {
            line: position.line + 1,
            character: position.character + 1,
        };
        // let signatures = get_signatures(contract, &position)?;

        // Some(SignatureHelp {
        //     signatures,
        //     active_signature,
        //     active_parameter: None,
        // })
        None
    }

    pub fn get_aggregated_diagnostics(
        &self,
    ) -> (
        Vec<(FileLocation, Vec<TxtxDiagnostic>)>,
        Option<(MessageType, String)>,
    ) {
        let mut runbooks = vec![];
        let mut erroring_files = HashSet::new();
        let mut warning_files = HashSet::new();

        for (_, workspace_state) in self.workspaces.iter() {
            for (runbook_url, state) in workspace_state.runbooks.iter() {
                let mut diags = vec![];

                let RunbookMetadata { relative_path, .. } = self
                    .runbooks_lookup
                    .get(runbook_url)
                    .expect("contract not in lookup");

                // Convert and collect errors
                if !state.errors.is_empty() {
                    erroring_files.insert(relative_path.clone());
                    for error in state.errors.iter() {
                        diags.push(error.clone());
                    }
                }

                // Convert and collect warnings
                if !state.warnings.is_empty() {
                    warning_files.insert(relative_path.clone());
                    for warning in state.warnings.iter() {
                        diags.push(warning.clone());
                    }
                }

                // Convert and collect notes
                for note in state.notes.iter() {
                    diags.push(note.clone());
                }
                runbooks.push((runbook_url.clone(), diags));
            }
        }

        let tldr = match (erroring_files.len(), warning_files.len()) {
            (0, 0) => None,
            (0, _warnings) => Some((
                MessageType::WARNING,
                format!(
                    "Warning detected in following contracts: {}",
                    warning_files.into_iter().collect::<Vec<_>>().join(", ")
                ),
            )),
            (_errors, 0) => Some((
                MessageType::ERROR,
                format!(
                    "Errors detected in following contracts: {}",
                    erroring_files.into_iter().collect::<Vec<_>>().join(", ")
                ),
            )),
            (_errors, _warnings) => Some((
                MessageType::ERROR,
                format!(
                    "Errors and warnings detected in following contracts: {}",
                    erroring_files.into_iter().collect::<Vec<_>>().join(", ")
                ),
            )),
        };

        (runbooks, tldr)
    }

    pub fn insert_active_runbook(&mut self, runbook_location: FileLocation, source: &str) {
        let runbook = ActiveRunbookData::new(source);
        self.active_runbooks.insert(runbook_location, runbook);
    }

    pub fn update_active_contract(
        &mut self,
        runbook_location: &FileLocation,
        source: &str,
        with_definitions: bool,
    ) -> Result<(), String> {
        let runbook = self
            .active_runbooks
            .get_mut(runbook_location)
            .ok_or("contract not in active_contracts")?;
        // runbook.update_sources(source, with_definitions);
        Ok(())
    }
}

#[derive(Clone, Default, Debug)]
pub struct WorkspaceState {
    runbooks: HashMap<FileLocation, RunbookState>,
    locations_lookup: HashMap<RunbookId, FileLocation>,
}

impl WorkspaceState {
    pub fn new() -> Self {
        WorkspaceState::default()
    }

    // pub fn consolidate(
    //     &mut self,
    //     locations: &mut HashMap<QualifiedContractIdentifier, FileLocation>,
    //     asts: &mut BTreeMap<QualifiedContractIdentifier, ContractAST>,
    //     deps: &mut BTreeMap<QualifiedContractIdentifier, DependencySet>,
    //     diags: &mut HashMap<QualifiedContractIdentifier, Vec<ClarityDiagnostic>>,
    //     definitions: &mut HashMap<QualifiedContractIdentifier, HashMap<ClarityName, Range>>,
    //     analyses: &mut HashMap<QualifiedContractIdentifier, Option<ContractAnalysis>>,
    //     clarity_versions: &mut HashMap<QualifiedContractIdentifier, ClarityVersion>,
    // ) {
    //     // Remove old paths
    //     // TODO(lgalabru)

    //     // Add / Replace new paths
    //     for (contract_id, runbook_location) in locations.iter() {
    //         let (contract_id, ast) = match asts.remove_entry(contract_id) {
    //             Some(ast) => ast,
    //             None => continue,
    //         };
    //         let deps = match deps.remove(&contract_id) {
    //             Some(deps) => deps,
    //             None => DependencySet::new(),
    //         };
    //         let diags = match diags.remove(&contract_id) {
    //             Some(diags) => diags,
    //             None => vec![],
    //         };
    //         let analysis = match analyses.remove(&contract_id) {
    //             Some(analysis) => analysis,
    //             None => None,
    //         };
    //         let clarity_version = match clarity_versions.remove(&contract_id) {
    //             Some(analysis) => analysis,
    //             None => DEFAULT_CLARITY_VERSION,
    //         };
    //         let definitions = match definitions.remove(&contract_id) {
    //             Some(definitions) => definitions,
    //             None => HashMap::new(),
    //         };

    //         let contract_state = ContractState::new(
    //             contract_id.clone(),
    //             ast,
    //             deps,
    //             diags,
    //             analysis,
    //             definitions,
    //             runbook_location.clone(),
    //             clarity_version,
    //         );
    //         self.contracts
    //             .insert(runbook_location.clone(), contract_state);

    //         self.locations_lookup
    //             .insert(contract_id, runbook_location.clone());
    //     }
    // }

    // pub fn get_contract_calls_for_contract(
    //     &self,
    //     contract_uri: &FileLocation,
    // ) -> Vec<CompletionItem> {
    //     let mut contract_calls = vec![];
    //     for (url, contract_state) in self.contracts.iter() {
    //         if !contract_uri.eq(url) {
    //             contract_calls.append(&mut contract_state.contract_calls.clone());
    //         }
    //     }
    //     contract_calls
    // }
}

pub async fn build_state(
    manifest_location: &FileLocation,
    workspace_state: &mut WorkspaceState,
    file_accessor: Option<&dyn FileAccessor>,
) -> Result<(), String> {
    // let manifest = match file_accessor {
    //     None => WorkspaceManifest::from_location(manifest_location)?,
    //     Some(file_accessor) => {
    //         WorkspaceManifest::from_file_accessor(manifest_location, file_accessor).await?
    //     }
    // };

    //     let (_manifest, _runbook_name, mut runbook, runbook_state) =
    //     load_runbook_from_manifest(&cmd.manifest_path, &cmd.runbook, &cmd.environment).await?;

    // match &runbook_state {
    //     Some(RunbookState::File(state_file_location)) => {
    //         let ctx = RunbookSnapshotContext::new();
    //         let old = load_runbook_execution_snapshot(state_file_location)?;
    //         for run in runbook.running_contexts.iter_mut() {
    //             let frontier = HashSet::new();
    //             let _res = run
    //                 .execution_context
    //                 .simulate_execution(
    //                     &runbook.runtime_context,
    //                     &run.workspace_context,
    //                     &runbook.supervision_context,
    //                     &frontier,
    //                 )
    //                 .await;
    //         }

    // let (deployment, mut artifacts) = generate_default_deployment(
    //     &manifest,
    //     &StacksNetwork::Simnet,
    //     false,
    //     file_accessor,
    //     Some(StacksEpochId::Epoch21),
    // )
    // .await?;

    // let mut session = initiate_session_from_deployment(&manifest);
    // let UpdateSessionExecutionResult { contracts, .. } = update_session_with_contracts_executions(
    //     &mut session,
    //     &deployment,
    //     Some(&artifacts.asts),
    //     false,
    //     Some(StacksEpochId::Epoch21),
    // );
    // for (contract_id, mut result) in contracts.into_iter() {
    //     let (_, runbook_location) = match deployment.contracts.get(&contract_id) {
    //         Some(entry) => entry,
    //         None => continue,
    //     };
    //     locations.insert(contract_id.clone(), runbook_location.clone());
    //     if let Some(contract_metadata) = manifest.contracts_settings.get(runbook_location) {
    //         clarity_versions.insert(contract_id.clone(), contract_metadata.clarity_version);
    //     }

    //     match result {
    //         Ok(mut execution_result) => {
    //             if let Some(entry) = artifacts.diags.get_mut(&contract_id) {
    //                 entry.append(&mut execution_result.diagnostics);
    //             }

    //             if let EvaluationResult::Contract(contract_result) = execution_result.result {
    //                 if let Some(ast) = artifacts.asts.get(&contract_id) {
    //                     definitions.insert(
    //                         contract_id.clone(),
    //                         get_public_function_definitions(&ast.expressions),
    //                     );
    //                 }
    //                 analyses.insert(contract_id.clone(), Some(contract_result.contract.analysis));
    //             };
    //         }
    //         Err(ref mut diags) => {
    //             if let Some(entry) = artifacts.diags.get_mut(&contract_id) {
    //                 entry.append(diags);
    //             }
    //             continue;
    //         }
    //     };
    // }

    // protocol_state.consolidate(
    //     &mut locations,
    //     &mut artifacts.asts,
    //     &mut artifacts.deps,
    //     &mut artifacts.diags,
    //     &mut definitions,
    //     &mut analyses,
    //     &mut clarity_versions,
    // );

    Ok(())
}
