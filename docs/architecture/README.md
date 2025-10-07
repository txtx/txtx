# txtx Architecture Documentation

This directory contains architectural documentation for txtx components using a **hybrid approach** that combines hand-written documentation with code-generated artifacts.

## Contents

### Linter Architecture

**[linter/architecture.md](linter/architecture.md)** - Complete linter architecture

- Multi-layer validation pipeline (HCL → Manifest → Linter Rules)
- Multi-file runbook validation with file boundary mapping
- Complete validation flow from CLI to output
- Module structure and performance characteristics
- Detailed Mermaid diagrams

**[linter/workspace.dsl](linter/workspace.dsl)** - Structurizr C4 model

- System context and container diagrams
- Dynamic diagrams for validation flows (single-file, multi-file, flow validation)
- Component relationships and interactions

### LSP Architecture

**[lsp/async-implementation.md](lsp/async-implementation.md)** - Async LSP architecture

- Tokio-based async request handling
- ~50% latency improvements from async design
- Document caching with TTL and LRU eviction
- Concurrent request processing
- Performance benchmarks and workspace state machine

**[lsp/sequences.md](lsp/sequences.md)** - LSP protocol sequences

- Detailed request/response flows
- Protocol interactions with IDE

**[lsp/state-management.md](lsp/state-management.md)** - State management architecture

- Workspace state machine
- Document lifecycle management

**[lsp/use-cases.md](lsp/use-cases.md)** - LSP use cases

- User interactions and scenarios

**[lsp/workspace.dsl](lsp/workspace.dsl)** - Structurizr C4 model

- System context showing IDE integration
- Container and component diagrams
- Dynamic diagrams for LSP request flows (didOpen, didChange, completion)

### Cross-Cutting Documentation

**[features.md](features.md)** - Linter and LSP feature behavior

- Feature scoping and interaction
- Validation rule behavior

**[performance-improvements.md](performance-improvements.md)** - Historical performance report

- August 2024 async refactoring achievements
- Benchmarks and metrics

---

## Documentation Strategy

### Hybrid Approach

We combine two documentation methods:

1. **Hand-Written Documentation** - Markdown files and Structurizr DSL for architecture, flows, and design decisions
2. **Auto-Generated Documentation** - Component definitions extracted from code annotations

### Hand-Written Documentation

**Files**: `workspace.dsl`, `architecture.md`, `async-implementation.md`

**Best for**:
- Dynamic behavior (sequences, flows, state machines)
- User interactions
- System context
- Architectural decisions not reflected in code structure
- Performance characteristics and design rationale

**Benefits**:
- Rich context and narrative
- Shows runtime behavior and protocol flows
- Documents intent, not just structure
- Stable, reviewed, and versioned

### Auto-Generated Documentation

**Files**: `workspace-generated.dsl` (created by `just arch-c4`)

**Best for**:
- Component inventory
- Component descriptions from code
- Responsibilities from code annotations
- Keeping docs synchronized with code changes

**Benefits**:
- Single source of truth (code is the documentation)
- Always up-to-date with codebase
- No manual synchronization burden
- Enforces documentation discipline in code

---

## Working with Architecture Docs

### Viewing Structurizr Diagrams

**Interactive visualization** (recommended):

```bash
just arch-view
```

Opens <http://localhost:8080> with:
- System context diagram
- Container diagram
- Component diagrams per container
- Dynamic diagrams showing validation and LSP flows

**Manual setup** with Podman (macOS):

```bash
cd docs/architecture/linter  # or docs/architecture/lsp
podman pull docker.io/structurizr/lite
podman run -it --rm -p 8080:8080 \
  -v $(pwd):/usr/local/structurizr:Z \
  docker.io/structurizr/lite
```

**Manual setup** with Docker:

```bash
cd docs/architecture/linter  # or docs/architecture/lsp
docker pull structurizr/lite
docker run -it --rm -p 8080:8080 \
  -v $(pwd):/usr/local/structurizr \
  structurizr/lite
```

**Export to other formats**:

```bash
# Install Structurizr CLI
brew install structurizr-cli

# Export to PlantUML
structurizr-cli export -workspace workspace.dsl -format plantuml

# Export to Mermaid
structurizr-cli export -workspace workspace.dsl -format mermaid
```

**Online viewer**:

Upload `workspace.dsl` to <https://structurizr.com/dsl>

### Viewing Markdown Documentation

**Mermaid diagrams** render automatically on GitHub. Just browse to:
- `architecture.md` (linter)
- `async-implementation.md` (LSP)

---

## Generating Diagrams from Code

### C4 Annotations

The codebase includes C4 architecture annotations as doc comments:

```rust
//! # C4 Architecture Annotations
//! @c4-component ValidationContext
//! @c4-container Validation Core
//! @c4-description Central state management for all validation operations
//! @c4-technology Rust
//! @c4-relationship "Delegates to" "HCL Validator"
//! @c4-uses FileBoundaryMapper "Maps multi-file errors"
//! @c4-responsibility Manage validation state across all validation layers
//! @c4-responsibility Compute effective inputs from manifest + environment + CLI
```

### Generating Component Diagrams

**Regenerate `workspace-generated.dsl` from code annotations**:

```bash
just arch-c4
```

This builds and runs the `c4-generator` Rust utility (located in `crates/c4-generator/`), which scans the codebase for `@c4-*` annotations and generates component definitions.

**Benefits**:
- Architecture documentation lives in the code
- Auto-sync diagrams with code changes
- Single source of truth for component descriptions

---

## When to Update Documentation

### Update Hand-Written Docs When:

- Adding new validation flows or LSP capabilities
- Changing user interactions or protocol handling
- Modifying the validation pipeline or async architecture
- Adding/removing containers or major components
- Making architectural decisions (document in ADRs)

### Regenerate Auto-Generated Docs When:

Run `just arch-c4` when:
- Adding/removing components
- Changing component descriptions
- Updating responsibilities
- Modifying component relationships

**Best practice**: Regenerate before submitting PRs to keep diagrams in sync.

---

## Best Practices

1. **Annotate as you code** - Add `@c4-*` annotations when creating new components
2. **Regenerate before PRs** - Run `just arch-c4` to sync generated docs
3. **Update hand-written for flows** - When changing validation sequences or LSP protocol handling, update `workspace.dsl`
4. **Keep responsibilities concise** - Each `@c4-responsibility` should be one clear statement
5. **Review generated output** - Check `workspace-generated.dsl` after major refactorings
6. **Use Mermaid for GitHub** - For simple diagrams, use Mermaid in Markdown (renders on GitHub)
7. **Use Structurizr for complexity** - For complex systems with multiple views, use Structurizr DSL

---

## Other Diagram Tools

### Rust Module Graphs

```bash
# Module dependency graph
cargo install cargo-modules
cargo modules generate graph --with-types | dot -Tpng > modules.png

# Dependency tree
cargo install cargo-deps
cargo deps | dot -Tpng > deps.png
```

---

## Architecture Decision Records

See [../adr/](../adr/) for architectural decisions with full context and rationale:

- [ADR 001: Parallel Runbook Validation](../adr/001-pr-architectural-premise.md)
- [ADR 002: Eliminate LSP Server Crate](../adr/002-eliminate-lsp-server-crate.md)
- [ADR 003: Capture Everything Pattern](../adr/003-capture-everything-filter-later-pattern.md)
- [ADR 004: Visitor Strategy Pattern](../adr/004-visitor-strategy-pattern-with-readonly-iterators.md)

---

## Structurizr Benefits

**Why use Structurizr?**

- **Single source of truth**: All diagrams generated from one DSL file
- **Multiple views**: Context, Container, Component, Dynamic views from same model
- **Auto-layout**: Diagrams auto-arrange (can be manually tweaked)
- **Export formats**: PlantUML, Mermaid, DOT, WebSequenceDiagrams
- **Version control friendly**: Text-based DSL diffs cleanly
- **Interactive**: Click through components in browser

**When to use Structurizr vs Mermaid:**

- **Structurizr**: Complex systems with multiple perspectives and dynamic flows
- **Mermaid**: Quick diagrams, GitHub rendering, simple flows, inline documentation
