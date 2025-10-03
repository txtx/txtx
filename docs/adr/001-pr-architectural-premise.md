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

## Future Work

Once workspace_context.rs has test coverage:

1. Extract common visitor utilities
2. Share span/position calculations
3. Unify block type definitions

But critically: **We don't wait for perfect to ship good**.

## Result

This architecture allows us to:

- Ship linting and LSP features immediately
- Add zero risk to production systems
- Maintain ability to disable if needed
- Set up a path for future consolidation

The duplication is not technical debt - it's technical insurance.
