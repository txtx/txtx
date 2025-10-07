//! Centralized workspace state management.
//!
//! This module provides [`WorkspaceState`] for coordinating documents, manifests,
//! and their relationships in the LSP server. Includes validation caching,
//! dependency tracking, and environment management.

use super::{
    dependency_graph::DependencyGraph,
    manifests::find_manifest_for_runbook,
    state_machine::{MachineState, StateHistory},
    validation_state::ValidationState,
    Document, Manifest,
};
use lsp_types::{Diagnostic, Url};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, RwLock};

/// The workspace state containing all documents and parsed information.
///
/// Central state manager for the LSP server that coordinates:
/// - Open documents and their content
/// - Parsed manifest files
/// - Runbook-to-manifest associations
/// - Validation caching and invalidation
/// - Dependency tracking between files
/// - Environment selection and variables
///
/// Uses content hashing and dependency tracking to minimize redundant
/// validation operations.
#[derive(Debug)]
pub struct WorkspaceState {
    /// All open documents indexed by URI.
    documents: HashMap<Url, Document>,
    /// Parsed manifests indexed by their URI.
    manifests: HashMap<Url, Manifest>,
    /// Map from runbook URI to its manifest URI.
    runbook_to_manifest: HashMap<Url, Url>,
    /// Cached environment variables for quick lookup.
    environment_vars: HashMap<String, HashMap<String, String>>,
    /// The currently selected environment from VS Code.
    current_environment: Option<String>,
    /// Validation state cache.
    validation_cache: HashMap<Url, ValidationState>,
    /// Dependency graph tracking file relationships.
    dependencies: DependencyGraph,
    /// Documents that need re-validation.
    dirty_documents: HashSet<Url>,
    /// Map from action name to the document URI where it's defined.
    action_definitions: HashMap<String, Url>,
    /// Map from variable name to the document URI where it's defined.
    variable_definitions: HashMap<String, Url>,
    /// Current workspace-level state machine state.
    machine_state: MachineState,
    /// State transition history for debugging.
    state_history: StateHistory,
}

