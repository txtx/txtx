# LSP Module

The Language Server Protocol (LSP) implementation for txtx, providing IDE features for runbook development.

## Architecture (Refactored)

The module now follows a handler-based architecture:

```
lsp/
├── handlers/           # Request handlers
│   ├── completion.rs  # Code completion
│   ├── definition.rs  # Go to definition
│   ├── diagnostics.rs # Real-time validation
│   ├── document_sync.rs # Document synchronization
│   └── hover.rs       # Hover information
├── validation/        # Doctor integration
│   ├── adapter.rs     # Adapts doctor rules for LSP
│   └── converter.rs   # Converts validation outcomes
├── workspace/         # State management
│   ├── documents.rs   # Document tracking
│   ├── manifests.rs   # Manifest parsing
│   └── state.rs       # Workspace state
├── utils.rs          # Helper functions
└── mod.rs           # Request routing
```

## Key Components

### Handler Trait
All request handlers implement this trait:

```rust
pub trait Handler: Send + Sync {
    fn method(&self) -> &'static str;
    fn handle(&self, params: serde_json::Value) -> Result<serde_json::Value, ResponseError>;
}
```

### Built-in Handlers
- **CompletionHandler**: Provides context-aware completions
- **DefinitionHandler**: Navigate to action/input definitions
- **DiagnosticsHandler**: Real-time validation (doctor integration pending)
- **DocumentSyncHandler**: Tracks document changes
- **HoverHandler**: Shows documentation on hover

### Workspace Management
- Thread-safe state management with `Arc<RwLock<WorkspaceState>>`
- Document versioning and change tracking
- Manifest parsing and caching
- Environment variable resolution

## Features

### Implemented
- ✅ Code completion for actions, inputs, and signers
- ✅ Go to definition for action references
- ✅ Hover documentation for actions
- ✅ Document synchronization
- ✅ Workspace symbol search
- ✅ HCL-integrated diagnostics (per ADR-002)
- ✅ Real-time validation with doctor rules

### Pending
- ⏳ Code actions (quick fixes)
- ⏳ Rename refactoring
- ⏳ Formatting
- ⏳ Enhanced HCL error position extraction

## Usage

The LSP server is started with:

```bash
txtx lsp
```

Configure your editor to connect to the txtx language server:

### VS Code
Install the txtx extension (when available)

### Neovim
```lua
require'lspconfig'.txtx.setup{
  cmd = {'txtx', 'lsp'},
  filetypes = {'txtx'},
  root_dir = require'lspconfig.util'.root_pattern('txtx.yml', '.git'),
}
```

## Extending

### Adding a New Handler

1. Create a new handler file in `handlers/`:
```rust
pub struct MyHandler;

impl Handler for MyHandler {
    fn method(&self) -> &'static str {
        "textDocument/myFeature"
    }
    
    fn handle(&self, params: serde_json::Value) -> Result<serde_json::Value, ResponseError> {
        // Implementation
    }
}
```

2. Register in `mod.rs`:
```rust
router.register(Box::new(MyHandler));
```

### Validation Architecture (ADR-002)

The LSP now integrates HCL parser diagnostics directly:

1. **HCL Syntax Validation**: 
   - `diagnostics_hcl_integrated.rs` parses HCL and extracts syntax errors
   - Error positions are extracted from HCL error messages
   - Provides immediate feedback for syntax issues

2. **Semantic Validation**:
   - Uses existing `hcl_validator` for semantic checks
   - Validates action types, signer references, undefined fields
   - Multi-file support through `diagnostics_multi_file.rs`

3. **Doctor Integration**:
   - `DoctorValidationAdapter` wraps doctor rules for LSP use
   - `validation_outcome_to_diagnostic` converts doctor outcomes to LSP diagnostics
   - Provides additional project-specific validation rules

## Testing

- Unit tests for individual handlers
- Integration tests for end-to-end LSP flows
- Mock workspace for testing state management

## Future Improvements

1. **Complete Doctor Integration**: Resolve type mismatch between LSP and core manifest types
2. **Incremental Parsing**: Parse only changed portions of documents
3. **Caching**: Cache parsed ASTs and validation results
4. **Multi-root Workspaces**: Support multiple txtx projects
5. **Custom Commands**: Expose txtx-specific commands through LSP