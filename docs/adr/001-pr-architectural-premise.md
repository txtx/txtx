# Architecture Decision: Parallel Validation Without Modifying Critical Paths

## Status

Accepted

## Date

2025-09-01

## Context

The txtx codebase has a critical execution path in `workspace_context.rs` that:

- Parses HCL runbooks and builds the execution graph
- Creates command instances and manages state
- Is complex (~900 lines) and lacks test coverage
- If broken, would break all txtx runbook execution in production

## Decision

Build validation as a **parallel, read-only system** that traverses the same AST but never modifies execution paths.

## Rationale

### Why Not Refactor workspace_context.rs?

1. **Risk**: Any bug introduced would break production runbooks
2. **No Tests**: Cannot safely refactor without test coverage
3. **Complexity**: The file handles imports, modules, actions, signers, flows - all interdependent
4. **Time**: Adding tests first would delay shipping user value

### Why Parallel Validation is Safe

```rust
// workspace_context.rs - EXISTING, UNTESTED, CRITICAL
match block.ident.value().as_str() {
    "action" => {
        runtime_context.create_action_instance(...) // Modifies state
        self.index_construct(...)                   // Builds graph
    }
}

// hcl_validator.rs - NEW, ISOLATED, SAFE
match block.ident.value().as_str() {
    "action" => {
        self.process_action_block(block)  // Read-only validation
        // Cannot affect runtime execution
    }
}
```

## Benefits of This Approach

1. **Zero Production Risk**
   - Validation can have bugs without breaking execution
   - Can be disabled instantly if issues arise
   - No changes to critical untested code

2. **Ship Features Faster**
   - Don't need to add tests to workspace_context first
   - Can iterate on validation independently
   - Users get value immediately

3. **Future Refactoring Path**
   - Once workspace_context has tests, can extract common code
   - But not blocked on that work
   - Technical debt is isolated and manageable

## Trade-offs

### Deliberate Code Duplication

Yes, both files have `span_to_position()` and similar block matching. This is intentional:

- **Shared code = shared risk**: A bug in shared utilities affects both paths
- **Duplication = isolation**: Each system can evolve independently
- **Future consolidation**: Can extract common patterns once tests exist

### Maintenance Cost

- Two places to update when adding new block types
- But: New block types are rare
- And: The safety benefit outweighs the maintenance cost

## Validation Principles

1. **Read-Only**: Never modify state that affects execution
2. **Fail-Safe**: Validation errors never stop execution
3. **Isolated**: Can be disabled without touching runtime
4. **Parallel**: Both systems traverse the same AST independently

## Evolution: Common Definitions Layer

**Date**: 2025-10-08 (Suggested by Micaiah, refactored collaboratively)

The original parallel validation premise still holds, but the architecture evolved to eliminate type duplication across the codebase through **common type definitions**.

### The Problem

The validation infrastructure introduced types that duplicated existing runtime types:

- Runtime parser used `Diagnostic`, `FileLocation`, etc. from `txtx-addon-kit`
- Validation introduced `ValidationError`, `ValidationWarning`, `LocatedInputRef`, etc.
- Similar types with different names serving overlapping purposes
- Duplication between runtime parser, linter, and LSP
- Changes to type definitions required updates in multiple places

### The Solution: Unified Type Definitions

Micaiah identified the duplication and suggested unifying the types. The `validator-merge` refactor created common type definitions that eliminate duplication:

