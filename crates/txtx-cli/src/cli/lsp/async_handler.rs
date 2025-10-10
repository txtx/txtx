//! Async request handler with caching
//!
//! # C4 Architecture Annotations
//! @c4-component AsyncLspHandler
//! @c4-container LSP Server
//! @c4-description Handles LSP requests concurrently with document caching
//! @c4-technology Rust (tokio async runtime)
//! @c4-responsibility Process LSP requests concurrently
//! @c4-responsibility Cache document parses with TTL and LRU eviction
//! @c4-responsibility Maintain workspace state across requests

#![allow(dead_code)]

use std::sync::Arc;
use std::time::{Duration, Instant};
use std::path::{Path, PathBuf};
use tokio::sync::RwLock;
use dashmap::DashMap;
use lru::LruCache;
use std::num::NonZeroUsize;
use serde_json;
use lsp_server::{Request, Response, RequestId};
use lsp_types::*;

use super::handlers::Handlers;

/// Async LSP handler with caching and concurrent request processing
pub struct AsyncLspHandler {
    cache: Arc<DocumentCache>,
    workspace: Arc<RwLock<WorkspaceState>>,
    handlers: Arc<Handlers>,
}

/// Workspace state shared across async requests
pub struct WorkspaceState {
    pub root_path: PathBuf,
    pub open_files: DashMap<PathBuf, String>,
}

/// Document cache with TTL and LRU eviction
struct DocumentCache {
    parsed: Arc<DashMap<PathBuf, (Instant, String)>>,
    max_age: Duration,
    completions: Arc<tokio::sync::Mutex<LruCache<String, Vec<CompletionItem>>>>,
}

impl AsyncLspHandler {
    pub fn new(handlers: Handlers, root_path: PathBuf) -> Self {
        let cache = DocumentCache {
            parsed: Arc::new(DashMap::new()),
            max_age: Duration::from_secs(60), // 1 minute cache
            completions: Arc::new(tokio::sync::Mutex::new(
                LruCache::new(NonZeroUsize::new(100).unwrap())
            )),
        };

        let workspace = WorkspaceState {
            root_path,
            open_files: DashMap::new(),
        };

        Self {
            cache: Arc::new(cache),
            workspace: Arc::new(RwLock::new(workspace)),
            handlers: Arc::new(handlers),
        }
    }

    pub async fn handle_request(
        &self,
        req: Request,
    ) -> Option<Response> {
        match req.method.as_str() {
            "textDocument/completion" => {
                self.handle_completion_async(req.id, req.params).await
            }
            "textDocument/hover" => {
                self.handle_hover_async(req.id, req.params).await
            }
            "textDocument/didOpen" | "textDocument/didChange" => {
                self.handle_document_change_async(req.id, req.params).await
            }
            _ => {
                self.handle_sync(req)
            }
        }
    }

    async fn handle_completion_async(
        &self,
        id: RequestId,
        params: serde_json::Value,
    ) -> Option<Response> {
        // Check cache first
        let cache_key = format!("{:?}", params);

        {
            let mut cache = self.cache.completions.lock().await;
            if let Some(cached) = cache.get(&cache_key) {
                return Some(Response::new_ok(id, cached.clone()));
            }
        }

        let completions = self.compute_completions(params).await.unwrap_or_default();

        {
            let mut cache = self.cache.completions.lock().await;
            cache.put(cache_key, completions.clone());
        }

        Some(Response::new_ok(id, completions))
    }

    async fn handle_hover_async(
        &self,
        id: RequestId,
        params: serde_json::Value,
    ) -> Option<Response> {
        let hover_info = self.compute_hover(params).await.ok()?;
        Some(Response::new_ok(id, hover_info))
    }

    async fn handle_document_change_async(
        &self,
        id: RequestId,
        params: serde_json::Value,
    ) -> Option<Response> {
        let _ = self.update_document(params).await;
        Some(Response::new_ok(id, ()))
    }

    fn handle_sync(&self, req: Request) -> Option<Response> {
        Some(Response::new_ok(req.id, serde_json::Value::Null))
    }

    async fn compute_completions(
        &self,
        params: serde_json::Value,
    ) -> Result<Vec<CompletionItem>, String> {
        // Parse completion params
        let completion_params: CompletionParams = serde_json::from_value(params)
            .map_err(|e| format!("Failed to parse completion params: {}", e))?;

        // Get document content asynchronously
        let uri = completion_params.text_document_position.text_document.uri.clone();
        let path = uri.to_file_path()
            .map_err(|_| "Invalid file URI")?;

        // Read document content with async I/O
        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| format!("Failed to read file: {}", e))?;

