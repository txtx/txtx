//! Language Server Protocol implementation
//!
//! # C4 Architecture Annotations
//! @c4-component LSP Server
//! @c4-container txtx-cli
//! @c4-description Provides real-time IDE diagnostics and code intelligence
//! @c4-technology Rust (LSP Protocol)
//! @c4-uses AsyncLspHandler "For concurrent request processing"
//! @c4-uses WorkspaceState "For shared workspace state"
//! @c4-uses Linter Engine "For validation via linter adapter"
//! @c4-responsibility Handle LSP protocol messages over stdin/stdout
//! @c4-responsibility Initialize server capabilities
//! @c4-responsibility Coordinate async request handlers

mod async_handler;
mod diagnostics;
mod linter_adapter;
mod diagnostics_multi_file;
mod functions;
mod handlers;
mod hcl_ast;
mod utils;
mod workspace;

mod diagnostics_hcl_integrated;

mod multi_file;
mod validation;

#[cfg(test)]
mod tests;

use lsp_server::{Connection, Message, Request, Response};
use lsp_types::{
    CompletionOptions, DiagnosticOptions, DiagnosticServerCapabilities, InitializeParams, OneOf,
    ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind, Url, WorkDoneProgressOptions,
};
use std::error::Error;

use self::async_handler::AsyncLspHandler;
use self::handlers::Handlers;
use self::workspace::SharedWorkspaceState;

/// Run the Language Server Protocol server
pub fn run_lsp() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Use stderr for logging so it doesn't interfere with LSP protocol on stdout
    eprintln!("Starting txtx Language Server");

    // Create the connection over stdin/stdout
    let (connection, io_threads) = Connection::stdio();

    // Wait for the initialize request
    let init_result = connection.initialize_start();
    let (initialize_id, initialize_params) = match init_result {
        Ok(params) => params,
        Err(e) => {
            eprintln!("Failed to receive initialize request: {:?}", e);
            return Err(Box::new(e));
        }
    };

    let initialize_params: InitializeParams = serde_json::from_value(initialize_params)?;

    eprintln!("Initialize params: {:?}", initialize_params.root_uri);

    // Check for initialization options (e.g., selected environment)
    let initial_environment = if let Some(init_options) = &initialize_params.initialization_options {
        eprintln!("Initialization options: {:?}", init_options);

        // Try to extract environment from initialization options
        init_options.get("environment")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    } else {
        None
    };

    // Build server capabilities
    let server_capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        definition_provider: Some(OneOf::Left(true)),
        hover_provider: Some(lsp_types::HoverProviderCapability::Simple(true)),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec![".".to_string()]),
            ..Default::default()
        }),
        references_provider: Some(OneOf::Left(true)),
        rename_provider: Some(OneOf::Right(lsp_types::RenameOptions {
            prepare_provider: Some(true),
            work_done_progress_options: Default::default(),
        })),
        execute_command_provider: Some(lsp_types::ExecuteCommandOptions {
            commands: vec![
                "txtx.getAllRunbookFiles".to_string(),
                "txtx.validateRunbook".to_string(),
            ],
            work_done_progress_options: Default::default(),
        }),
        diagnostic_provider: Some(DiagnosticServerCapabilities::Options(DiagnosticOptions {
            identifier: Some("txtx-linter".to_string()),
            inter_file_dependencies: true,  // We have multi-file runbooks
            workspace_diagnostics: true,     // We support workspace diagnostics
            work_done_progress_options: WorkDoneProgressOptions::default(),
        })),

        ..Default::default()
    };

    let initialize_result = lsp_types::InitializeResult {
        capabilities: server_capabilities,
        server_info: Some(lsp_types::ServerInfo {
            name: "txtx-language-server".to_string(),
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
        }),
    };

    // Complete initialization
    connection.initialize_finish(initialize_id, serde_json::to_value(initialize_result)?)?;

    eprintln!("LSP server initialized successfully");

    // Create shared workspace state and handlers
    let workspace = SharedWorkspaceState::new();
    let handlers = Handlers::new(workspace);

    // Set initial environment if provided
    if let Some(env) = initial_environment {
        eprintln!("Setting initial environment to: {}", env);
        handlers.workspace.set_environment(env);
    } else {
        eprintln!("No initial environment provided, checking for stored environment...");
        // VS Code might send the environment in a notification after initialization
        // For now, we'll default to checking if sepolia exists and use it, otherwise global
        let _workspace_state = handlers.workspace.workspace_state();
        let available_envs = handlers.workspace.get_environments();

        // Check if 'sepolia' exists and prefer it over 'global'
        if available_envs.contains(&"sepolia".to_string()) {
            eprintln!("Found 'sepolia' environment, using it as default");
            handlers.workspace.set_environment("sepolia".to_string());
        } else if !available_envs.is_empty() {
            // Use the first non-global environment if available
            if let Some(env) = available_envs.iter().find(|e| *e != "global") {
                eprintln!("Using first available environment: {}", env);
                handlers.workspace.set_environment(env.clone());
            }
        }
    }

    let runtime = tokio::runtime::Runtime::new()?;

    for message in &connection.receiver {
        match message {
            Message::Request(req) => {
                eprintln!("Received request: {}", req.method);

                // Handle shutdown request
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }

                let is_heavy = matches!(
                    req.method.as_str(),
                    "textDocument/completion" | "textDocument/semanticTokens/full"
                );

                if is_heavy {
                    let handlers_clone = handlers.clone();
                    let sender = connection.sender.clone();

                    runtime.spawn(async move {
                        let response = handle_request_async(req, &handlers_clone).await;
                        if let Some(resp) = response {
                            let _ = sender.send(Message::Response(resp));
                        }
                    });
                } else {
                    let response = handle_request(req, &handlers);
                    if let Some(resp) = response {
                        connection.sender.send(Message::Response(resp))?;
                    }
                }
            }
            Message::Notification(not) => {
                eprintln!("Received notification: {}", not.method);
                handle_notification(not, &handlers, &connection)?;
            }
            Message::Response(_) => {
                // We don't send requests, so we shouldn't get responses
                eprintln!("Unexpected response received");
            }
        }
    }

    // Join the IO threads
    io_threads.join()?;

    eprintln!("LSP server shutting down");
    Ok(())
}

