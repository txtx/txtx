# ADR-004: Visitor Strategy Pattern with Read-Only Iterators for HCL Validation

## Status

Accepted

## Date

2025-09-27

## Context

The HCL validator in txtx needed significant refactoring to address several issues:

1. **DRY Violations**: Block processing logic was duplicated across multiple methods (~50-100 lines each)
2. **Tight Coupling**: The `HclValidationVisitor` directly handled all block types, making it difficult to extend
3. **State Management Complexity**: Mutable state was shared between the visitor and processing logic, creating complex borrowing scenarios
4. **Circular Dependency Bug**: The circular dependency detection was failing due to timing issues in when block names were set

### Original Implementation Problems

The original implementation had several interconnected issues:

```rust
// Old approach - direct mutation and tight coupling
impl HclValidationVisitor {
    fn process_variable_block(&mut self, block: &Block) {
        // 50+ lines of duplicated logic
        self.defined_variables.insert(name);
        self.current_block.name = name;
        // More direct mutations...
    }

    fn process_action_block(&mut self, block: &Block) {
        // Another 100+ lines of similar logic
        // Direct mutations of visitor state
    }
    // ... repeated for each block type
}
```

### Requirements

- Eliminate code duplication in block processing
- Enable easy addition of new block types
- Maintain clear ownership and borrowing patterns
- Fix circular dependency detection
- Preserve all existing functionality and tests

## Decision

Implement a **Strategy Pattern with Read-Only Iterators** where:

1. Block processors only receive read-only references to visitor state
2. Processors return results instead of mutating state
3. The visitor maintains ownership and applies results
4. Block names are extracted early to enable proper dependency tracking

### Architecture

```rust
// Result type returned by processors
pub struct ProcessingResult {
    pub variables: Vec<String>,
    pub signers: Vec<(String, String)>,
    pub outputs: Vec<String>,
    pub actions: Vec<(String, String, Option<CommandSpecification>)>,
    pub flows: Vec<(String, Vec<String>, (usize, usize))>,
    pub errors: Vec<ValidationError>,
    pub blocks_with_errors: Vec<String>,
    pub current_block_name: Option<String>,
}

// Processing context with read-only access
pub struct ProcessingContext<'a> {
    // Read-only references to visitor's state
    pub defined_variables: &'a HashSet<String>,
    pub defined_signers: &'a HashMap<String, String>,
    pub addon_specs: &'a HashMap<String, Vec<(String, CommandSpecification)>>,
    // ... other read-only fields

    // Error reporting utilities
    pub file_path: &'a str,
    pub source: &'a str,
}

// Strategy trait for block processors
pub trait BlockProcessor {
    fn process_collection(&mut self, block: &Block, context: &ProcessingContext) -> ProcessingResult;
    fn process_validation(&mut self, block: &Block, context: &ProcessingContext) -> ProcessingResult;
}
```

## Consequences

### Positive

1. **Clear Ownership**: The visitor maintains exclusive ownership of all state, eliminating complex borrowing patterns

2. **Functional Style**: Processors are pure functions (conceptually) that take input and return results, making them easier to test and reason about

3. **Extensibility**: Adding new block types only requires implementing the `BlockProcessor` trait

4. **No Shared Mutable State**: Eliminates entire classes of bugs related to concurrent mutation

5. **Performance**: No unnecessary cloning - only read-only references are passed around

6. **Maintainability**: Each processor is self-contained with clear inputs and outputs

7. **Bug Fix**: Circular dependency detection now works correctly because block names are set before processing

### Negative

1. **Slightly More Verbose**: Must explicitly return results and apply them, rather than direct mutation

2. **Two-Step Process**: Process then apply, rather than direct mutation (though this improves clarity)

## Implementation Details

### Key Changes

1. **ProcessingContext**: Changed from having mutable write channels to only read-only references
2. **BlockProcessor trait**: Methods now return `ProcessingResult` instead of mutating context
3. **Visitor**: Applies results after processing, maintaining clear ownership
4. **Block name extraction**: Done immediately when visiting blocks, not deferred

### Example Processor

```rust
impl BlockProcessor for VariableProcessor {
    fn process_collection(&mut self, block: &Block, _context: &ProcessingContext) -> ProcessingResult {
        let mut result = ProcessingResult::new();

        if let Some(BlockLabel::String(name)) = block.labels.get(0) {
            let var_name = name.value().to_string();
            result.current_block_name = Some(var_name.clone());
            result.variables.push(var_name);
        }

        result
    }
}
```

## Alternatives Considered

1. **Mutable Write Channels**: Pass mutable vectors as "channels" for results
   - Rejected: Creates complex borrowing scenarios

2. **Clone Everything**: Clone all state for each processor
   - Rejected: Unnecessary performance overhead

3. **Visitor Trait Methods**: Keep all logic in the visitor
   - Rejected: Doesn't solve the duplication problem

## References

- [Strategy Pattern](https://refactoring.guru/design-patterns/strategy)
- [Visitor Pattern](https://refactoring.guru/design-patterns/visitor)
- Rust Ownership and Borrowing best practices