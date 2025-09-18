# PR Summary: Doctor Command and LSP Implementation

## Executive Summary

This PR introduces fundamental improvements to txtx's architecture by adding a doctor command and Language Server Protocol (LSP) support. These features emerged from recognizing that txtx is fundamentally a **domain-specific language compiler**, not just a configuration tool. This shift in perspective enabled better validation, IDE integration, and dramatically improved the developer experience through structured testing patterns.

## Overview

This branch transforms txtx into a professional development environment by introducing:

1. **Doctor Command** - A diagnostic tool for what ails your runbooks
2. **Language Server Protocol (LSP)** - Full IDE integration with real-time validation and intelligent code assistance
3. **Unified Validation Architecture** - A compiler-inspired validation pipeline shared between features
4. **Developer Experience Improvements** - Build aliases, test utilities, and structured testing patterns that streamline development

These additions reflect the understanding that txtx requires compiler-like infrastructure to properly validate, analyze, and execute blockchain automation runbooks.

## Architectural Context

### The Context

As documented in [LESSONS_LEARNED.md](./LESSONS_LEARNED.md), the development process revealed opportunities for architectural improvements. Analysis showed that significant development time was being spent on test infrastructure and addon integration complexity. This observation indicated that txtx's evolution would benefit from compiler-like infrastructure to better support its growing capabilities.

### The Solution

Implementing the doctor command and LSP forced a reconceptualization of txtx as a domain-specific language compiler, leading to:

- Clear separation of compilation phases (parsing, validation, execution)
- Type-safe test construction with builder patterns
- Isolated, testable components
- Proper error handling and diagnostics infrastructure

## Major Features Introduced

### 1. **Doctor Command** (`txtx doctor`)

A diagnostic/linter tool to analyze your txtx runbooks. 

**Key Features:**

- Multi-phase validation pipeline (syntax → semantic → manifest validation)
- Multiple output formats (terminal, JSON, quickfix)
- Enhanced diagnostics with context and suggestions
- Extensible rule system for custom validations

### 2. **Language Server Protocol (LSP) Implementation**

A full-featured LSP server that treats txtx as a proper programming language.

**Key Features:**

- Real-time syntax and semantic validation
- Intelligent completions based on addon types
- Hover documentation from addon specifications
- Cross-file reference resolution
- Workspace-aware validation against manifests

### 3. **Unified Validation Architecture**

A compiler-inspired validation infrastructure shared between doctor and LSP.

**Key Components:**

- Multi-phase validation pipeline (parse → validate → analyze)
- Type-aware validation using addon specifications
- Manifest-based environment validation
- Extensible rule system for custom checks

## Architecture Overview

### Compiler-Inspired Pipeline

```text
Source Files (.tx)
      ↓
[Lexing/Parsing] → AST with location info
      ↓
[Syntax Validation] → Syntactically valid AST
      ↓
[Semantic Analysis] → Type-checked AST
      ↓
[Manifest Validation] → Environment-validated AST
      ↓
[Doctor Rules] → Best-practice validated AST
      ↓
[Output Generation] → Diagnostics/LSP responses
```

This pipeline reflects the compiler architecture insights that emerged during development, providing clear separation of concerns and testable phases.

## Key Technical Decisions

### 1. **Compiler Architecture Adoption**

- **Decision**: Structure validation as a multi-phase compiler pipeline
- **Rationale**: Enables isolated testing, better error reporting, and future optimizations
- **Context**: Emerged from the realization that txtx is a DSL compiler, not just a config tool

### 2. **Integrated CLI Design**

- **Decision**: Both doctor and LSP integrated into main CLI binary
- **Rationale**: Simplified distribution and shared infrastructure
- **Learning**: As noted in [EXPERIMENTS.md](./EXPERIMENTS.md), separate crates created circular dependencies

### 3. **Synchronous LSP Implementation**

- **Decision**: Use rust-analyzer's synchronous `lsp-server` instead of async tower-lsp
- **Rationale**: Tower-lsp is unmaintained; synchronous design is simpler and sufficient
- **Learning**: Async complexity provided no real benefits for our use case

### 4. **Builder-Based Testing**

- **Decision**: Introduce `RunbookBuilder` for type-safe test construction
- **Rationale**: Streamlines test creation by making tests composable and maintainable
- **Impact**: Tests are now readable, refactorable, and don't require blockchain mocks for most scenarios

## Developer Experience Improvements

### 1. **Streamlined Testing Infrastructure**

The primary motivation for these improvements was to address the testing complexity that had become a significant development bottleneck:

#### Justfile for Developer Workflows

A `justfile` now provides streamlined development commands with automatic warning suppression and proper feature flags:

```bash
# Component-focused testing with granular control
just dr         # Run all doctor tests (unit + integration)
just dr-unit    # Run doctor unit tests only
just dr-int     # Run doctor integration tests only

just lsp        # Run all LSP tests
just lsp-unit   # Run LSP unit tests only
just lsp-int    # Run LSP integration tests only

# Additional developer conveniences
just test-by-name test_undefined_variable  # Run specific test
just test-match doctor::analyzer           # Run tests matching pattern
just watch-dev                             # Watch mode with auto-rerun
just clippy-dev                           # Linting with warnings suppressed
```

**Key Benefits of Justfile:**

1. **Automatic Warning Suppression**: All commands set `RUST_DEV_FLAGS` to suppress development noise (unused variables, dead code, etc.), keeping test output clean and focused

2. **No Supervisor Dependencies**: Commands use `--no-default-features --features cli` to exclude UI dependencies, eliminating webkit build issues

3. **Organized by Component**: Tests are grouped by feature (doctor, LSP, CLI) with clear separation of unit vs integration tests

4. **Consistent Environment**: All commands use the same flags and configurations across machines

#### Build and Test Aliases

```console
# Build without supervisor UI dependencies
cargo build-cli           # No webkit/supervisor issues
cargo build-cli-release

# Focused testing commands (alternative to justfile)
cargo test-cli-unit-doctor # Test just doctor logic
cargo test-cli-unit-lsp    # Test just LSP logic
```

**Impact**: Contributors can now work on core features without dealing with supervisor UI dependencies or running unrelated tests. The justfile approach has become the preferred method due to its convenience and consistency.

#### Type-Safe Test Construction with RunbookBuilder

The `RunbookBuilder` directly addresses test fixture management challenges:

```rust
// Before: String manipulation and fixture files
let content = read_fixture("evm_deploy.tx");
let modified = content.replace("$CONTRACT", "./Token.sol");

// After: Composable, type-safe builders
let result = RunbookBuilder::new()
    .addon("evm", vec![("network_id", "1")])
    .action("deploy", "evm::deploy_contract")
        .input("contract", "./Token.sol")
    .validate();  // No blockchain mocks needed!
```

**Key Innovation**: By separating validation testing from execution testing, most tests no longer need blockchain mocks. This significantly reduces CI complexity and resource usage for validation tests.

#### Compiler-Inspired Test Organization

Following compiler architecture principles, tests are now organized by phase:

```rust
// Phase 1: Syntax validation tests
builder.validate_syntax()  // Just parsing, no addons needed

// Phase 2: Semantic validation tests  
builder.validate_semantic()  // Type checking, no runtime needed

// Phase 3: Full validation with environment
builder
    .with_environment("production", vec![("API_KEY", "$KEY")])
    .validate_full()  // Complete validation pipeline
```

This phase separation means:

- Syntax tests run in milliseconds (no addon loading)
- Semantic tests don't need blockchain mocks
- Only integration tests require full infrastructure

## Supporting Infrastructure

### 1. **Test Utilities** (`txtx-test-utils`)

A new crate that embodies the compiler-inspired testing approach:

**Core Components:**

- **RunbookBuilder**: Type-safe test construction without string manipulation
- **SimpleValidator**: Validation-only testing without execution infrastructure
- **Phased Validation**: Separate syntax, semantic, and full validation
- **TestHarness**: Existing execution testing (moved from txtx-core)

**Architecture Impact**: This separation reflects the understanding that txtx has distinct compilation phases (validation) and execution phases, each requiring different testing approaches.

### 2. **Common Utilities** (`txtx-cli/src/cli/common`)

Shared infrastructure for CLI commands:

- **Addon Registry** - Loads all blockchain addons
- **Specification Extraction** - Extracts command specs from addons

### 3. **Enhanced Error Types**

New error types with rich context:

```rust
pub struct ValidationError {
    pub message: String,
    pub file: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub context: Option<String>,
    pub suggestion: Option<ValidationSuggestion>,
    pub documentation_link: Option<String>,
}
```

## File Structure

### New Directories

```text
crates/txtx-cli/src/cli/
├── doctor/                 # Doctor command implementation
│   ├── analyzer/          # Validation rules and logic
│   ├── formatter/         # Output formatters
│   └── tests/            # Doctor-specific tests
├── lsp/                   # LSP server implementation
│   ├── handlers/         # Request handlers
│   ├── workspace/        # State management
│   ├── validation/       # Validation adapters
│   └── tests/           # LSP tests
└── common/               # Shared utilities

crates/txtx-core/src/
└── validation/           # Core validation logic
    ├── hcl_validator.rs  # HCL-based validation
    ├── manifest_validator.rs # Manifest validation
    └── doctor_rules.rs   # Validation rules

crates/txtx-test-utils/   # New test utilities crate

vscode-extension/         # VSCode extension
```