fn handle_request(req: Request, handlers: &Handlers) -> Option<Response> {
    match req.method.as_str() {
        "textDocument/definition" => {
            let params: lsp_types::GotoDefinitionParams = match serde_json::from_value(req.params) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Failed to parse definition params: {}", e);
                    return Some(Response::new_err(
                        req.id,
                        lsp_server::ErrorCode::InvalidParams as i32,
                        "Invalid parameters".to_string(),
                    ));
                }
            };

            let result = handlers.definition.goto_definition(params);
            Some(Response::new_ok(req.id, result))
        }
        "textDocument/hover" => {
            let params: lsp_types::HoverParams = match serde_json::from_value(req.params) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Failed to parse hover params: {}", e);
                    return Some(Response::new_err(
                        req.id,
                        lsp_server::ErrorCode::InvalidParams as i32,
                        "Invalid parameters".to_string(),
                    ));
                }
            };

            let result = handlers.hover.hover(params);
            Some(Response::new_ok(req.id, result))
        }
        "textDocument/completion" => {
            let params: lsp_types::CompletionParams = match serde_json::from_value(req.params) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Failed to parse completion params: {}", e);
                    return Some(Response::new_err(
                        req.id,
                        lsp_server::ErrorCode::InvalidParams as i32,
                        "Invalid parameters".to_string(),
                    ));
                }
            };

            let result = handlers.completion.completion(params);
            Some(Response::new_ok(req.id, result))
        }
        "textDocument/references" => {
            let params: lsp_types::ReferenceParams = match serde_json::from_value(req.params) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Failed to parse references params: {}", e);
                    return Some(Response::new_err(
                        req.id,
                        lsp_server::ErrorCode::InvalidParams as i32,
                        "Invalid parameters".to_string(),
                    ));
                }
            };

            let result = handlers.references.find_references(params);
            Some(Response::new_ok(req.id, result))
        }
        "textDocument/prepareRename" => {
            let params: lsp_types::TextDocumentPositionParams = match serde_json::from_value(req.params) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Failed to parse prepareRename params: {}", e);
                    return Some(Response::new_err(
                        req.id,
                        lsp_server::ErrorCode::InvalidParams as i32,
                        "Invalid parameters".to_string(),
                    ));
                }
            };

            eprintln!("[PrepareRename] URI: {:?}, Position: {:?}", params.text_document.uri, params.position);

            let result = handlers.rename.prepare_rename(params);
            eprintln!("[PrepareRename] Result: {:?}", result);
            Some(Response::new_ok(req.id, result))
        }
        "textDocument/rename" => {
            let params: lsp_types::RenameParams = match serde_json::from_value(req.params) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Failed to parse rename params: {}", e);
                    return Some(Response::new_err(
                        req.id,
                        lsp_server::ErrorCode::InvalidParams as i32,
                        "Invalid parameters".to_string(),
                    ));
                }
            };

            eprintln!("[Rename] URI: {:?}, Position: {:?}, New name: {}",
                     params.text_document_position.text_document.uri,
                     params.text_document_position.position,
                     params.new_name);

            let result = handlers.rename.rename(params);
            eprintln!("[Rename] Result: {:?}", result.is_some());
            Some(Response::new_ok(req.id, result))
        }
        "workspace/environments" => {
            eprintln!("[DEBUG] Received workspace/environments request");
            let environments = handlers.workspace.get_environments();
            Some(Response::new_ok(req.id, environments))
        }
        "workspace/diagnostic" => {
            eprintln!("[DEBUG] Received workspace/diagnostic request");
            let result = handle_workspace_diagnostics(handlers);
            Some(Response::new_ok(req.id, result))
        }
        _ => {
            eprintln!("Unhandled request: {}", req.method);
            Some(Response::new_err(
                req.id,
                lsp_server::ErrorCode::MethodNotFound as i32,
                format!("Method not found: {}", req.method),
            ))
        }
    }
}