impl WorkspaceState {
    /// Creates a new empty workspace state.
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            manifests: HashMap::new(),
            runbook_to_manifest: HashMap::new(),
            environment_vars: HashMap::new(),
            current_environment: None,
            validation_cache: HashMap::new(),
            dependencies: DependencyGraph::new(),
            dirty_documents: HashSet::new(),
            action_definitions: HashMap::new(),
            variable_definitions: HashMap::new(),
            machine_state: MachineState::default(),
            state_history: StateHistory::default(),
        }
    }

    /// Computes hash of content for change detection.
    ///
    /// Uses Rust's `DefaultHasher` for fast, non-cryptographic hashing.
    /// The hash is used to detect when document content has changed.
    ///
    /// # Arguments
    ///
    /// * `content` - The document content to hash
    ///
    /// # Returns
    ///
    /// A 64-bit hash value representing the content.
    pub fn compute_content_hash(content: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }

    /// Checks if a document needs validation.
    ///
    /// Returns `true` if:
    /// - No cached validation exists
    /// - Content has changed since last validation
    /// - Environment has changed since last validation
    /// - Validation is marked as stale (dependency changed)
    ///
    /// # Arguments
    ///
    /// * `uri` - The document to check
    /// * `content` - Current content of the document
    pub fn needs_validation(&self, uri: &Url, content: &str) -> bool {
        if let Some(validation_state) = self.validation_cache.get(uri) {
            let current_hash = Self::compute_content_hash(content);
            !validation_state.is_valid_for(current_hash, &self.current_environment)
        } else {
            // No validation state = needs validation
            true
        }
    }

    /// Get validation state for a document
    pub fn get_validation_state(&self, uri: &Url) -> Option<&ValidationState> {
        self.validation_cache.get(uri)
    }

    /// Update validation state for a document
    pub fn update_validation_state(
        &mut self,
        uri: &Url,
        status: super::validation_state::ValidationStatus,
        content_hash: u64,
        diagnostics: Vec<Diagnostic>,
    ) {
        let validation_state = self
            .validation_cache
            .entry(uri.clone())
            .or_insert_with(ValidationState::new);

        validation_state.update_with_results(
            status,
            content_hash,
            self.current_environment.clone(),
            diagnostics,
        );

        // Remove from dirty set if successfully validated
        if status != super::validation_state::ValidationStatus::Stale {
            self.dirty_documents.remove(uri);
        }
    }

    /// Mark a document as dirty (needs re-validation)
    pub fn mark_dirty(&mut self, uri: &Url) {
        self.dirty_documents.insert(uri.clone());
        if let Some(state) = self.validation_cache.get_mut(uri) {
            state.mark_stale();
        }
    }

    /// Marks all documents affected by changes to `uri` as dirty.
    ///
    /// Uses transitive dependency tracking to mark all dependents.
    fn mark_affected_documents_dirty(&mut self, uri: &Url) {
        let affected = self.dependencies.get_affected_documents(uri);
        for dep_uri in affected {
            self.mark_dirty(&dep_uri);
        }
    }

    /// Get all dirty documents
    pub fn get_dirty_documents(&self) -> &HashSet<Url> {
        &self.dirty_documents
    }

    /// Get the dependency graph
    pub fn dependencies(&self) -> &DependencyGraph {
        &self.dependencies
    }

    /// Get mutable access to dependency graph
    pub fn dependencies_mut(&mut self) -> &mut DependencyGraph {
        &mut self.dependencies
    }

    /// Open a document in the workspace
    pub fn open_document(&mut self, uri: Url, content: String) {
        let document = Document::new(uri.clone(), content.clone());

        // If it's a manifest, parse and index it
        if document.is_manifest() {
            self.index_manifest(&uri, &content);
        }
        // If it's a runbook, find its manifest and extract dependencies
        else if document.is_runbook() {
            self.index_runbook(&uri);
            self.extract_and_update_dependencies(&uri, &content);
        }

        self.documents.insert(uri, document);
    }

    /// Update an existing document
    pub fn update_document(&mut self, uri: &Url, content: String) {
        // Check needs validation before getting mutable borrow
        let needs_validation = self.needs_validation(uri, &content);

        let (is_manifest, is_runbook) = if let Some(doc) = self.documents.get(uri) {
            (doc.is_manifest(), doc.is_runbook())
        } else {
            (false, false)
        };

        if let Some(doc) = self.documents.get_mut(uri) {
            doc.update(content.clone());
        }

        // Mark as dirty if content changed
        if needs_validation {
            self.mark_dirty(uri);
        }

        // Re-index if it's a manifest
        if is_manifest {
            self.index_manifest(uri, &content);
            self.mark_affected_documents_dirty(uri);
        }
        // Re-extract dependencies if it's a runbook
        else if is_runbook {
            self.extract_and_update_dependencies(uri, &content);
            self.mark_affected_documents_dirty(uri);
        }
    }

    /// Close a document
    pub fn close_document(&mut self, uri: &Url) {
        // Check if it's a manifest before removing document
        let is_manifest = self.manifests.contains_key(uri);

        self.documents.remove(uri);

        // Clean up validation state
        self.validation_cache.remove(uri);
        self.dirty_documents.remove(uri);

        // Clean up dependencies
        self.dependencies.remove_document(uri);

        // Clean up manifest data if closing a manifest
        if is_manifest {
            self.manifests.remove(uri);
            // Remove runbook associations
            self.runbook_to_manifest.retain(|_, manifest_uri| manifest_uri != uri);
            // Clear environment cache for this manifest's environments
            // (We could be more precise here, but clearing all is safe)
            self.environment_vars.clear();
            // Re-populate from remaining manifests
            for manifest in self.manifests.values() {
                for (env_name, vars) in &manifest.environments {
                    self.environment_vars.insert(env_name.clone(), vars.clone());
                }
            }
        }
    }

    /// Get a document by URI
    pub fn get_document(&self, uri: &Url) -> Option<&Document> {
        self.documents.get(uri)
    }

    /// Get all open documents
    #[allow(dead_code)]
    pub fn documents(&self) -> &HashMap<Url, Document> {
        &self.documents
    }

    /// Get URIs of all open documents
    pub fn get_all_document_uris(&self) -> Vec<Url> {
        self.documents.keys().cloned().collect()
    }

    /// Get a manifest by URI
    #[allow(dead_code)]
    pub fn get_manifest(&self, uri: &Url) -> Option<&Manifest> {
        self.manifests.get(uri)
    }

    /// Get the manifest for a runbook
    pub fn get_manifest_for_runbook(&self, runbook_uri: &Url) -> Option<&Manifest> {
        self.runbook_to_manifest
            .get(runbook_uri)
            .and_then(|manifest_uri| self.manifests.get(manifest_uri))
    }

    /// Get the manifest for a document (alias for get_manifest_for_runbook)
    pub fn get_manifest_for_document(&self, document_uri: &Url) -> Option<&Manifest> {
        self.get_manifest_for_runbook(document_uri)
    }

    /// Get environment variables for a specific environment
    #[allow(dead_code)]
    pub fn get_environment_vars(&self, env_name: &str) -> Option<&HashMap<String, String>> {
        self.environment_vars.get(env_name)
    }

    /// Parse and index a manifest
    fn index_manifest(&mut self, uri: &Url, content: &str) {
        eprintln!("[DEBUG] Indexing manifest: {}", uri);
        match Manifest::parse(uri.clone(), content) {
            Ok(manifest) => {
                eprintln!(
                    "[DEBUG] Manifest parsed successfully with {} runbooks",
                    manifest.runbooks.len()
                );
                // Update environment cache
                for (env_name, vars) in &manifest.environments {
                    self.environment_vars.insert(env_name.clone(), vars.clone());
                }

                // Update runbook associations
                for runbook in &manifest.runbooks {
                    if let Some(runbook_uri) = &runbook.absolute_uri {
                        self.runbook_to_manifest.insert(runbook_uri.clone(), uri.clone());
                    }
                }

                self.manifests.insert(uri.clone(), manifest);
            }
            Err(e) => {
                eprintln!("Failed to parse manifest {}: {}", uri, e);
            }
        }
    }

    /// Index a runbook by finding its manifest
    fn index_runbook(&mut self, runbook_uri: &Url) {
        if let Some(manifest_uri) = find_manifest_for_runbook(runbook_uri) {
            self.runbook_to_manifest.insert(runbook_uri.clone(), manifest_uri.clone());

            // Try to load the manifest if we haven't already
            if !self.manifests.contains_key(&manifest_uri) {
                if let Ok(content) = std::fs::read_to_string(manifest_uri.path()) {
                    self.index_manifest(&manifest_uri, &content);
                }
            }
        }
    }

    /// Updates a definition map with new definitions from a document.
    ///
    /// Removes old definitions for the document, then adds new ones.
    fn update_definition_map(
        map: &mut HashMap<String, Url>,
        uri: &Url,
        new_definitions: &HashSet<String>,
    ) {
        map.retain(|_, def_uri| def_uri != uri);
        for name in new_definitions {
            map.insert(name.clone(), uri.clone());
        }
    }

    /// Adds dependencies from name references to their definitions.
    ///
    /// For each name in `references`, looks it up in `definitions` and adds
    /// a dependency edge if found and not self-referential.
    fn add_reference_dependencies(
        dependencies: &mut DependencyGraph,
        uri: &Url,
        references: &HashSet<String>,
        definitions: &HashMap<String, Url>,
    ) {
        for name in references {
            if let Some(def_uri) = definitions.get(name) {
                if def_uri != uri {
                    dependencies.add_dependency(uri.clone(), def_uri.clone());
                }
            }
        }
    }

    /// Extract dependencies from content and update dependency graph
    fn extract_and_update_dependencies(&mut self, uri: &Url, content: &str) {
        use super::dependency_extractor::extract_dependencies;

        // Remove old dependencies for this document
        let old_deps: Vec<Url> = self
            .dependencies
            .get_dependencies(uri)
            .map(|deps| deps.iter().cloned().collect())
            .unwrap_or_default();

        for old_dep in old_deps {
            self.dependencies.remove_dependency(uri, &old_dep);
        }

        // Extract new dependencies from content
        let deps = extract_dependencies(content);

        // Update definition maps
        Self::update_definition_map(&mut self.action_definitions, uri, &deps.defined_actions);
        Self::update_definition_map(&mut self.variable_definitions, uri, &deps.defined_variables);

        // Add dependency on manifest if uses input.*
        if deps.uses_manifest_inputs {
            if let Some(manifest_uri) = self.runbook_to_manifest.get(uri) {
                self.dependencies
                    .add_dependency(uri.clone(), manifest_uri.clone());
            }
        }

        // Add dependencies for output.* and variable.* references
        Self::add_reference_dependencies(
            &mut self.dependencies,
            uri,
            &deps.action_outputs,
            &self.action_definitions,
        );
        Self::add_reference_dependencies(
            &mut self.dependencies,
            uri,
            &deps.variables,
            &self.variable_definitions,
        );
    }

    /// Get the currently selected environment
    pub fn get_current_environment(&self) -> Option<String> {
        self.current_environment.clone()
    }

    /// Sets the currently selected environment.
    ///
    /// When the environment changes, all open runbook documents are automatically
    /// marked as dirty to trigger re-validation with the new environment context.
    /// This ensures that validation results reflect the correct environment-specific
    /// inputs and variables.
    ///
    /// # Arguments
    ///
    /// * `environment` - The new environment name, or `None` to clear the selection
    ///
    /// # Side Effects
    ///
    /// If the environment actually changes (new value differs from current):
    /// - All open runbook documents are marked as dirty
    /// - Subsequent validation will use the new environment context
    /// - Manifest documents are not affected (they don't depend on environment)
    ///
    /// # Example
    ///
    /// ```ignore
    /// workspace.set_current_environment(Some("production".to_string()));
    /// // All runbooks now marked dirty and will be re-validated with production env
    /// ```
    pub fn set_current_environment(&mut self, environment: Option<String>) {
        // If environment actually changed, mark all runbooks as dirty
        if self.current_environment != environment {
            // Collect URIs first to avoid holding immutable borrow during mark_dirty
            let runbook_uris: Vec<Url> = self
                .documents
                .iter()
                .filter_map(|(uri, doc)| doc.is_runbook().then(|| uri.clone()))
                .collect();

            for uri in runbook_uris {
                self.mark_dirty(&uri);
            }
        }

        self.current_environment = environment;
    }

    /// Returns the current workspace-level machine state.
    pub fn get_machine_state(&self) -> &MachineState {
        &self.machine_state
    }

    /// Returns the state transition history for debugging.
    pub fn get_state_history(&self) -> &StateHistory {
        &self.state_history
    }

    /// Transitions to a new machine state with logging.
    ///
    /// Records the transition in the state history and emits a log message.
    ///
    /// # Arguments
    ///
    /// * `new_state` - State to transition to
    /// * `event` - Description of triggering event
    fn transition_state(&mut self, new_state: MachineState, event: impl Into<String>) {
        use super::state_machine::StateTransition;

        let old_state = std::mem::replace(&mut self.machine_state, new_state.clone());
        let transition = StateTransition::new(old_state, new_state, event);

        eprintln!("[LSP STATE] {}", transition.format());
        self.state_history.record(transition);
    }

    /// Handles document validation events.
    ///
    /// Transitions to Validating state and queues validation action.
    fn handle_document_validation(
        &mut self,
        uri: Url,
        event_name: &str,
    ) -> Vec<super::state_machine::StateAction> {
        use super::state_machine::StateAction;

        if self.machine_state.can_accept_requests() {
            self.transition_state(
                MachineState::Validating {
                    document: uri.clone(),
                },
                event_name,
            );
            vec![StateAction::ValidateDocument { uri }]
        } else {
            Vec::new()
        }
    }

    /// Creates a publish diagnostics action.
    fn publish_diagnostics_action(
        uri: Url,
        diagnostics: Vec<lsp_types::Diagnostic>,
    ) -> super::state_machine::StateAction {
        use super::state_machine::StateAction;
        StateAction::PublishDiagnostics { uri, diagnostics }
    }

    /// Processes a state event and produces actions.
    ///
    /// Core event-driven method handling state transitions and generating actions.
    /// The method validates events, performs transitions, and returns actions.
    ///
    /// # Arguments
    ///
    /// * `event` - Event to process
    ///
    /// # Returns
    ///
    /// Actions to perform in response to the event.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut workspace = WorkspaceState::new();
    /// let event = StateEvent::Initialize;
    /// let actions = workspace.process_event(event);
    /// ```
    pub fn process_event(
        &mut self,
        event: super::state_machine::StateEvent,
    ) -> Vec<super::state_machine::StateAction> {
        use super::state_machine::{StateAction, StateEvent};

        let mut actions = Vec::new();

        match event {
            StateEvent::Initialize => {
                self.transition_state(MachineState::Indexing, "Initialize");
                actions.push(StateAction::LogTransition {
                    message: "LSP server initialized, starting workspace indexing".to_string(),
                });
            }

            StateEvent::IndexingComplete => {
                if self.machine_state == MachineState::Indexing {
                    self.transition_state(MachineState::Ready, "IndexingComplete");
                    actions.push(StateAction::LogTransition {
                        message: "Workspace indexing completed successfully".to_string(),
                    });
                }
            }

            StateEvent::IndexingFailed { error } => {
                if self.machine_state == MachineState::Indexing {
                    self.transition_state(MachineState::IndexingError, "IndexingFailed");
                    actions.push(StateAction::LogTransition {
                        message: format!("Workspace indexing failed: {}", error),
                    });
                }
            }

            StateEvent::DocumentOpened { uri, content: _ } => {
                actions.extend(self.handle_document_validation(uri, "DocumentOpened"));
            }

            StateEvent::DocumentChanged { uri, content: _ } => {
                actions.extend(self.handle_document_validation(uri, "DocumentChanged"));
            }

            StateEvent::DocumentClosed { uri } => {
                actions.push(StateAction::InvalidateCache { uri });
            }

            StateEvent::EnvironmentChanged { new_env } => {
                if self.machine_state.can_accept_requests() {
                    self.transition_state(
                        MachineState::EnvironmentChanging {
                            new_env: new_env.clone(),
                        },
                        "EnvironmentChanged",
                    );

                    let runbook_uris: Vec<Url> = self
                        .documents
                        .iter()
                        .filter_map(|(uri, doc)| doc.is_runbook().then(|| uri.clone()))
                        .collect();

                    if !runbook_uris.is_empty() {
                        self.transition_state(
                            MachineState::Revalidating {
                                documents: runbook_uris.clone(),
                                current: 0,
                            },
                            "Revalidating after environment change",
                        );

                        for uri in runbook_uris {
                            actions.push(StateAction::ValidateDocument { uri });
                        }
                    } else {
                        self.transition_state(MachineState::Ready, "No runbooks to revalidate");
                    }
                }
            }

            StateEvent::ValidationCompleted {
                uri,
                diagnostics,
                success: _,
            } => {
                match &self.machine_state {
                    MachineState::Validating { document } if document == &uri => {
                        self.transition_state(MachineState::Ready, "ValidationCompleted");
                        actions.push(Self::publish_diagnostics_action(uri, diagnostics));
                    }
                    MachineState::Revalidating { documents, current } => {
                        let next = current + 1;
                        if next >= documents.len() {
                            self.transition_state(MachineState::Ready, "All revalidations completed");
                        } else {
                            self.transition_state(
                                MachineState::Revalidating {
                                    documents: documents.clone(),
                                    current: next,
                                },
                                format!("Revalidating {}/{}", next + 1, documents.len()),
                            );
                        }
                        actions.push(Self::publish_diagnostics_action(uri, diagnostics));
                    }
                    _ => {
                        actions.push(Self::publish_diagnostics_action(uri, diagnostics));
                    }
                }
            }

            StateEvent::DependencyChanged { uri: _, affected } => {
                if self.machine_state.can_accept_requests() {
                    self.transition_state(
                        MachineState::Invalidating {
                            affected: affected.clone(),
                        },
                        "DependencyChanged",
                    );

                    for affected_uri in &affected {
                        self.mark_dirty(affected_uri);
                        actions.push(StateAction::InvalidateCache {
                            uri: affected_uri.clone(),
                        });
                    }

                    if !affected.is_empty() {
                        let docs: Vec<Url> = affected.iter().cloned().collect();
                        self.transition_state(
                            MachineState::Revalidating {
                                documents: docs,
                                current: 0,
                            },
                            "Revalidating affected documents",
                        );

                        for affected_uri in affected {
                            actions.push(StateAction::ValidateDocument {
                                uri: affected_uri,
                            });
                        }
                    } else {
                        self.transition_state(MachineState::Ready, "No affected documents");
                    }
                }
            }
        }

        actions
    }
}