```rust
// Before: Duplicated types
// txtx-addon-kit/types/diagnostics.rs
pub struct Diagnostic { ... }

// txtx-core/validation/types.rs
pub struct ValidationError { ... }  // Duplicate!

// After: Unified common types
// crates/txtx-core/validation/types.rs (shared foundation)
pub struct Diagnostic { ... }           // Unified diagnostic type
pub struct ValidationContext { ... }    // Shared context
pub struct ValidationResult { ... }     // Common result type
pub struct LocatedInputRef { ... }      // Common reference type

// Runtime parser - uses unified types
use txtx_core::validation::types::{Diagnostic, LocatedInputRef};

impl WorkspaceContext {
    fn parse(&self) -> Result<(), Diagnostic> {
        // Runtime using unified types
    }
}

// Linter - uses same unified types
use txtx_core::validation::{ValidationContext, ValidationResult};

impl LintRule {
    fn validate(&self, ctx: &ValidationContext) -> ValidationResult {
        // Linter using unified types
    }
}

// LSP - uses same unified types
use txtx_core::validation::{ValidationContext, Diagnostic};

impl LspHandler {
    fn validate_document(&self, ctx: &ValidationContext) -> Vec<Diagnostic> {
        // LSP using unified types
    }
}
```

### Leveraging Rust's Type System

**Key insight**: Unified types eliminate duplication and prevent drift through compile-time enforcement.

- Change a `Diagnostic` field → Compiler errors in runtime, linter, AND LSP
- Add a new error variant → All three systems must handle it
- Modify a type definition → Type checker ensures consistent usage everywhere

**This eliminates duplication and makes drift impossible** - you have one type definition, and the compiler ensures it's used consistently everywhere.

### Benefits

1. **Eliminated Duplication**
   - Single source of truth for error types, diagnostics, and contexts
   - No more duplicated type definitions with similar purposes
   - Reduced cognitive overhead - one type name for one concept

2. **Type-Safe Synchronization**
   - Compiler enforces consistency across runtime, linter, and LSP
   - "Make illegal states unrepresentable" - divergence won't compile
   - Change once, compiler tells you everywhere that needs updating

3. **Maintains Original Safety**
   - Linter and LSP validation is still parallel and read-only
   - Still zero production risk
   - Can still be disabled independently

4. **Reduced Maintenance**
   - Update type definitions once in common module
   - Compiler identifies all locations requiring updates
   - No manual synchronization across modules

### Architecture Layers

```
┌──────────────────────────────────────────────────────────┐
│  exp/2/linter    │    exp/3/lsp    │  workspace_context │  ← Consumer layer
│  (CLI validation)│    (IDE)        │  (Runtime)         │
└──────────────┬───────────┬──────────────────┬────────────┘
               │           │                  │
               ├───────────┼──────────────────┤
               ▼           ▼                  ▼
                    ┌──────────────────────┐
                    │  exp/1/validator     │              ← Common definitions
                    │  (Shared validation  │
                    │   types & traits)    │
                    └──────────────────────┘
```

The common definitions layer acts as a typed contract that runtime, linter, and LSP all implement against.

### Trade-off Resolution

This evolution improves upon the original "deliberate duplication" strategy:

- **Original approach**: Duplicate code to isolate validation from runtime
- **Problem discovered**: Duplication extended to type definitions (Diagnostic, Error types, etc.)
- **Resolution**: Unify type definitions while keeping validation logic separate
- **Result**: Eliminated type duplication with DRY definitions, while maintaining isolated validation logic

The runtime, linter, and LSP now share common type definitions, but the validation _logic_ remains parallel and isolated from the execution path - preserving the original safety guarantee while eliminating unnecessary type duplication.

## Future Work

Once workspace_context.rs has test coverage:

1. Extract common visitor utilities
2. Share span/position calculations
3. Unify block type definitions

But critically: **We don't wait for perfect to ship good**.

## Result

This architecture (and its evolution) allows us to:

- Ship linting and LSP features immediately
- Add zero risk to production systems
- Maintain ability to disable if needed
- Eliminate type duplication while keeping validation logic isolated
- Keep runtime, linter, and LSP in sync through type-safe common definitions
- Leverage Rust's compiler to prevent type drift across all systems

**Original insight**: The duplication is not technical debt - it's technical insurance.

**Evolution insight**: We can unify type definitions to eliminate duplication while keeping validation logic separate - preserving safety while being DRY where it matters.