/// Handles workspace/diagnostic request to return diagnostics for all files in the workspace.
///
/// This implements LSP 3.17's pull-based workspace diagnostics.
fn handle_workspace_diagnostics(handlers: &Handlers) -> lsp_types::WorkspaceDiagnosticReportResult {
    use lsp_types::{
        FullDocumentDiagnosticReport, WorkspaceDocumentDiagnosticReport,
        WorkspaceFullDocumentDiagnosticReport, WorkspaceDiagnosticReport,
        WorkspaceDiagnosticReportResult,
    };

    // Get all documents from workspace
    let all_docs = {
        let workspace = handlers.workspace.workspace_state().read();
        workspace.get_all_document_uris()
    };

    eprintln!("[DEBUG] Workspace diagnostics: scanning {} documents", all_docs.len());

    let items: Vec<WorkspaceDocumentDiagnosticReport> = all_docs
        .into_iter()
        .flat_map(|uri| {
            let diagnostics_by_file = handlers.diagnostics.get_diagnostics_with_env(&uri, None);

            diagnostics_by_file.into_iter().map(|(file_uri, diagnostics)| {
                eprintln!("[DEBUG]   {} has {} diagnostics", file_uri, diagnostics.len());

                WorkspaceDocumentDiagnosticReport::Full(
                    WorkspaceFullDocumentDiagnosticReport {
                        uri: file_uri,
                        version: None,
                        full_document_diagnostic_report: FullDocumentDiagnosticReport {
                            result_id: None,
                            items: diagnostics,
                        },
                    }
                )
            })
        })
        .collect();

    eprintln!("[DEBUG] Returning {} diagnostic reports", items.len());
    WorkspaceDiagnosticReportResult::Report(WorkspaceDiagnosticReport { items })
}

/// Publishes diagnostics for a document to the LSP client.
///
/// Creates a `textDocument/publishDiagnostics` notification and sends it through
/// the LSP connection. This is the final step in the validation pipeline.
///
/// # Arguments
///
/// * `connection` - The LSP connection to send the notification through
/// * `uri` - The URI of the document the diagnostics are for
/// * `diagnostics` - The diagnostics to publish (can be empty)
///
/// # Errors
///
/// Returns an error if JSON serialization fails or the notification cannot be sent.
fn publish_diagnostics(
    connection: &Connection,
    uri: Url,
    diagnostics: Vec<lsp_types::Diagnostic>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let params = lsp_types::PublishDiagnosticsParams { uri, diagnostics, version: None };
    let notification = lsp_server::Notification {
        method: "textDocument/publishDiagnostics".to_string(),
        params: serde_json::to_value(params)?,
    };
    connection.sender.send(Message::Notification(notification))?;
    Ok(())
}