## Usage Examples

### Doctor Command

```bash
# Basic validation
txtx doctor

# Validate specific runbook
txtx doctor deploy

# With environment and inputs
txtx doctor deploy -e production --input api_key=$API_KEY

# JSON output for CI
txtx doctor --format json > validation-results.json
```

### LSP with VSCode

1. Install the extension from `vscode-extension/`
2. Open a `.tx` file
3. Get real-time validation, completions, and hover docs
4. Ctrl+Click to go to definitions

## Testing

### Test Coverage

Both features include extensive test suites:

**Doctor Tests:**

- Unit tests for each validation rule
- Integration tests with sample runbooks
- Multi-file runbook validation tests
- Output format tests

**LSP Tests:**

- Protocol handling tests
- Handler functionality tests
- Workspace state management tests
- Multi-file support tests

### New Testing Patterns

With the RunbookBuilder API, tests are now more maintainable:

```rust
#[test]
fn test_undefined_signer_reference() {
    let result = RunbookBuilder::new()
        .addon("evm", vec![])
        .action("transfer", "evm::send_transaction")
            .input("signer", "signer.missing")  // Undefined signer
        .validate();
    
    assert_validation_error!(result, "undefined signer 'missing'");
}

#[test] 
fn test_environment_variable_resolution() {
    let result = RunbookBuilder::new()
        .addon("evm", vec![])
        .action("deploy", "evm::deploy_contract")
            .input("rpc_url", "env.RPC_URL")
        .with_environment("test", vec![("RPC_URL", "http://localhost:8545")])
        .set_current_environment("test")
        .validate();
    
    assert_validation_passes!(result);
}
```

## Documentation

New documentation reflects the architectural insights:

- `docs/VALIDATION_ARCHITECTURE.md` - Compiler-inspired validation pipeline
- `docs/developer/TESTING_GUIDE.md` - How to use the new testing infrastructure
- `docs/developer/TESTING_CONVENTIONS.md` - Testing patterns and best practices
- Implementation guides for doctor and LSP features
- User guides for new functionality

## Impact

### For Users

- Professional IDE experience with real-time validation
- diagnostics and validation before deployment
- Clear, actionable error messages
- Tooling comparable to mainstream programming languages

### For txtx Development

- **Improved Testing Efficiency**: Testing is now straightforward and fast
- **Clear Architecture**: Compiler model provides obvious extension points
- **Maintainable Tests**: Type-safe builders prevent test decay
- **Reduced Mock Complexity**: Most tests don't need blockchain simulators
- **Foundation for Future**: Ready for optimization, type inference, and more compiler features

## Future Enhancements

Building on the compiler architecture foundation:

### Compiler Pipeline Extensions

- **Optimization Phase**: Dead code elimination, constant folding
- **Type Inference**: Reduce boilerplate through type inference
- **Cross-Compilation**: Generate execution plans for different environments
- **Static Analysis**: Security scanning, gas optimization suggestions

### Advanced IDE Features

- **Refactoring Support**: Rename symbols across files
- **Code Actions**: Auto-fix common issues
- **Debugging**: Step through runbook execution
- **Performance Profiling**: Identify bottlenecks before deployment

### Testing Evolution

As outlined in [COMPILER_ARCHITECTURE_PROPOSAL.md](./COMPILER_ARCHITECTURE_PROPOSAL.md), the testing infrastructure can evolve to support:

- Property-based testing of compiler phases
- Differential testing against previous versions

## Summary

This PR represents a fundamental architectural evolution of txtx, driven by the recognition that it is a domain-specific language compiler. The doctor command and LSP implementation are the visible features, but the real transformation is in the underlying architecture and developer experience.

### Key Achievements

1. **Improved Testing Infrastructure**: Testing efficiency is dramatically improved through compiler-inspired architecture and type-safe test builders
2. **Professional Tooling**: txtx now has IDE support and validation comparable to mainstream programming languages
3. **Architectural Clarity**: The compiler model provides clear extension points and phases
4. **Sustainable Development**: Contributors can now work efficiently without blockchain mocking complexity

### Architectural Insights

As documented in our companion notes:

- [LESSONS_LEARNED.md](./LESSONS_LEARNED.md): How development challenges led to architectural insights
- [EXPERIMENTS.md](./EXPERIMENTS.md): Alternative approaches that informed our decisions
- [COMPILER_ARCHITECTURE_PROPOSAL.md](./COMPILER_ARCHITECTURE_PROPOSAL.md): Future vision for txtx as a full compiler