/// Thread-safe wrapper for [`WorkspaceState`].
///
/// Provides concurrent access to workspace state using `Arc<RwLock<...>>`.
/// Multiple readers can access simultaneously, but writers get exclusive access.
///
/// # Examples
///
/// ```
/// # use txtx_cli::cli::lsp::workspace::SharedWorkspaceState;
/// # use lsp_types::Url;
/// let workspace = SharedWorkspaceState::new();
///
/// // Read access (can have multiple readers)
/// {
///     let reader = workspace.read();
///     // Use reader...
/// }
///
/// // Write access (exclusive)
/// {
///     let mut writer = workspace.write();
///     let uri = Url::parse("file:///test.tx").unwrap();
///     writer.open_document(uri, "content".to_string());
/// }
/// ```
#[derive(Clone)]
pub struct SharedWorkspaceState {
    inner: Arc<RwLock<WorkspaceState>>,
}

impl SharedWorkspaceState {
    /// Creates a new shared workspace state.
    pub fn new() -> Self {
        Self { inner: Arc::new(RwLock::new(WorkspaceState::new())) }
    }

    /// Acquires a read lock on the workspace state.
    ///
    /// Multiple readers can hold the lock simultaneously. Blocks if a writer
    /// currently holds the lock.
    ///
    /// # Panics
    ///
    /// Panics if the lock is poisoned (a writer panicked while holding the lock).
    pub fn read(&self) -> std::sync::RwLockReadGuard<WorkspaceState> {
        self.inner.read().unwrap()
    }

