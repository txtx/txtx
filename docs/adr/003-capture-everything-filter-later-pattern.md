# ADR-0001: Capture Everything, Filter Later Pattern for Runbook Analysis

## Status

Accepted

## Date

2025-09-15

## Context

The txtx lint command needed to evolve from a simple validator into a configurable linter following ESLint/Clippy paradigms. Initially, we considered creating multiple specialized iterators for different runbook elements (variables, actions, signers, etc.), following the existing pattern established by `RunbookVariableIterator`.

### Initial Approach Considered

- Create specialized iterators for each runbook element type
- Each iterator would traverse the HCL AST independently
- Each lint rule would potentially trigger its own traversal
- Estimated 5+ iterators needed (variables, actions, signers, attributes, blocks)

### Problems Identified

1. **Code duplication**: Each iterator would need ~300 lines of similar traversal logic
2. **Performance**: Multiple AST traversals (O(n×r) where n=nodes, r=rules)
3. **Maintenance burden**: Adding new element types requires new iterators
4. **Complexity**: Rules need to understand visitor patterns and AST traversal

## Decision

Implement a single `RunbookCollector` that traverses the AST once, collecting all runbook items into a unified data structure, which rules can then filter and process as needed.

### Implementation

```rust
pub enum RunbookItem {
    InputReference { name, full_path, location, raw },
    VariableDef { name, location, raw },
    ActionDef { name, action_type, namespace, action_name, location, raw },
    SignerDef { name, signer_type, location, raw },
    // ... other variants
}

pub struct RunbookCollector {
    items: Vec<RunbookItem>,
    source: Arc<String>,  // Shared source for memory efficiency
}

pub struct RunbookItems {
    // Provides filtered views via iterator methods
    pub fn input_references(&self) -> impl Iterator<Item = (&str, &Location)>
    pub fn actions(&self) -> impl Iterator<Item = (&str, &str, &Location)>
    // ... other filtering methods
}
```

## Consequences

### Positive

1. **55% code reduction** (692 lines vs estimated 1,552 lines)
   - Single 447-line collector replaces 5+ iterators
   - Rules reduced from 100-150 lines to 20-30 lines each

2. **Performance improvement**
   - Single AST traversal: O(n) instead of O(n×r)
   - Shared memory via Arc for source text
   - Lazy filtering via iterator chains

3. **Simplified rule implementation**

   ```rust
   // Before: Complex visitor pattern
   impl LintRule for UndefinedInputRule {
       fn check(&self, context: &LintContext) -> Vec<Violation> {
           // 50-100 lines of traversal logic
       }
   }

   // After: Simple filtering
   for (input_name, location) in items.input_references() {
       if !environment_vars.contains_key(input_name) {
           violations.push(/*...*/);
       }
   }
   ```

4. **Extensibility**
   - Adding new item types: ~20 lines (enum variant + collection logic)
   - Adding new rules: ~20 lines (match arm using existing data)
   - Previously: 300+ lines for new iterator, 100+ for new rule

5. **Composability**

   ```rust
   items.input_references()
       .filter(|(name, _)| name.starts_with("AWS_"))
       .map(|(name, loc)| check_naming(name, loc))
   ```

### Negative

1. **Memory usage**: Stores all items in memory at once
   - Mitigated by Arc sharing and selective field storage
   - Not an issue for typical runbook sizes

2. **Less specialized**: Generic collection vs purpose-built iterators
   - Mitigated by providing specialized filtering methods
   - Raw AST nodes preserved for unforeseen use cases

3. **Upfront collection cost**: Must collect everything even if only need subset
   - Negligible for single-pass traversal
   - Offset by avoiding multiple traversals

### Neutral

- **Learning curve**: Developers need to understand the collection model
- **Testing**: Requires different testing strategy (test collector + filters separately)

## Metrics

| Metric | Specialized Iterators | Capture Everything | Improvement |
|--------|----------------------|-------------------|-------------|
| Total ELOC | ~1,552 | 692 | 55% reduction |
| Lines per rule | 100-150 | 20-30 | 80% reduction |
| AST traversals | Multiple | Single | O(n×r) → O(n) |
| Add new item type | ~300 lines | ~20 lines | 93% reduction |
| Add new rule | ~100 lines | ~20 lines | 80% reduction |

## Alternatives Considered

### 1. Multiple Specialized Iterators

- **Pros**: Type-safe, specialized APIs, follows existing pattern
- **Cons**: Code duplication, multiple traversals, high maintenance
- **Rejected because**: Excessive code duplication and performance overhead

### 2. Visitor Pattern with Callbacks

- **Pros**: Flexible, follows HCL library pattern
- **Cons**: Complex callbacks, difficult composition, verbose rules
- **Rejected because**: Too complex for rule authors

### 3. Lazy Streaming Iterator

- **Pros**: Memory efficient, composable
- **Cons**: Complex lifetime management, can't look ahead/behind
- **Rejected because**: Complexity outweighs benefits for typical runbook sizes

## References

- Original discussion: User suggestion "what if we had 1 iterator that captures EVERYTHING"
- User guidance: "i prefer if we kept the abstraction simple and focused"
- Implementation: `crates/txtx-core/src/runbook/collector.rs`
- Usage: `crates/txtx-cli/src/cli/linter_impl/linter/engine_v2.rs`

## Notes

This pattern could be applied to other areas of the codebase where multiple passes over the AST are currently performed. The success of this approach validates the principle of "parse once, query many" for AST-based tools.
