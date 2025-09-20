# txtx Language Server Protocol (LSP) Architecture

## Overview

The txtx LSP is implemented as a modular, synchronous language server integrated directly into the txtx CLI. It follows rust-analyzer's architecture patterns, using `lsp-server` for protocol handling and providing real-time IDE support for txtx runbooks.

## Architecture

### High-Level Design

```console
VSCode ←→ stdio ←→ txtx lsp ←→ lsp-server ←→ Handler System
                                                ↓
                                          Workspace State
                                                ↓
                                          HCL Validation
```

### Module Structure

```console
crates/txtx-cli/src/cli/lsp/
├── mod.rs                  # Entry point, message loop, request routing
├── handlers/               # Modular request handlers
│   ├── mod.rs             # Handler traits and registry
│   ├── completion.rs      # Auto-completion logic
│   ├── definition.rs      # Go-to-definition implementation
│   ├── hover.rs           # Hover information provider
│   ├── diagnostics.rs     # Diagnostics handler (has TODOs)
│   └── document_sync.rs   # Document lifecycle management
├── workspace/              # State management
│   ├── mod.rs            # Workspace module exports
│   ├── state.rs          # Central workspace state (RwLock)
│   ├── documents.rs      # Document tracking
│   └── manifests.rs      # Manifest parsing and caching
├── validation/             # Validation integration
│   ├── adapter.rs        # Doctor validation adapter
│   └── converter.rs      # Diagnostic conversion
├── diagnostics.rs          # HCL validation implementation
├── functions.rs            # Function documentation database
└── utils.rs               # Shared utilities
```

## Core Components

### 1. Main Entry Point ([`mod.rs`](crates/txtx-cli/src/cli/lsp/mod.rs))

The synchronous message loop that processes LSP requests:

```rust
pub fn run_lsp_server() -> Result<(), Box<dyn std::error::Error>> {
    let (connection, io_threads) = Connection::stdio();
    let capabilities = lsp_types::ServerCapabilities { 
        // ... capabilities
    };
    
    connection.initialize(capabilities)?;
    
    // Main message loop
    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => handle_request(req, &backend),
            Message::Notification(not) => handle_notification(not, &backend),
            _ => {}
        }
    }
}
```

### 2. Handler System ([`handlers/`](crates/txtx-cli/src/cli/lsp/handlers/))

#### Base Traits

```rust
// Base handler trait
pub trait Handler: Send + Sync {
    fn workspace(&self) -> &SharedWorkspaceState;
}

// Text document handler trait
pub trait TextDocumentHandler: Handler {
    fn get_document_at_position(&self, params: &TextDocumentPositionParams) 
        -> Option<(Url, String, Position)>;
}
```

#### Handler Implementations

- **CompletionHandler**: Provides input variable completions
- **DefinitionHandler**: Jumps to variable definitions in txtx.yml
- **HoverHandler**: Shows documentation for functions, actions, and variables
- **DiagnosticsHandler**: Manages error diagnostics (currently returns empty diagnostics with TODOs for doctor integration)
- **DocumentSyncHandler**: Tracks document changes and triggers validation via `diagnostics::validate_runbook()`

### 3. Workspace State ([`workspace/state.rs`](crates/txtx-cli/src/cli/lsp/workspace/state.rs))

Thread-safe state management using `Arc<RwLock<WorkspaceState>>`:

```rust
pub struct WorkspaceState {
    /// All open documents
    documents: HashMap<Url, Document>,
    
    /// Parsed manifests (txtx.yml files)
    manifests: HashMap<Url, Manifest>,
    
    /// Runbook to manifest mapping
    runbook_to_manifest: HashMap<Url, Url>,
    
    /// Cached environment variables
    environment_vars: HashMap<String, HashMap<String, String>>,
}
```

### 4. Validation System

#### HCL Validator ([`diagnostics.rs`](crates/txtx-cli/src/cli/lsp/diagnostics.rs))

Uses `txtx_core::validation::hcl_validator` for real-time validation:

```rust
pub fn validate_runbook(file_uri: &Url, content: &str) -> Vec<Diagnostic> {
    let addons = addon_registry::get_all_addons();
    let addon_specs = addon_registry::extract_addon_specifications(&addons);
    
    match txtx_core::validation::hcl_validator::validate_with_hcl_and_addons(
        content,
        &mut validation_result,
        file_path,
        addon_specs,
    ) {
        Ok(_) | Err(_) => {
            // Convert validation errors to LSP diagnostics
        }
    }
}
```

#### Parser Integration

