# Async LSP Implementation Guide

## Overview

The txtx Language Server Protocol (LSP) implementation uses asynchronous handlers for performance-critical operations, providing better responsiveness and concurrent request handling.

## Architecture

### Request Flow

```
Client Request → LSP Server → Request Router → Async/Sync Handler → Response
```

1. **Heavy Operations** (async): Completion, Hover, Semantic Tokens
2. **Light Operations** (sync): Definitions, References, Diagnostics

## Async Handler Implementation

### Core Components

#### AsyncLspHandler (`async_handler.rs`)

```rust
pub struct AsyncLspHandler {
    cache: Arc<DocumentCache>,
    workspace: Arc<RwLock<WorkspaceState>>,
    handlers: Arc<Handlers>,
}
```

Key features:

- Thread-safe with `Arc` and `RwLock`
- Integrated caching layer
- Cloneable for task spawning

### Adding New Async Handlers

To add a new async handler:

1. **Define the async method**:

```rust
async fn handle_my_feature_async(
    &self,
    id: RequestId,
    params: serde_json::Value,
) -> Option<Response> {
    // Parse parameters
    let my_params: MyParams = serde_json::from_value(params)
        .map_err(|e| eprintln!("Parse error: {}", e))
        .ok()?;

    // Async operations
    let result = self.compute_my_feature(my_params).await?;

    // Return response
    Some(Response::new_ok(id, result))
}
```

2. **Add computation logic**:

```rust
async fn compute_my_feature(
    &self,
    params: MyParams,
) -> Result<MyResult, String> {
    // Read file asynchronously
    let content = tokio::fs::read_to_string(&params.file_path)
        .await
        .map_err(|e| format!("Read error: {}", e))?;

    // Process content (potentially in parallel)
    let processed = self.process_content(&content).await;

    Ok(MyResult { data: processed })
}
```

3. **Route the request**:

```rust
// In async_handler.rs
pub async fn handle_request(&self, req: Request) -> Option<Response> {
    match req.method.as_str() {
        "textDocument/myFeature" => {
            self.handle_my_feature_async(req.id, req.params).await
        }
        // ... other handlers
    }
}
```

## Caching Strategy

### Document Cache

```rust
struct DocumentCache {
    parsed: Arc<DashMap<PathBuf, (Instant, String)>>,
    max_age: Duration,  // 60 seconds default
    completions: Arc<Mutex<LruCache<String, Vec<CompletionItem>>>>,
}
```

### Cache Usage

```rust
// Check cache first
if let Some(cached) = self.cache.get_or_parse(&path).await {
    return Ok(cached);
}

// Compute and cache
let result = expensive_computation().await;
self.cache.insert(key, result.clone());
```

### Cache Invalidation

```rust
// Invalidate specific entry
cache.invalidate(&path);

// Clear all entries
cache.clear();
```

## Parallel Processing

### Parallel Document Parsing

```rust
use futures::future::join_all;

pub async fn parse_documents_parallel(
    &self,
    paths: Vec<PathBuf>
) -> Vec<Result<String, String>> {
    let futures = paths.into_iter().map(|path| {
        async move {
            self.parse_document(&path).await
        }
    });

    join_all(futures).await
}
```

### Concurrent Request Handling

```rust
// In main loop
runtime.spawn(async move {
    let response = handle_request_async(req, &handlers).await;
    if let Some(resp) = response {
        let _ = sender.send(Message::Response(resp));
    }
});
```

## Performance Optimization

### Best Practices

1. **Use async I/O for file operations**:

```rust
// Good
let content = tokio::fs::read_to_string(path).await?;

// Avoid
let content = std::fs::read_to_string(path)?;
```

2. **Cache frequently accessed data**:

```rust
// Check cache before expensive operations
if let Some(cached) = cache.get(&key) {
    return cached;
}
```

3. **Batch operations when possible**:

```rust
// Process multiple files in parallel
let results = join_all(files.iter().map(process_file)).await;
```

4. **Use appropriate data structures**:

- `DashMap` for concurrent access
- `LruCache` for bounded caches
- `Arc<RwLock<T>>` for shared state

### Benchmarking

Run benchmarks to measure performance:

```bash
# Run all benchmarks
cargo bench --package txtx-cli

# Run specific benchmark
cargo bench --package txtx-cli lsp_performance

# Generate HTML report
cargo bench --package txtx-cli -- --save-baseline my_baseline
```

## Debugging

### Logging

Add debug logging for async operations:

```rust
eprintln!("[ASYNC] Starting completion request");
let start = Instant::now();

let result = compute_completion().await;

eprintln!("[ASYNC] Completion took {:?}", start.elapsed());
```

### Tracing

For detailed tracing, use the `tracing` crate:

```rust
use tracing::{instrument, debug};

#[instrument(skip(self))]
async fn compute_completion(&self, params: CompletionParams) -> Result<Vec<CompletionItem>> {
    debug!("Computing completions");
    // ... implementation
}
```

## Common Patterns

### Error Handling

```rust
async fn safe_operation(&self) -> Result<Value, String> {
    tokio::fs::read_to_string(path)
        .await
        .map_err(|e| format!("Failed to read: {}", e))?;

    serde_json::from_str(&content)
        .map_err(|e| format!("Parse error: {}", e))
}
```

### Timeout Handling

```rust
use tokio::time::{timeout, Duration};

async fn with_timeout(&self) -> Result<Value> {
    match timeout(Duration::from_secs(5), expensive_operation()).await {
        Ok(result) => result,
        Err(_) => Err("Operation timed out"),
    }
}
```

### Cancellation

```rust
use tokio_util::sync::CancellationToken;

async fn cancellable_operation(
    &self,
    cancel: CancellationToken,
) -> Result<Value> {
    tokio::select! {
        result = expensive_operation() => result,
        _ = cancel.cancelled() => {
            Err("Operation cancelled")
        }
    }
}
```

## Testing Async Handlers

### Unit Tests

```rust
#[tokio::test]
async fn test_async_completion() {
    let handler = create_test_handler();
    let params = create_completion_params();

    let result = handler.compute_completions(params).await;

    assert!(result.is_ok());
    assert!(!result.unwrap().is_empty());
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_concurrent_requests() {
    let handler = create_test_handler();

    let futures = (0..10).map(|_| {
        let h = handler.clone();
        async move {
            h.handle_request(create_request()).await
        }
    });

    let results = join_all(futures).await;
    assert_eq!(results.len(), 10);
}
```

## Migration Checklist

When converting a sync handler to async:

- [ ] Add `async` keyword to function signatures
- [ ] Replace blocking I/O with async equivalents
- [ ] Add appropriate error handling
- [ ] Implement caching where beneficial
- [ ] Add timeout handling for long operations
- [ ] Update tests to use `#[tokio::test]`
- [ ] Benchmark before and after
- [ ] Document the changes

## Future Improvements

### Planned Enhancements

1. **Incremental Parsing**: Parse only changed portions of documents
2. **Workspace Indexing**: Pre-index symbols for faster lookup
3. **Streaming Responses**: Stream large results incrementally
4. **Request Prioritization**: Handle user-visible requests first
5. **Adaptive Caching**: Adjust cache size based on memory pressure