/// Validates a document and publishes its diagnostics.
///
/// This helper combines validation and diagnostic publishing into a single operation.
/// It validates the document using the current environment context, updates the
/// workspace's validation cache, and publishes the results to the LSP client.
///
/// # Arguments
///
/// * `handlers` - The LSP handlers containing the diagnostics handler
/// * `connection` - The LSP connection for publishing diagnostics
/// * `uri` - The URI of the document to validate
/// * `environment` - Optional environment name for context-aware validation
///
/// # Errors
///
/// Returns an error if validation fails or diagnostics cannot be published.
fn validate_and_publish(
    handlers: &Handlers,
    connection: &Connection,
    uri: &Url,
    environment: Option<&str>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let diagnostics_by_file = handlers.diagnostics.validate_and_update_state(uri, environment);

    eprintln!("[DEBUG] Publishing diagnostics to {} files", diagnostics_by_file.len());

    // Publish diagnostics to all affected files
    for (file_uri, diagnostics) in diagnostics_by_file {
        eprintln!("[DEBUG]   Publishing {} diagnostics to {}", diagnostics.len(), file_uri);
        publish_diagnostics(connection, file_uri, diagnostics)?;
    }

    Ok(())
}

fn handle_notification(
    not: lsp_server::Notification,
    handlers: &Handlers,
    connection: &Connection,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match not.method.as_str() {
        "textDocument/didOpen" => {
            let params: lsp_types::DidOpenTextDocumentParams = serde_json::from_value(not.params)?;
            let uri = params.text_document.uri.clone();
            handlers.document_sync.did_open(params);

            let current_env = handlers.workspace.get_current_environment();
            validate_and_publish(handlers, connection, &uri, current_env.as_deref())?;
        }
        "textDocument/didChange" => {
            let params: lsp_types::DidChangeTextDocumentParams =
                serde_json::from_value(not.params)?;
            let uri = params.text_document.uri.clone();
            handlers.document_sync.did_change(params);

            let current_env = handlers.workspace.get_current_environment();

            // Validate the changed document
            validate_and_publish(handlers, connection, &uri, current_env.as_deref())?;

            // Cascade validation: validate all dirty dependents
            let dirty_docs = handlers.diagnostics.get_dirty_documents();
            for dirty_uri in dirty_docs {
                validate_and_publish(handlers, connection, &dirty_uri, current_env.as_deref())?;
            }
        }
        "textDocument/didSave" => {
            let _params: lsp_types::DidSaveTextDocumentParams = serde_json::from_value(not.params)?;
            // Currently a no-op, but could trigger validation
        }
        "textDocument/didClose" => {
            let params: lsp_types::DidCloseTextDocumentParams = serde_json::from_value(not.params)?;
            handlers.document_sync.did_close(params);
        }
        "workspace/setEnvironment" => {
            let params: handlers::workspace::SetEnvironmentParams =
                serde_json::from_value(not.params)?;
            eprintln!("[DEBUG] Received setEnvironment notification: {:?}", params);
            handlers.workspace.set_environment(params.environment.clone());

            // Re-validate all open documents with the new environment
            let document_uris: Vec<Url> = {
                let workspace = handlers.workspace.workspace_state().read();
                workspace.documents().keys().cloned().collect()
            };

            let current_env = handlers.workspace.get_current_environment();
            eprintln!("[DEBUG] Re-validating {} documents", document_uris.len());
            for uri in document_uris {
                validate_and_publish(handlers, connection, &uri, current_env.as_deref())?;
            }
        }
        _ => {
            eprintln!("Unhandled notification: {}", not.method);
        }
    }
    Ok(())
}

/// Handle requests asynchronously for heavy computation operations
///
/// This provides true async implementations for performance-critical operations
async fn handle_request_async(req: Request, handlers: &Handlers) -> Option<Response> {
    match req.method.as_str() {
        "textDocument/completion" | "textDocument/hover" => {
            // Use async handler for these operations
            let root_path = std::env::current_dir().unwrap_or_default();
            let async_handler = AsyncLspHandler::new(handlers.clone(), root_path);
            async_handler.handle_request(req).await
        }
        "textDocument/semanticTokens/full" => {
            // For now, still delegate to sync handler
            // This can be made async in a future iteration
            handle_request(req, handlers)
        }
        _ => handle_request(req, handlers),
    }
}