        // Check if we're after "input."
        let position = &completion_params.text_document_position.position;
        if !self.is_after_input_dot(&content, position) {
            return Ok(vec![]);
        }

        // Get workspace state
        let _workspace = self.workspace.read().await;

        // Collect available inputs (this could be parallelized further)
        let mut inputs = std::collections::HashSet::new();

        // In a real implementation, we'd get the manifest for this runbook
        // For now, return some example completions
        inputs.insert("api_key".to_string());
        inputs.insert("region".to_string());
        inputs.insert("environment".to_string());

        // Create completion items
        let items: Vec<CompletionItem> = inputs
            .into_iter()
            .map(|input| CompletionItem {
                label: input.clone(),
                kind: Some(CompletionItemKind::VARIABLE),
                detail: Some(format!("Input variable: {}", input)),
                ..Default::default()
            })
            .collect();

        Ok(items)
    }

    fn is_after_input_dot(&self, content: &str, position: &Position) -> bool {
        let lines: Vec<&str> = content.lines().collect();
        if let Some(line) = lines.get(position.line as usize) {
            if position.character >= 6 {
                let start = (position.character - 6) as usize;
                let end = position.character as usize;
                if let Some(slice) = line.get(start..end) {
                    return slice == "input.";
                }
            }
        }
        false
    }

    async fn compute_hover(
        &self,
        params: serde_json::Value,
    ) -> Result<Option<Hover>, String> {
        // Parse hover params
        let hover_params: HoverParams = serde_json::from_value(params)
            .map_err(|e| format!("Failed to parse hover params: {}", e))?;

        // Get document content asynchronously
        let uri = hover_params.text_document_position_params.text_document.uri.clone();
        let path = uri.to_file_path()
            .map_err(|_| "Invalid file URI")?;

        // Read document content with async I/O
        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| format!("Failed to read file: {}", e))?;

        // Get the word at position
        let position = &hover_params.text_document_position_params.position;
        let word = self.get_word_at_position(&content, position);

        if let Some(word) = word {
            // Check if it's an input reference
            if word.starts_with("input.") {
                let input_name = &word[6..];

                // Create hover content
                let hover_content = format!(
                    "**Input Variable**: `{}`\n\nThis references an input variable defined in the manifest.",
                    input_name
                );

                let hover = Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: hover_content,
                    }),
                    range: None,
                };

                return Ok(Some(hover));
            }
        }

        Ok(None)
    }

    fn get_word_at_position(&self, content: &str, position: &Position) -> Option<String> {
        let lines: Vec<&str> = content.lines().collect();
        if let Some(line) = lines.get(position.line as usize) {
            let char_pos = position.character as usize;

            // Find word boundaries
            let mut start = char_pos;
            let mut end = char_pos;

            // Move start back to beginning of word
            while start > 0 && line.chars().nth(start - 1)
                .map_or(false, |c| c.is_alphanumeric() || c == '.' || c == '_')
            {
                start -= 1;
            }

            // Move end forward to end of word
            while end < line.len() && line.chars().nth(end)
                .map_or(false, |c| c.is_alphanumeric() || c == '.' || c == '_')
            {
                end += 1;
            }

            if start < end {
                return Some(line[start..end].to_string());
            }
        }
        None
    }

    async fn update_document(
        &self,
        _params: serde_json::Value,
    ) -> Result<(), String> {
        Ok(())
    }
}

impl DocumentCache {
    async fn get_or_parse(&self, path: &Path) -> Result<String, String> {
        if let Some(entry) = self.parsed.get(path) {
            if entry.0.elapsed() < self.max_age {
                return Ok(entry.1.clone());
            }
        }

        let parsed = self.parse_document_async(path).await?;
        self.parsed.insert(path.to_owned(), (Instant::now(), parsed.clone()));
        Ok(parsed)
    }

    async fn parse_document_async(&self, path: &Path) -> Result<String, String> {
        tokio::fs::read_to_string(path)
            .await
            .map_err(|e| format!("Failed to read document: {}", e))
    }

    /// Parse multiple documents in parallel
    pub async fn parse_documents_parallel(&self, paths: Vec<PathBuf>) -> Vec<Result<String, String>> {
        use futures::future::join_all;

        let futures = paths.into_iter().map(|path| {
            async move {
                self.get_or_parse(&path).await
            }
        });

        join_all(futures).await
    }

    /// Invalidate cache entry for a specific path
    pub fn invalidate(&self, path: &Path) {
        self.parsed.remove(path);
    }

    /// Clear all cached documents
    pub fn clear(&self) {
        self.parsed.clear();
    }
}