The LSP uses `hcl-edit` for parsing txtx files, leveraging its visitor pattern for efficient AST traversal. This provides:

- Accurate syntax understanding
- Position tracking for errors
- Support for HCL features like interpolations

### 5. Function Documentation ([`functions.rs`](crates/txtx-cli/src/cli/lsp/functions.rs))

Compile-time generated documentation for all addon functions:

```rust
pub fn get_function_hover_content(namespace: &str, function: &str) -> Option<String> {
    match (namespace, function) {
        ("evm", "encode_calldata") => Some(format_function_doc(
            "encode_calldata",
            "Encodes function call data for EVM contracts",
            vec![("function_name", "string"), ("args", "array")],
            "bytes",
            "evm::encode_calldata(\"transfer\", [recipient, amount])"
        )),
        // ... hundreds more functions
    }
}
```

## Request Flow

### Example: Go-to-Definition

1. **User clicks** on `input.api_key` in a `.tx` file
2. **VSCode sends** `textDocument/definition` request
3. **LSP routes** to `DefinitionHandler::handle()`
4. **Handler**:
   - Extracts variable name from position
   - Looks up manifest in workspace state
   - Finds variable definition location
   - Returns `Location` response
5. **VSCode jumps** to the definition

### Example: Validation Flow

1. **User saves** a `.tx` file
2. **VSCode sends** `textDocument/didSave` notification
3. **DocumentSyncHandler** receives notification in `did_save()`
4. **Handler calls** `crate::cli::lsp::diagnostics::validate_runbook()`
5. **Validator**:
   - Loads addon specifications via `addon_registry::get_all_addons()`
   - Runs HCL validation with `txtx_core::validation::hcl_validator::validate_with_hcl_and_addons()`
   - Converts validation errors to LSP diagnostics
6. **LSP publishes** diagnostics via `PublishDiagnosticsParams`
7. **VSCode shows** errors in Problems panel

## Performance Characteristics

### Synchronous Design

Following rust-analyzer's pattern, the LSP is synchronous:

- No async runtime overhead
- Direct request/response handling
- Predictable performance

### Efficient Parsing

- Uses `hcl-edit`'s visitor pattern
- Only parses changed documents
- Caches parsed manifests

### State Management

- `RwLock` allows concurrent reads
- Workspace state is lightweight
- No heavy computations in handlers

## Adding New Features

### Step 1: Create Handler

```rust
// In handlers/references.rs
pub struct ReferencesHandler {
    workspace: SharedWorkspaceState,
}

impl Handler for ReferencesHandler {
    fn workspace(&self) -> &SharedWorkspaceState {
        &self.workspace
    }
}

impl ReferencesHandler {
    pub fn find_references(&self, params: ReferenceParams) -> Vec<Location> {
        // Implementation
    }
}
```

### Step 2: Add to Handler Registry

```rust
// In handlers/mod.rs
pub struct Handlers {
    // ... existing handlers
    pub references: ReferencesHandler,
}
```

### Step 3: Route Requests

```rust
// In mod.rs
"textDocument/references" => {
    let params: ReferenceParams = serde_json::from_value(req.params)?;
    let locations = handlers.references.find_references(params);
    Response::new_ok(req.id, locations)
}
```

## Technical Decisions

### Why lsp-server over tower-lsp?

Based on [ADR-001](doc/adr/001-eliminate-lsp-server-crate.md):

| Aspect | tower-lsp | lsp-server |
|--------|-----------|------------|
| Maintenance | Unmaintained | Active (rust-analyzer) |
| Performance | Async overhead | Synchronous |
| Complexity | High abstraction | Direct control |
| Testing | Difficult | Straightforward |
| Proven | Limited use | Powers rust-analyzer |

### Why Integrated into CLI?

- Eliminates separate crate complexity
- LSP is txtx-specific, no reuse needed
- Simpler build and distribution
- Direct access to CLI infrastructure

### Parser Choice

Using `hcl-edit` instead of tree-sitter because:

- Already used by txtx-core
- Native Rust implementation
- Good error recovery
- Supports HCL features natively

## Integration Points

### 1. Doctor Command Integration

The LSP reuses doctor's validation infrastructure:

- Same addon loading mechanism
- Same validation rules
- Consistent error messages

**Current Implementation**:

- The `diagnostics.rs` module directly calls HCL validation via `txtx_core::validation::hcl_validator`
- Document sync handler triggers validation on save
- The `DiagnosticsHandler` in `handlers/diagnostics.rs` has TODOs for deeper doctor integration due to manifest type differences between LSP's simplified `Manifest` and doctor's `WorkspaceManifest`

