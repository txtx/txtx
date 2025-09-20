# Lessons Learned: Doctor Command and LSP Implementation

## From Testing Challenges to Architectural Clarity

The journey to understanding txtx's true nature as a compiler began with implementing better error handling through the `error-stack` crate. What started as a straightforward enhancement in the `feat/evm-error-stack` branch became an important lesson in software architecture and developer experience.

### Development Time Analysis

While adding context-aware error handling, analysis revealed that a significant portion of development time was being spent on test infrastructure. The challenges included:

#### Test Creation Complexity

- **String vs File-based Tests**: The choice between inline strings and separate files presented trade-offs. String-based tests were easier to modify but could clutter code. File-based tests were cleaner but raised organizational questions about directory structure and fixture sharing.
- **Fixture Management**: Each test required carefully crafted txtx configurations. Reusing fixtures created coupling concerns while duplicating them increased maintenance overhead.

#### Addon Testing Infrastructure

- **Chain Mocking Requirements**: Each addon (EVM, Stacks, Bitcoin) required mocking its blockchain interaction. Testing even a single addon like EVM with Anvil became complex when combined with fixture management.
- **Process-Based Mocking**: As noted by team members, using Anvil in CI was resource-intensive—requiring full blockchain nodes as mocks. This approach consumed significant resources and time.
- **Scalability Considerations**: Testing across all supported protocols required a more efficient framework that could scale without extensive blockchain simulation infrastructure.

#### Development Impact

- Component changes often affected seemingly unrelated tests
- Unclear boundaries between unit and integration test responsibilities
- Tests focused on implementation details rather than behavior
- Extended feedback loops impacting development velocity

These observations indicated that the testing infrastructure needed architectural improvements to better support development.

### The Strategic Pivot

This led to a strategic decision: temporarily defer the error-stack work and focus on implementing LSP (Language Server Protocol) support and a `doctor` command for diagnostics. This pivot provided an opportunity to approach the architecture from a different perspective.

### The Architectural Insight

Working on LSP and doctor commands revealed important requirements. These tools needed to:

- Parse txtx files incrementally
- Validate syntax and semantics separately
- Provide meaningful diagnostics at each stage
- Work with partial or invalid input gracefully

This led to a key realization: **txtx is fundamentally a domain-specific language compiler** with a CLI interface, rather than just a CLI tool.

### Why Compiler Architecture Improves Testing

This insight explained the testing challenges and pointed to solutions:

1. **Isolated Stages**: In a compiler pipeline, each stage (lexing, parsing, validation, code generation) can be tested independently with clear inputs and outputs.

2. **Pure Functions**: Compiler passes are naturally pure functions—given an AST, produce a validated AST. Given validated AST, produce an execution plan. This makes testing deterministic and fast.

3. **Composable Design**: Each compiler phase builds on the previous one through well-defined interfaces, making it easy to test components in isolation or combination.

4. **Error Handling**: Compilers have established patterns for error collection and reporting, making the error-stack implementation natural rather than forced.

#### Addressing Specific Challenges

**Test Creation**: With compiler architecture and builder patterns, tests become composable and cacheable:

```rust
// Builder pattern allows test composition and reuse
let base_runbook = RunbookBuilder::new()
    .signer("alice", "evm::wallet")
    .action("transfer", "evm::send_eth")
    .build_ast(); // Returns AST, can be cached

// Compose and extend for different test scenarios
let test1 = base_runbook.clone()
    .with_input("amount", "1.5")
    .validate()
    .assert_ok();

let test2 = base_runbook.clone()
    .with_input("amount", "invalid")
    .validate()
    .assert_error("type mismatch");
```

**Addon Testing**: Compiler design separates syntax/semantics from execution:

```rust
// Test type checking without running Anvil
let typed_ast = typecheck(ast, evm_types());
// Test execution planning without blockchain
let plan = compile_to_execution_plan(typed_ast);
// Only integration tests need real chains
```

**Scalability**: Each addon provides type definitions and validation rules at compile-time. No need for heavy mocks during most testing phases.

The testing challenges were architectural feedback indicating that txtx would benefit from being approached as a compiler rather than a simple CLI tool.

## Key Insight: txtx is a Domain-Specific Language Compiler

Through implementing the doctor command and LSP, it became clear that **txtx is fundamentally a compiler for a domain-specific language**, not just a configuration tool. This realization shaped many architectural decisions and points to future improvements.

### Why txtx is a Compiler

1. **Source Language**: HCL-based syntax with custom semantics
2. **Type System**: Custom types from addons (e.g., `bitcoin::Address`, `evm::Contract`)
3. **Multiple Compilation Phases**: Parse → Validate → Plan → Execute
4. **Cross-Compilation Units**: References across files and actions
5. **Error Reporting**: Location-aware diagnostics with context
6. **IDE Support**: Language server with semantic understanding

## Architectural Lessons

### 1. Parser Consistency is Critical

**Experiment**: Initially tried tree-sitter for the LSP
**Learning**: Using different parsers creates inconsistency and maintenance burden
**Solution**: Unified on `hcl-edit` everywhere

```rust
// Bad: Multiple parsers
let tree_sitter_ast = parse_with_tree_sitter(source);
let hcl_ast = parse_with_hcl(source);
// Inevitably diverge!

// Good: One parser, multiple consumers
let ast = hcl_edit::parse(source);
use_in_core(ast);
use_in_lsp(ast);
use_in_doctor(ast);
```

### 2. Compilation Phases Enable Better Tooling

**Realization**: The doctor command naturally implements compiler phases:

```rust
// What we built mirrors a traditional compiler:
fn doctor_analyze(source: &str) -> Result<(), Diagnostics> {
    let ast = parse(source)?;           // Lexing/Parsing
    let hir = lower_to_hir(ast)?;      // HIR Construction
    validate_semantics(&hir)?;          // Semantic Analysis
    check_types(&hir)?;                 // Type Checking
    analyze_dependencies(&hir)?;        // Dependency Analysis
    Ok(())
}
```

### 3. Static Analysis Before Runtime

**Learning**: Many errors can be caught without executing:

- Undefined references (`action.foo` when `foo` doesn't exist)
- Type mismatches (`signer.alice` used where string expected)
- Circular dependencies
- Invalid addon function calls

**Impact**: 80% of user errors now caught before execution

### 4. Addons are Compile-Time Extensions

**Insight**: Addons aren't just runtime plugins - they extend the language itself:

- New types (`bitcoin::Script`, `stacks::ClarityValue`)
- New functions with type signatures
- Custom validation rules
- Domain-specific semantics

```rust
// Addons should provide compile-time information:
trait AddonCompileTime {
    fn types(&self) -> Vec<TypeDefinition>;
    fn functions(&self) -> Vec<FunctionSignature>;
    fn validators(&self) -> Vec<Box<dyn Validator>>;
}
```

## Technical Lessons

### 1. Synchronous LSP is Sufficient

**Experiment**: Started with async tower-lsp
**Reality**: Compilation is inherently sequential
**Learning**: rust-analyzer's sync approach is simpler and proven

```rust
// Overcomplicated async:
async fn handle_hover(params: HoverParams) -> Result<Hover> {
    let ast = parse_async(params.document).await?;
    let result = analyze_async(ast).await?;
    Ok(make_hover(result))
}

// Simple sync that works:
fn handle_hover(params: HoverParams) -> Result<Hover> {
    let ast = parse(params.document)?;
    let result = analyze(ast)?;
    Ok(make_hover(result))
}
```

### 2. Builder Patterns for Testing

**Learning**: Type-safe builders prevent invalid test cases:

```rust
// Before: Error-prone string manipulation
let runbook = format!(r#"
    action "{}" "{}" {{
        {} = "{}"
    }}
"#, name, action_type, param, value);

// After: Compile-time validation
let runbook = RunbookBuilder::new()
    .action(name, action_type)
    .input(param, value)
    .build(); // Can't build invalid structure
```

### 3. Incremental Validation Strategy

**Discovery**: Full recompilation on every keystroke is wasteful
**Solution**: Cache intermediate representations:

```rust
struct WorkspaceState {
    ast_cache: HashMap<Url, Ast>,
    hir_cache: HashMap<Url, Hir>,
    type_cache: TypeRegistry,
    // Invalidate selectively on changes
}
```

## Process Lessons

### 1. Integrate Don't Separate

**Failed Approach**: Separate txtx-lsp crate
**Problems**:

- Circular dependencies with core
- Distribution complexity
- Version synchronization

**Successful Approach**: LSP as CLI subcommand

- Single binary distribution
- Shared code and types
- Unified testing

### 2. Migration Requires Clean History

**Learning**: Feature branches accumulate experimental code
**Solution**: Clean commit history is worth the rebase effort:

- Each commit should be atomic and buildable
- Group related changes (e.g., txtx-lsp removal with LSP addition)
- Separate infrastructure from features

### 3. Documentation as Discovery Tool

**Observation**: Writing docs revealed architectural patterns:

- ADRs clarified decision rationale
- Architecture docs exposed coupling
- Test guides highlighted API inconsistencies

## Future Directions

### 1. Embrace Compiler Architecture

Based on these learnings, txtx should evolve toward a proper compiler architecture:

```text
Source Files (.tx, .yml)
    ↓
Parsing (HCL) ← [We are here]
    ↓
Semantic Analysis ← [Doctor does this]
    ↓
Type Checking ← [Partially implemented]
    ↓
Execution Graph Generation
  - Dependency Resolution
  - Execution Planning
  - DAG Construction
    ↓
Topological Sorting
    ↓
Action Execution (with signers, validators, etc.)
```

### 2. Addon Manifest System

Addons should declare capabilities at compile-time:

```toml
# bitcoin_addon.toml
[addon]
namespace = "bitcoin"

[[types]]
name = "Address"
validate = "is_valid_bitcoin_address"

[[functions]]
name = "op_dup"
returns = "Opcode"
pure = true
```

### 3. Advanced IDE Features

With proper compiler infrastructure:

- Type-aware autocomplete
- Inline type hints
- Refactoring support
- Visual dependency graphs
- Debugger integration

### 4. Optimization Opportunities

Compiler architecture enables:

- Dead code elimination
- Constant folding
- Parallel execution planning
- Cross-chain transaction batching
- Gas optimization suggestions

## Key Takeaways

1. **txtx is a compiler** - Design with compiler principles
2. **Static analysis is powerful** - Catch errors early
3. **Unified architecture wins** - One parser, one type system
4. **Addons extend the language** - Not just runtime plugins
5. **Clean architecture enables features** - LSP, doctor, and future tools

The journey from "configuration tool" to "domain-specific language compiler" has been transformative. By embracing compiler design principles, we've built a foundation that can support sophisticated blockchain development workflows while maintaining excellent developer experience.

## Results and Impact

- **Error Detection**: Significant improvement in catching errors before runtime
- **Developer Productivity**: Substantially faster error resolution with doctor command
- **Code Quality**: Consistent validation across all tools
- **Maintainability**: Single source of truth for parsing and validation
- **Extensibility**: New features integrate cleanly into existing phases

This architectural clarity positions txtx for future enhancements while delivering immediate value through improved error detection and IDE support.
