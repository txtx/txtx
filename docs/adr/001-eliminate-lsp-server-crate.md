# ADR-001: Eliminate txtx-lsp-server Crate

## Status

Accepted

## Date

2025-09-1

## Context

After migrating from `tower-lsp` to `lsp-server` (following rust-analyzer's architecture), we have a separate `txtx-lsp-server` crate that contains the LSP backend implementation. This crate structure was inherited from the original tower-lsp design, where the async runtime and complex trait system necessitated separation.

### Current Architecture

```console
txtx-cli
├── src/cli/lsp.rs (message loop)
└── depends on → txtx-lsp-server
                  ├── backend_sync.rs (492 lines - ACTIVE)
                  ├── backend.rs (26KB - UNUSED, old tower-lsp)
                  ├── document.rs (11KB - UNUSED)
                  ├── symbols.rs (14KB - UNUSED)
                  └── lib.rs (only exports TxtxLspBackend)
```

### Problems with Current Structure

1. **Unnecessary Indirection**: The separate crate adds complexity without benefits
2. **Dead Code**: 70% of the crate (51KB out of 70KB) is unused legacy code
3. **Maintenance Overhead**: Extra crate to version, build, and maintain
4. **Confusing Architecture**: Developers must understand why LSP is split across crates
5. **No Reusability**: The LSP backend is txtx-specific and won't be reused elsewhere

## Decision

Eliminate the `txtx-lsp-server` crate entirely by:

1. Moving `backend_sync.rs` directly into `txtx-cli/src/cli/lsp/backend.rs`
2. Deleting the entire `txtx-lsp-server` crate
3. Removing the dependency from `txtx-cli/Cargo.toml`

### New Architecture

```console
txtx-cli
└── src/cli/lsp/
    ├── mod.rs (message loop, routes requests)
    └── backend.rs (LSP implementation, ~500 lines)
```

## Consequences

### Positive

- **Simpler Architecture**: One less crate to understand and maintain
- **Faster Compilation**: Fewer crate boundaries means better optimization
- **Cleaner Dependencies**: Removes unused dependencies from the project
- **Direct Integration**: LSP is clearly part of the CLI, not a separate library
- **Less Dead Code**: Removes 51KB of unused legacy implementation
- **Easier Navigation**: Developers can find all LSP code in one place

### Negative

- **Larger CLI Module**: The CLI crate grows by ~500 lines (acceptable)
- **No Separate Testing**: Can't test LSP backend in isolation (but we test at protocol level anyway)
- **Less Modularity**: Can't publish LSP as a separate crate (not needed)

### Neutral

- **Git History**: History is preserved through git, though file moves
- **Breaking Change**: Internal architecture change, no external API impact

## Alternatives Considered

### 1. Keep Separate Crate but Clean It Up

- **Pros**: Maintains separation of concerns
- **Cons**: Still has unnecessary indirection for no benefit
- **Rejected**: The separation provides no value since LSP is txtx-specific

### 2. Create a Workspace-Level LSP Crate

- **Pros**: Could potentially share with other tools
- **Cons**: No other tools need this LSP implementation
- **Rejected**: Over-engineering for a hypothetical future need

### 3. Move to txtx-core

- **Pros**: Central location for core functionality
- **Cons**: LSP is CLI-specific, not core logic
- **Rejected**: Would pollute core with CLI concerns

## Implementation Plan

1. ✅ Create this ADR documenting the decision
2. Move `backend_sync.rs` → `txtx-cli/src/cli/lsp/backend.rs`
3. Update imports in `txtx-cli/src/cli/lsp.rs`
4. Remove `txtx-lsp-server` from `txtx-cli/Cargo.toml`
5. Delete `crates/txtx-lsp-server/` directory
6. Update workspace `Cargo.toml` to remove the crate
7. Run tests to ensure everything still works
8. Update documentation (LSP.md) to reflect new structure

## Notes

This decision aligns with our broader architectural principle of "simplicity over modularity when modularity provides no clear benefit." The LSP backend is inherently tied to the txtx CLI and treating it as a separate library added complexity without value.

The migration from tower-lsp to lsp-server already eliminated the technical reasons for separation (async runtime, complex traits). This change completes that simplification by eliminating the organizational separation as well.

## References

- Original tower-lsp architecture required separation due to async traits
- rust-analyzer keeps LSP in the main binary, not a separate crate
- YAGNI principle: "You Aren't Gonna Need It" - don't add modularity until needed
