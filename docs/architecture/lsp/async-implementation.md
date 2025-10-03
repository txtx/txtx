# LSP Async Architecture

## Overview

The LSP implementation features true async handlers for performance-critical operations, improving responsiveness and enabling concurrent request handling.

## Architecture Diagram

```text
                    ┌──────────────┐
                    │   VS Code    │
                    └──────┬───────┘
                           │ JSON-RPC
                    ┌──────▼───────┐
                    │  lsp_server  │
                    │  (Message Loop)│
                    └──────┬───────┘
                           │
                ┌──────────▼──────────┐
                │   Request Router    │
                └─────┬──────────┬────┘
                      │          │
            Heavy Ops │          │ Light Ops
                      │          │
         ┌────────────▼───┐  ┌──▼──────────┐
         │  Async Handler │  │Sync Handler │
         │  (Tokio Tasks) │  │  (Direct)   │
         └────────────────┘  └─────────────┘
                 │
         ┌───────▼────────┐
         │  Cache Layer   │
         │ (DashMap, LRU) │
         └────────────────┘
```

## Components

### 1. Message Loop (`mod.rs`)

- Uses `lsp_server` for robust protocol handling
- Routes requests based on computational weight
- Spawns Tokio tasks for heavy operations

### 2. Async Handler (`async_handler.rs`)

- Handles completion, hover, and document operations
- Uses `tokio::fs` for async file I/O
- Implements caching for performance

### 3. Cache Layer

- **Document Cache**: 60-second TTL for parsed documents
- **Completion Cache**: LRU with 100-item limit
- **Concurrent Access**: DashMap for thread-safe operations

## Performance Features

### Async I/O

**Before (Blocking):**

```rust
let content = std::fs::read_to_string(path)?;
```

**After (Async):**

```rust
let content = tokio::fs::read_to_string(path).await?;
```

### Parallel Document Parsing

```rust
// Parse multiple documents concurrently
let documents = cache.parse_documents_parallel(paths).await;
```

### Smart Caching

```rust
// Cache with TTL
if let Some(cached) = cache.get_or_parse(&path).await {
    return cached;
}
```

## Performance Metrics

### Request Flow Comparison

**Before (Synchronous)**:

```text
Request → Block Thread → Read File → Process → Response
         └── Thread blocked for entire duration ──┘
```

**After (Asynchronous)**:

```text
Request → Spawn Task → Async Read → Process → Response
         └── Thread free to handle other requests ──┘
```

### Operation Latencies

| Operation | Sync (ms) | Async (ms) | Improvement | With Cache |
|-----------|-----------|------------|-------------|------------|
| Completion | 50-100 | 25-50 | ~50% | 5-10ms |
| Hover | 30-60 | 15-30 | ~50% | 3-5ms |
| Document Parse | 100-200 | 100-200 | - | 0ms (cached) |
| Multi-file (10) | 1000 | 400 | ~60% | 50ms |

*Estimated improvements; actual results depend on file size and system I/O*

### Memory Efficiency

#### Cache Characteristics

| Cache Type | Size Limit | TTL | Memory Impact |
|------------|------------|-----|---------------|
| Document Cache | Unlimited* | 60s | ~10-50MB |
| Completion Cache | 100 items | None | ~1-5MB |
| Parse Cache | Per session | 60s | ~5-20MB |

*Documents auto-expire after 60 seconds, preventing unbounded growth*

#### Memory Usage Profile

```text
Startup:      ~50MB
After 1 hour: ~80MB (with caching)
Peak usage:   ~150MB (heavy load)
Idle state:   ~60MB (caches expired)
```

## Benefits

### 1. Non-blocking I/O

Editor remains responsive during file operations.

### 2. Concurrent Request Handling

Multiple requests can be processed simultaneously.

### 3. Reduced Latency

Caching and async I/O reduce response times by ~50%.

### 4. Bounded Memory

TTL-based caching prevents memory growth.

## Implementation Details

### Request Routing

Heavy operations (completion, hover) use async handlers:

```rust
match method.as_str() {
    "textDocument/completion" => spawn_async_task(handle_completion),
    "textDocument/hover" => spawn_async_task(handle_hover),
    "textDocument/definition" => handle_sync(handle_definition), // Fast lookup
    // ...
}
```

### Cache Management

```rust
pub struct DocumentCache {
    documents: DashMap<Url, CachedDocument>,
    completions: LruCache<CompletionKey, Vec<CompletionItem>>,
}

struct CachedDocument {
    content: String,
    parsed: Body,
    timestamp: Instant,
}
```

### Concurrency

DashMap provides lock-free concurrent access:

```rust
// Multiple threads can read concurrently
let doc1 = cache.get(&url1);
let doc2 = cache.get(&url2);
```

## Workspace State Machine

The LSP server uses an explicit state machine to coordinate workspace-level operations and provide observability into the server's behavior.

### State Diagram

```text
Uninitialized -> Indexing -> Ready
                       ↓         ↑
                 IndexingError   |
                       ↓         |
                   Indexing -----+

Ready -> Validating -> Ready
  ↓         ↓           ↑
  ↓    ValidationError  |
  ↓         ↓           |
  ↓      Validating ----+
  ↓
  +-> EnvironmentChanging -> Revalidating -> Ready
  ↓
  +-> DependencyResolving -> Invalidating -> Revalidating -> Ready
```

### States

| State | Description | Can Accept Requests? |
|-------|-------------|---------------------|
| **Uninitialized** | Before LSP initialization | No |
| **Indexing** | Discovering manifests and runbooks | No |
| **IndexingError** | Failed to index workspace | No |
| **Ready** | Idle, ready for requests | **Yes** |
| **Validating** | Validating single document | No |
| **EnvironmentChanging** | Switching txtx environment | No |
| **Revalidating** | Re-validating multiple documents | No |
| **DependencyResolving** | Resolving cross-file dependencies | No |
| **Invalidating** | Marking documents for re-validation | No |

### State Events

Events trigger state transitions:

- `ServerInitialized` → Start indexing workspace
- `DocumentOpened` → Trigger validation for new document
- `DocumentChanged` → Validate changed document
- `EnvironmentSwitched` → Re-validate all documents with new environment
- `ValidationCompleted` → Return to Ready state
- `IndexingCompleted` → Workspace ready

### Benefits

1. **Observability**: Explicit states make server behavior visible
2. **Debugging**: State history tracks what led to current state
3. **Request Handling**: Only accept new requests when Ready
4. **Coordination**: Prevents concurrent validation conflicts

### Implementation

```rust
pub enum MachineState {
    Uninitialized,
    Indexing,
    Ready,
    Validating { document: Url },
    EnvironmentChanging { new_env: String },
    Revalidating { documents: Vec<Url>, current: usize },
    // ...
}
```

See `crates/txtx-cli/src/cli/lsp/workspace/state_machine.rs` for full implementation.

## Documenting Validation Behavior

The linter includes a **documentation format** (`--format doc`) designed for creating shareable examples that show validation errors with visual indicators:

```bash
txtx lint example.tx --format doc
```

**Example output:**

```text
example.tx:

  6 │ action "deploy" {
  7 │   constructor_args = [
  8 │     flow.missing_field
    │     ^^^^^^^^^^^^^ error: Undefined flow input 'missing_field'
  9 │   ]
 10 │ }
```

This format is ideal for:

- **Documentation**: Include in architecture docs to show validation behavior
- **Bug reports**: Share working or breaking examples with error context
- **Testing**: Capture expected validation output for test cases
- **Education**: Demonstrate txtx validation rules with real examples

The formatter automatically:

- Shows 2 lines of context before and after each error
- Aligns line numbers for readability
- Uses caret indicators (`^^^`) to point to error locations
- Groups errors by file
- Skips irrelevant lines (shown with `⋮`)

## See Also

- [Performance Improvements](../performance-improvements.md) - Detailed benchmarks
- [State Management](../../lsp-state-management.md) - State machine architecture
- [ADR 002: Eliminate LSP Server Crate](../../adr/002-eliminate-lsp-server-crate.md)
