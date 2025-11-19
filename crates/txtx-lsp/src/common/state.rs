use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionItemLabelDetails, DocumentSymbol, Hover,
    InsertTextFormat, InsertTextMode, MarkupContent, MarkupKind, MessageType, Position,
    SignatureHelp,
};
use std::borrow::BorrowMut;
use std::collections::{HashMap, HashSet};
use std::vec;
use txtx_addon_kit::helpers::fs::{FileAccessor, FileLocation};
use txtx_addon_kit::types::diagnostics::{Diagnostic as TxtxDiagnostic, DiagnosticLevel};
use txtx_addon_kit::types::RunbookId;
use txtx_addon_kit::Addon;
use txtx_addon_telegram::TelegramAddon;
use txtx_core::std::StdAddon;

use super::requests::capabilities::InitializationOptions;

lazy_static! {
    pub static ref FUNCTIONS: Vec<CompletionItem> = {
        let addons: Vec<Box<dyn Addon>> = vec![Box::new(StdAddon::new()), Box::new(TelegramAddon::new())];
        let mut completion_items = vec![];
        for addon in addons.iter() {
            for func in addon.get_functions() {
                completion_items.push(lsp_types::CompletionItem {
                    // The label of this completion item. By default
                    // also the text that is inserted when selecting
                    // this completion.
                    label: format!("{}::{}", addon.get_namespace(), func.name),
                    // Additional details for the label
                    label_details: Some(CompletionItemLabelDetails {
                        detail: Some(format!("1) {}", func.documentation)),
                        description: Some(format!("2) {}", func.documentation))
                    }), //Option<CompletionItemLabelDetails>,
                    // The kind of this completion item. Based of the kind
                    // an icon is chosen by the editor.
                    kind: Some(CompletionItemKind::FUNCTION), //Option<CompletionItemKind>,
                    // A human-readable string with additional information
                    // about this item, like type or symbol information.
                    detail: Some(format!("{}::{}({})", addon.get_namespace(), func.name, func.inputs.iter().map(|i| i.name.clone()).collect::<Vec<_>>().join(", "))), //Option<String>,
                    // A human-readable string that represents a doc-comment.
                    documentation: Some(lsp_types::Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: format!("{}\n\n## Arguments\n{}\n\n## Example\n```hcl\n{}\n```", func.documentation, func.inputs.iter().map(|i| format!("`{}`: {}", i.name, i.documentation)).collect::<Vec<_>>().join("\n\n"), func.example)
                    })),
                    // Indicates if this item is deprecated.
                    deprecated: None, //Option<bool>,
                    // Select this item when showing.
                    preselect: None, //Option<bool>,
                    // A string that should be used when comparing this item
                    // with other items. When `falsy` the label is used
                    // as the sort text for this item.
                    sort_text: None, // Option<String>,
                    // A string that should be used when filtering a set of
                    // completion items. When `falsy` the label is used as the
                    // filter text for this item.
                    filter_text: None, // Option<String>,
                    // A string that should be inserted into a document when selecting
                    // this completion. When `falsy` the label is used as the insert text
                    // for this item.
                    //
                    // The `insertText` is subject to interpretation by the client side.
                    // Some tools might not take the string literally. For example
                    // VS Code when code complete is requested in this example
                    // `con<cursor position>` and a completion item with an `insertText` of
                    // `console` is provided it will only insert `sole`. Therefore it is
                    // recommended to use `textEdit` instead since it avoids additional client
                    // side interpretation.
                    insert_text: Some(format!("{}::{}({})", addon.get_namespace(), func.name, func.inputs.iter().enumerate().map(|(i, input)| format!("${{{}:{}}}", i, input.name)).collect::<Vec<_>>().join(", "))),
                    // The format of the insert text. The format applies to both the `insertText` property
                    // and the `newText` property of a provided `textEdit`. If omitted defaults to `InsertTextFormat.PlainText`.
                    insert_text_format: Some(InsertTextFormat::SNIPPET), // Option<InsertTextFormat>,
                    // How whitespace and indentation is handled during completion
                    // item insertion. If not provided the client's default value depends on
                    // the `textDocument.completion.insertTextMode` client capability.
                    insert_text_mode: Some(InsertTextMode::ADJUST_INDENTATION),
                    // An edit which is applied to a document when selecting
                    // this completion. When an edit is provided the value of
                    // insertText is ignored.
                    //
                    // Most editors support two different operation when accepting a completion item. One is to insert a
                    // completion text and the other is to replace an existing text with a completion text. Since this can
                    // usually not predetermined by a server it can report both ranges. Clients need to signal support for
                    // `InsertReplaceEdits` via the `textDocument.completion.insertReplaceSupport` client capability
                    // property.
                    //
                    // *Note 1:* The text edit's range as well as both ranges from a insert replace edit must be a
                    // [single line] and they must contain the position at which completion has been requested.
                    // *Note 2:* If an `InsertReplaceEdit` is returned the edit's insert range must be a prefix of
                    // the edit's replace range, that means it must be contained and starting at the same position.
                    text_edit: None,
                    // An optional array of additional text edits that are applied when
                    // selecting this completion. Edits must not overlap with the main edit
                    // nor with themselves.
                    additional_text_edits: None,
                    // An optional command that is executed *after* inserting this completion. *Note* that
                    // additional modifications to the current document should be described with the
                    // additionalTextEdits-property.
                    command: None,
                    // An optional set of characters that when pressed while this completion is
                    // active will accept it first and then type that character. *Note* that all
                    // commit characters should have `length=1` and that superfluous characters
                    // will be ignored.
                    commit_characters: None, //Option<Vec<String>>,
                    data: None, // Option<Value>,
                    ..Default::default()
                });
            }
        }
        completion_items
    };

    pub static ref ACTIONS: Vec<CompletionItem> = {
        let addons: Vec<Box<dyn Addon>> = vec![Box::new(StdAddon::new()), Box::new(TelegramAddon::new())];
        let mut completion_items = vec![];
        for addon in addons.iter() {
            for action in addon.get_actions() {
                let spec = action.expect_atomic_specification();
                completion_items.push(lsp_types::CompletionItem {
                    // The label of this completion item. By default
                    // also the text that is inserted when selecting
                    // this completion.
                    label: format!("{}::{}", addon.get_namespace(), spec.matcher),
                    // Additional details for the label
                    label_details: None, //Option<CompletionItemLabelDetails>,
                    // The kind of this completion item. Based of the kind
                    // an icon is chosen by the editor.
                    kind: Some(CompletionItemKind::CLASS), //Option<CompletionItemKind>,
                    // A human-readable string with additional information
                    // about this item, like type or symbol information.
                    detail: Some(format!("action <name> \"{}::{}\" {{\n{}\n}}", addon.get_namespace(), spec.matcher, spec.inputs.iter().map(|i| i.name.clone()).collect::<Vec<_>>().join("\n"))), //Option<String>,
                    // A human-readable string that represents a doc-comment.
                    documentation: Some(lsp_types::Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: format!("{}\n\n## Arguments\n{}\n\n## Example\n```hcl\n{}\n```", spec.documentation, spec.inputs.iter().map(|i| format!("`{}`: {}", i.name, i.documentation)).collect::<Vec<_>>().join("\n\n"), spec.example)
                    })),
                    // Indicates if this item is deprecated.
                    deprecated: None, //Option<bool>,
                    // Select this item when showing.
                    preselect: None, //Option<bool>,
                    // A string that should be used when comparing this item
                    // with other items. When `falsy` the label is used
                    // as the sort text for this item.
                    sort_text: None, // Option<String>,
                    // A string that should be used when filtering a set of
                    // completion items. When `falsy` the label is used as the
                    // filter text for this item.
                    filter_text: None, // Option<String>,
                    // A string that should be inserted into a document when selecting
                    // this completion. When `falsy` the label is used as the insert text
                    // for this item.
                    //
                    // The `insertText` is subject to interpretation by the client side.
                    // Some tools might not take the string literally. For example
                    // VS Code when code complete is requested in this example
                    // `con<cursor position>` and a completion item with an `insertText` of
                    // `console` is provided it will only insert `sole`. Therefore it is
                    // recommended to use `textEdit` instead since it avoids additional client
                    // side interpretation.
                    insert_text: Some(format!("action \"${{1:name}}\" \"{}::{}\" {{\n{}\n}}", addon.get_namespace(), spec.matcher, spec.inputs.iter().enumerate().map(|(i, input)| format!("    // {}\n    {} = ${{{}:{}}}", input.documentation, input.name, i+2, input.name)).collect::<Vec<_>>().join("\n"))),
                    // The format of the insert text. The format applies to both the `insertText` property
                    // and the `newText` property of a provided `textEdit`. If omitted defaults to `InsertTextFormat.PlainText`.
                    insert_text_format: Some(InsertTextFormat::SNIPPET), // Option<InsertTextFormat>,
                    // How whitespace and indentation is handled during completion
                    // item insertion. If not provided the client's default value depends on
                    // the `textDocument.completion.insertTextMode` client capability.
                    insert_text_mode: Some(InsertTextMode::ADJUST_INDENTATION),
                    // An edit which is applied to a document when selecting
                    // this completion. When an edit is provided the value of
                    // insertText is ignored.
                    //
                    // Most editors support two different operation when accepting a completion item. One is to insert a
                    // completion text and the other is to replace an existing text with a completion text. Since this can
                    // usually not predetermined by a server it can report both ranges. Clients need to signal support for
                    // `InsertReplaceEdits` via the `textDocument.completion.insertReplaceSupport` client capability
                    // property.
                    //
                    // *Note 1:* The text edit's range as well as both ranges from a insert replace edit must be a
                    // [single line] and they must contain the position at which completion has been requested.
                    // *Note 2:* If an `InsertReplaceEdit` is returned the edit's insert range must be a prefix of
                    // the edit's replace range, that means it must be contained and starting at the same position.
                    text_edit: None,
                    // An optional array of additional text edits that are applied when
                    // selecting this completion. Edits must not overlap with the main edit
                    // nor with themselves.
                    additional_text_edits: None,
                    // An optional command that is executed *after* inserting this completion. *Note* that
                    // additional modifications to the current document should be described with the
                    // additionalTextEdits-property.
                    command: None,
                    // An optional set of characters that when pressed while this completion is
                    // active will accept it first and then type that character. *Note* that all
                    // commit characters should have `length=1` and that superfluous characters
                    // will be ignored.
                    commit_characters: None, //Option<Vec<String>>,
                    data: None, // Option<Value>,
                    ..Default::default()
                });
            }
        }
        completion_items
    };


    pub static ref WALLETS: Vec<CompletionItem> = {
        let addons: Vec<Box<dyn Addon>> = vec![Box::new(StdAddon::new()), Box::new(TelegramAddon::new())];
        let mut completion_items = vec![];
        for addon in addons.iter() {
            for signer in addon.get_signers() {
                let spec = signer;
                completion_items.push(lsp_types::CompletionItem {
                    // The label of this completion item. By default
                    // also the text that is inserted when selecting
                    // this completion.
                    label: format!("{}::{}", addon.get_namespace(), spec.matcher),
                    // Additional details for the label
                    label_details: None, //Option<CompletionItemLabelDetails>,
                    // The kind of this completion item. Based of the kind
                    // an icon is chosen by the editor.
                    kind: Some(CompletionItemKind::CLASS), //Option<CompletionItemKind>,
                    // A human-readable string with additional information
                    // about this item, like type or symbol information.
                    detail: Some(format!("signer <name> \"{}::{}\" {{\n{}\n}}", addon.get_namespace(), spec.matcher, spec.inputs.iter().map(|i| i.name.clone()).collect::<Vec<_>>().join("\n"))), //Option<String>,
                    // A human-readable string that represents a doc-comment.
                    documentation: Some(lsp_types::Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: format!("{}\n\n## Arguments\n{}\n\n## Example\n```hcl\n{}\n```", spec.documentation, spec.inputs.iter().map(|i| format!("`{}`: {}", i.name, i.documentation)).collect::<Vec<_>>().join("\n\n"), spec.example)
                    })),
                    // Indicates if this item is deprecated.
                    deprecated: None, //Option<bool>,
                    // Select this item when showing.
                    preselect: None, //Option<bool>,
                    // A string that should be used when comparing this item
                    // with other items. When `falsy` the label is used
                    // as the sort text for this item.
                    sort_text: None, // Option<String>,
                    // A string that should be used when filtering a set of
                    // completion items. When `falsy` the label is used as the
                    // filter text for this item.
                    filter_text: None, // Option<String>,
                    // A string that should be inserted into a document when selecting
                    // this completion. When `falsy` the label is used as the insert text
                    // for this item.
                    //
                    // The `insertText` is subject to interpretation by the client side.
                    // Some tools might not take the string literally. For example
                    // VS Code when code complete is requested in this example
                    // `con<cursor position>` and a completion item with an `insertText` of
                    // `console` is provided it will only insert `sole`. Therefore it is
                    // recommended to use `textEdit` instead since it avoids additional client
                    // side interpretation.
                    insert_text: Some(format!("signer \"${{1:name}}\" \"{}::{}\" {{\n{}\n}}", addon.get_namespace(), spec.matcher, spec.inputs.iter().enumerate().map(|(i, input)| format!("    // {}\n    {} = ${{{}:{}}}", input.documentation, input.name, i+2, input.name)).collect::<Vec<_>>().join("\n"))),
                    // The format of the insert text. The format applies to both the `insertText` property
                    // and the `newText` property of a provided `textEdit`. If omitted defaults to `InsertTextFormat.PlainText`.
                    insert_text_format: Some(InsertTextFormat::SNIPPET), // Option<InsertTextFormat>,
                    // How whitespace and indentation is handled during completion
                    // item insertion. If not provided the client's default value depends on
                    // the `textDocument.completion.insertTextMode` client capability.
                    insert_text_mode: Some(InsertTextMode::ADJUST_INDENTATION),
                    // An edit which is applied to a document when selecting
                    // this completion. When an edit is provided the value of
                    // insertText is ignored.
                    //
                    // Most editors support two different operation when accepting a completion item. One is to insert a
                    // completion text and the other is to replace an existing text with a completion text. Since this can
                    // usually not predetermined by a server it can report both ranges. Clients need to signal support for
                    // `InsertReplaceEdits` via the `textDocument.completion.insertReplaceSupport` client capability
                    // property.
                    //
                    // *Note 1:* The text edit's range as well as both ranges from a insert replace edit must be a
                    // [single line] and they must contain the position at which completion has been requested.
                    // *Note 2:* If an `InsertReplaceEdit` is returned the edit's insert range must be a prefix of
                    // the edit's replace range, that means it must be contained and starting at the same position.
                    text_edit: None,
                    // An optional array of additional text edits that are applied when
                    // selecting this completion. Edits must not overlap with the main edit
                    // nor with themselves.
                    additional_text_edits: None,
                    // An optional command that is executed *after* inserting this completion. *Note* that
                    // additional modifications to the current document should be described with the
                    // additionalTextEdits-property.
                    command: None,
                    // An optional set of characters that when pressed while this completion is
                    // active will accept it first and then type that character. *Note* that all
                    // commit characters should have `length=1` and that superfluous characters
                    // will be ignored.
                    commit_characters: None, //Option<Vec<String>>,
                    data: None, // Option<Value>,
                    ..Default::default()
                });
            }
        }
        completion_items
    };

}