### 2. Addon System

All addon specifications are loaded for validation:

```rust
let addons = addon_registry::get_all_addons();
let addon_specs = addon_registry::extract_addon_specifications(&addons);
```

### 3. Manifest System

The LSP understands txtx's manifest structure:

- Environment inheritance
- Multi-file runbooks
- Variable resolution order

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_variable_extraction() {
        let content = "value = input.api_key";
        let variable = extract_variable_at_position(content, 15);
        assert_eq!(variable, Some("api_key"));
    }
}
```

### Integration Tests

Located in various test files:

- Workspace state building
- Handler functionality
- Diagnostic conversion

### Manual Testing

1. Build the LSP: `cargo build --package txtx-cli`
2. Open VSCode with extension in debug mode
3. Test each feature with sample `.tx` files

## Current Implementation Status

### Working Features

- **Document Synchronization**: Full document lifecycle tracking (open, change, save, close)
- **HCL Validation**: Real-time syntax and semantic validation via `diagnostics.rs`
- **Addon Integration**: All addons are loaded and their specifications used for validation
- **Error Reporting**: Validation errors are converted to LSP diagnostics and shown in editor

### Implementation Details

The validation flow is split between two components:

1. **`handlers/document_sync.rs`**: Triggers validation on document save

   ```rust
   pub fn did_save(&self, params: DidSaveTextDocumentParams) -> Option<PublishDiagnosticsParams> {
       let diagnostics = if document.is_runbook() {
           crate::cli::lsp::diagnostics::validate_runbook(uri, document.content())
       } else {
           Vec::new()
       };
       // ...
   }
   ```

2. **`diagnostics.rs`**: Performs actual HCL validation

   ```rust
   pub fn validate_runbook(file_uri: &Url, content: &str) -> Vec<Diagnostic> {
       // Loads addons, runs HCL validation, converts to diagnostics
   }
   ```

### Known Limitations

1. **Diagnostics Handler TODOs**: The `handlers/diagnostics.rs` module has placeholder implementations that return empty diagnostics. The actual validation is performed by `diagnostics.rs` called from document sync.

2. **Manifest Type Mismatch**: LSP uses a simplified `Manifest` struct while doctor uses `WorkspaceManifest`, preventing full integration of doctor's validation rules.

3. **Single Workspace**: Currently limited to one workspace at a time.

4. **Limited Refactoring**: No support yet for rename refactoring, extract variable, or code actions.

## Future Enhancements

### Near Term

- [ ] Complete diagnostics handler integration
- [ ] Add references provider (find all references)
- [ ] Implement document symbols (outline view)
- [ ] Add code actions for quick fixes

### Long Term

- [ ] Multi-workspace support
- [ ] Semantic token provider (semantic highlighting)
- [ ] Rename refactoring
- [ ] Code lens (inline action buttons)
- [ ] Workspace symbols (project-wide search)

## Debugging

### Enable Verbose Logging

```rust
// Add throughout the code
eprintln!("[LSP] Processing request: {:?}", req.method);
```

### Test Protocol Directly

```bash
# Send initialization
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}' | txtx lsp

# Send completion request
echo '{"jsonrpc":"2.0","id":2,"method":"textDocument/completion","params":{...}}' | txtx lsp
```

### Common Issues

1. **Handler not called**: Check request routing in `mod.rs`
2. **Empty responses**: Verify workspace state is populated
3. **Performance issues**: Profile with `cargo flamegraph`

## Code References

- Entry point: [`crates/txtx-cli/src/cli/lsp/mod.rs`](crates/txtx-cli/src/cli/lsp/mod.rs)
- Handler system: [`crates/txtx-cli/src/cli/lsp/handlers/`](crates/txtx-cli/src/cli/lsp/handlers/)
- Workspace state: [`crates/txtx-cli/src/cli/lsp/workspace/state.rs`](crates/txtx-cli/src/cli/lsp/workspace/state.rs)
- HCL validation: [`crates/txtx-cli/src/cli/lsp/diagnostics.rs`](crates/txtx-cli/src/cli/lsp/diagnostics.rs)
- VSCode extension: [`vscode-extension/`](vscode-extension/)

## See Also

- [LSP_USER_GUIDE.md](LSP_USER_GUIDE.md) - User documentation
- [ARCHITECTURAL_REFACTORING.md](ARCHITECTURAL_REFACTORING.md) - Refactoring details
- [Language Server Protocol Specification](https://microsoft.github.io/language-server-protocol/)