    /// Acquires a write lock on the workspace state.
    ///
    /// Provides exclusive access. Blocks if any readers or writers currently
    /// hold the lock.
    ///
    /// # Panics
    ///
    /// Panics if the lock is poisoned (a writer panicked while holding the lock).
    pub fn write(&self) -> std::sync::RwLockWriteGuard<WorkspaceState> {
        self.inner.write().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_document_lifecycle() {
        let mut workspace = WorkspaceState::new();
        let uri = Url::parse("file:///test.tx").unwrap();

        // Open document
        workspace.open_document(uri.clone(), "initial content".to_string());
        assert!(workspace.get_document(&uri).is_some());

        // Update document
        workspace.update_document(&uri, "updated content".to_string());
        let doc = workspace.get_document(&uri).unwrap();
        assert_eq!(doc.content(), "updated content");
        assert_eq!(doc.version(), 2);

        // Close document
        workspace.close_document(&uri);
        assert!(workspace.get_document(&uri).is_none());
    }

    #[test]
    fn test_manifest_indexing() {
        let mut workspace = WorkspaceState::new();
        let manifest_uri = Url::parse("file:///project/txtx.yml").unwrap();
        let manifest_content = r#"
runbooks:
  - name: deploy
    location: runbooks/deploy.tx

environments:
  prod:
    api_key: prod_key
        "#;

        workspace.open_document(manifest_uri.clone(), manifest_content.to_string());

        // Check manifest was parsed
        assert!(workspace.get_manifest(&manifest_uri).is_some());

        // Check environment vars were cached
        let prod_vars = workspace.get_environment_vars("prod").unwrap();
        assert_eq!(prod_vars.get("api_key").unwrap(), "prod_key");
    }
}