#[derive(Debug, Clone, PartialEq)]
pub struct ActiveRunbookData {
    source: String,
}

impl ActiveRunbookData {
    pub fn new(source: &str) -> Self {
        ActiveRunbookData { source: source.to_string() }
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

        RunbookState { runbook_id, errors, warnings, notes, location }
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
                let mut segments = url.path_segments_mut().expect("could not find root location");
                segments.pop();
                segments.pop();
            }
        };

        for (runbook_location, _runbook_state) in workspace.runbooks.iter() {
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
        _runbook_location: &FileLocation,
        _position: &Position,
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
        let functions = FUNCTIONS.clone();
        let mut actions = ACTIONS.clone();
        let mut signers = WALLETS.clone();
        let mut completion_items = functions;
        completion_items.append(&mut actions);
        completion_items.append(&mut signers);
        completion_items
    }

    pub fn get_document_symbols_for_runbook(
        &self,
        _runbook_location: &FileLocation,
    ) -> Vec<DocumentSymbol> {
        vec![]
    }

    pub fn get_definition_location(
        &self,
        _runbook_location: &FileLocation,
        _position: &Position,
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
        _runbook_location: &FileLocation,
        _position: &lsp_types::Position,
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
        _active_signature: Option<u32>,
    ) -> Option<SignatureHelp> {
        let _runbook = self.active_runbooks.get(runbook_location)?;
        let _position = Position { line: position.line + 1, character: position.character + 1 };
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
    ) -> (Vec<(FileLocation, Vec<TxtxDiagnostic>)>, Option<(MessageType, String)>) {
        let mut runbooks = vec![];
        let mut erroring_files = HashSet::new();
        let mut warning_files = HashSet::new();

        for (_, workspace_state) in self.workspaces.iter() {
            for (runbook_url, state) in workspace_state.runbooks.iter() {
                let mut diags = vec![];

                let RunbookMetadata { relative_path, .. } =
                    self.runbooks_lookup.get(runbook_url).expect("contract not in lookup");

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
        _source: &str,
        _with_definitions: bool,
    ) -> Result<(), String> {
        let _runbook = self
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
    _manifest_location: &FileLocation,
    _workspace_state: &mut WorkspaceState,
    _file_accessor: Option<&dyn FileAccessor>,
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
