mod diagnostics;
mod diagnostics_enhanced;
mod diagnostics_multi_file;
mod functions;
mod handlers;
mod utils;
mod workspace;

mod diagnostics_hcl_integrated;

mod multi_file;
mod validation;

#[cfg(test)]
mod tests;

use lsp_server::{Connection, Message, Request, Response};
use lsp_types::{
    CompletionOptions, InitializeParams, OneOf, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind,
};
use std::error::Error;

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

    // Main message loop
    for message in &connection.receiver {
        match message {
            Message::Request(req) => {
                eprintln!("Received request: {}", req.method);

                // Handle shutdown request
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }

                // Route the request to appropriate handler
                let response = handle_request(req, &handlers);
                if let Some(resp) = response {
                    connection.sender.send(Message::Response(resp))?;
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
        "workspace/environments" => {
            eprintln!("[DEBUG] Received workspace/environments request");
            let environments = handlers.workspace.get_environments();
            Some(Response::new_ok(req.id, environments))
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

            // Send diagnostics (always, even if empty)
            let diagnostics = handlers.diagnostics.get_diagnostics(&uri);
            let params = lsp_types::PublishDiagnosticsParams { uri, diagnostics, version: None };
            let notification = lsp_server::Notification {
                method: "textDocument/publishDiagnostics".to_string(),
                params: serde_json::to_value(params)?,
            };
            connection.sender.send(Message::Notification(notification))?;
        }
        "textDocument/didChange" => {
            let params: lsp_types::DidChangeTextDocumentParams =
                serde_json::from_value(not.params)?;
            let uri = params.text_document.uri.clone();
            handlers.document_sync.did_change(params);

            // Send diagnostics (always, even if empty)
            let diagnostics = handlers.diagnostics.get_diagnostics(&uri);
            let params = lsp_types::PublishDiagnosticsParams { uri, diagnostics, version: None };
            let notification = lsp_server::Notification {
                method: "textDocument/publishDiagnostics".to_string(),
                params: serde_json::to_value(params)?,
            };
            connection.sender.send(Message::Notification(notification))?;
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
            let workspace_handler = &handlers.workspace;
            let workspace = workspace_handler.workspace_state().read();
            let document_uris: Vec<lsp_types::Url> =
                workspace.documents().keys().cloned().collect();
            drop(workspace); // Release the lock before re-validation

            eprintln!("[DEBUG] Re-validating {} documents", document_uris.len());
            for uri in document_uris {
                let current_env = handlers.workspace.get_current_environment();
                let diagnostics =
                    handlers.diagnostics.get_diagnostics_with_env(&uri, current_env.as_deref());
                let params =
                    lsp_types::PublishDiagnosticsParams { uri, diagnostics, version: None };
                let notification = lsp_server::Notification {
                    method: "textDocument/publishDiagnostics".to_string(),
                    params: serde_json::to_value(params)?,
                };
                connection.sender.send(Message::Notification(notification))?;
            }
        }
        _ => {
            eprintln!("Unhandled notification: {}", not.method);
        }
    }
    Ok(())
}
