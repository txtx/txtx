# txtx Development Guide

## Overview

This guide provides essential information for contributing to the txtx project, including development setup, workflow commands, coding principles, and architectural patterns established during the 2024 refactoring.

## Quick Start with Justfile

The txtx project uses [just](https://github.com/casey/just) as a command runner to streamline development workflows. The justfile provides convenient shortcuts for common development tasks while handling build configurations and suppressing noisy compiler warnings during development.

### Prerequisites

Install `just` command runner:

```bash
# macOS
brew install just

# Linux/WSL
cargo install just

# Or download from https://github.com/casey/just/releases
```

### Common Development Commands

```bash
# Show all available commands
just

# Or explicitly:
just --list
```

## Testing Commands

### Quick Test Commands

The justfile provides focused test commands that automatically suppress development warnings and exclude supervisor dependencies:

#### Doctor Command Tests

```bash
just dr         # Run all doctor tests (unit + integration)
just dr-unit    # Run doctor unit tests only
just dr-int     # Run doctor integration tests only
```

#### LSP Tests

```bash
just lsp        # Run all LSP tests (unit + integration)
just lsp-unit   # Run LSP unit tests only
just lsp-int    # Run LSP integration tests only
```

#### CLI Tests

```bash
just cli        # Run all CLI tests (unit + integration)
just cli-unit   # Run CLI unit tests only
just cli-int    # Run CLI integration tests only
```

### Test Patterns and Specific Tests

```bash
# Run tests matching a specific name
just test-by-name test_undefined_variable

# Run tests matching a pattern
just test-match doctor::analyzer

# Run tests with output visible
just test-dev-verbose
```

### Watch Mode

For test-driven development:

```bash
# Watch for changes and run tests
just watch

# Watch with warnings suppressed
just watch-dev
```

## Build Commands

```bash
# Build CLI without supervisor dependencies
just build-cli

# Build CLI in release mode
just build-cli-release

# Quick check without building
just check
```

## Code Quality

```bash
# Format all code
just fmt

# Run clippy linter
just clippy

# Run clippy with development flags (warnings suppressed)
just clippy-dev
```

## Documentation

```bash
# Generate docs for txtx-cli
just doc

# Generate and open docs in browser
just doc-open

# Generate docs for all packages
just doc-all

# Include private items
just doc-private
```

## Development Workflow

### Typical Development Cycle

1. **Start with a clean build:**

   ```bash
   just clean
   just build-cli
   ```

2. **Make changes and test iteratively:**

   ```bash
   just dr-unit     # Test specific component
   just watch-dev   # Or use watch mode
   ```

3. **Run full test suite before committing:**

   ```bash
   just dr          # Test doctor
   just lsp         # Test LSP
   just cli         # Test CLI
   ```

4. **Check code quality:**

   ```bash
   just fmt         # Format code
   just clippy      # Run linter
   ```

### Working on Specific Features

#### Doctor Command Development

```bash
# Quick iteration on doctor tests
just dr-unit

# Test with output to see diagnostic messages
RUSTFLAGS="-A unused_assignments -A unused_variables" \
  cargo test --package txtx-cli cli::doctor -- --nocapture

# Run the example doctor demo
cd addons/evm/fixtures/doctor_demo
./doctor_demo.sh
```

#### LSP Development

```bash
# Test LSP functionality
just lsp-unit

# Build and test VSCode extension
cd vscode-extension
npm install
npm run package
# Press F5 in VSCode to test
```

## Key Benefits of Using Justfile

### 1. **Suppressed Development Warnings**

The justfile automatically sets `RUST_DEV_FLAGS` to suppress common development warnings:

- Unused assignments
- Unused variables
- Dead code
- Unused imports

This keeps test output clean and focused on actual failures.

### 2. **No Supervisor Dependencies**

All commands use `--no-default-features --features cli` to exclude supervisor UI dependencies, resulting in:

- Faster builds
- Simpler test environment
- No UI-related test failures

### 3. **Organized Test Suites**

Commands are organized by component (doctor, LSP, CLI) with granular control over unit vs integration tests.

### 4. **Consistent Environment**

All commands use the same flags and configurations, ensuring consistent behavior across different development machines.

## Troubleshooting

### Tests Failing with Warnings

If you see test failures due to warnings, ensure you're using the just commands which suppress them:

```bash
# Instead of:
cargo test --package txtx-cli

# Use:
just cli
```

### Build Issues

Clear the build cache and rebuild:

```bash
just clean
just build-cli
```

### Missing `just` Command

If `just` is not found, you can still run the underlying commands directly:

```bash
# View what a just command does:
just --show dr

# Run the command manually:
RUSTFLAGS="-A unused_assignments -A unused_variables -A dead_code -A unused_imports" \
  cargo test --package txtx-cli --no-default-features --features cli cli::doctor
```

## Advanced Usage

### Custom RUSTFLAGS

Override the default development flags:

```bash
RUST_DEV_FLAGS="-A dead_code" just dr
```

### Verbose Output

For debugging test failures:

```bash
just test-dev-verbose
```

### Parallel Test Execution

Tests run in parallel by default. To run serially:

```bash
RUST_TEST_THREADS=1 just dr
```

## Contributing

When contributing to txtx:

1. **Use justfile commands** for consistency with CI
2. **Run full test suite** before submitting PRs
3. **Keep tests fast** - use unit tests where possible
4. **Document new just recipes** if you add them

## Coding Principles & Architecture

### Core Principles

#### 1. Modular Architecture Over Monolithic Files

**❌ Avoid**: Single files exceeding 500 lines with mixed responsibilities
**✅ Prefer**: Modular structure with clear separation of concerns

```console
module/
├── mod.rs           # Thin orchestrator (<200 lines)
├── config.rs        # Configuration types
├── types.rs         # Shared types
├── submodule1/      # Feature-specific logic
└── submodule2/      # Feature-specific logic
```

#### 2. Trait-Based Extensibility

**❌ Avoid**: Hard-coded switch statements and if-else chains
**✅ Prefer**: Trait-based design for extensible behavior

```rust
// Define clear trait boundaries
pub trait ValidationRule {
    fn name(&self) -> &str;
    fn validate(&self, context: &Context) -> Vec<Diagnostic>;
}

// Implement specific behaviors
struct MyRule;
impl ValidationRule for MyRule { ... }
```

#### 3. Composition Over Inheritance

**❌ Avoid**: Deep inheritance hierarchies or complex state machines
**✅ Prefer**: Compose small, focused components

```rust
// Compose validators
let validator = Validator::new()
    .add_rule(InputRule::new())
    .add_rule(FlowRule::new())
    .add_rule(SecurityRule::new());
```

#### 4. Explicit Over Implicit

**❌ Avoid**: Magic strings, hidden dependencies, global state
**✅ Prefer**: Explicit dependencies, clear interfaces

```rust
// Bad: Hidden dependency
fn validate() {
    let config = CONFIG.get(); // Global state
}

// Good: Explicit dependency
fn validate(config: &Config) {
    // Use provided config
}
```

### Architectural Patterns

#### Handler Pattern for Request/Response

When building request/response systems (like LSP):

```rust
trait Handler {
    type Request;
    type Response;
    fn handle(&self, req: Self::Request, ctx: &Context) -> Result<Self::Response>;
}
```

#### Visitor Pattern for AST Traversal

When processing hierarchical data:

```rust
trait Visitor {
    fn visit_block(&mut self, block: &Block);
    fn visit_expression(&mut self, expr: &Expression);
}
```

#### Builder Pattern for Complex Configuration

When constructing complex objects:

```rust
WorkspaceBuilder::new()
    .manifest_path("./txtx.yml")
    .environment("production")
    .build()?
```

### Code Organization

#### File Structure Guidelines

- **mod.rs**: Public API and orchestration only
- **types.rs**: Shared types and traits
- **impl.rs**: Private implementation details
- **tests.rs**: Unit tests (or separate tests/ directory)

#### Module Boundaries

- Each module should have a single, clear purpose
- Dependencies should flow in one direction
- Circular dependencies indicate poor boundaries

#### Error Handling

```rust
// Define module-specific error types
#[derive(Debug, thiserror::Error)]
pub enum DoctorError {
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// Use Result type alias
pub type Result<T> = std::result::Result<T, DoctorError>;
```

### Testing Strategy

#### Test Organization

```console
tests/
├── unit/           # Fast, isolated unit tests
├── integration/    # Cross-module integration tests
└── fixtures/       # Test data and examples
```

#### Test Principles

- **Fast**: Unit tests should run in milliseconds
- **Isolated**: Tests shouldn't depend on external state
- **Descriptive**: Test names should explain the scenario

```rust
#[test]
fn validation_rule_detects_missing_flow_attribute() {
    // Arrange
    let rule = FlowAttributeRule::new();
    let context = test_context();

    // Act
    let diagnostics = rule.validate(&context);

    // Assert
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("missing attribute"));
}
```

### Performance Considerations

#### Lazy Evaluation

**❌ Avoid**: Eagerly computing all possibilities
**✅ Prefer**: Compute only what's needed

```rust
// Bad: Loads all files immediately
let all_files = load_all_files(&directory)?;

// Good: Returns iterator that loads on demand
let files = directory.files()
    .filter(|f| f.extension() == "tx")
    .map(|f| load_file(f));
```

#### Caching Strategy

Cache expensive computations at appropriate boundaries:

```rust
struct Workspace {
    manifests: Cache<PathBuf, Manifest>,
}
```

### Documentation Standards

#### Module Documentation

Every module should have a clear purpose:

```rust
//! # Doctor Module
//!
//! Provides validation and linting for txtx runbooks.
//!
//! ## Architecture
//! - `analyzer/`: Core validation logic
//! - `formatter/`: Output formatting
//! - `rules/`: Validation rule implementations
```

#### Public API Documentation

All public items need documentation:

```rust
/// Validates a runbook and returns diagnostics.
///
/// # Arguments
/// * `runbook` - Path to the runbook file
/// * `config` - Validation configuration
///
/// # Returns
/// A vector of diagnostics, empty if validation passes
pub fn validate(runbook: &Path, config: &Config) -> Vec<Diagnostic> {
```

## Code Review Checklist

Before submitting PRs, ensure:

- [ ] No single file exceeds 500 lines
- [ ] Clear module boundaries with single responsibilities
- [ ] Traits used for extensible behavior
- [ ] Dependencies are explicit (no global state)
- [ ] Tests pass and cover new functionality
- [ ] Documentation updated for public APIs
- [ ] Error handling uses proper types
- [ ] Performance implications considered
- [ ] Justfile commands used for testing (`just dr`, `just lsp`, etc.)

## Metrics for Success

A well-architected module should have:

- **Orchestrator**: <200 lines
- **Components**: <300 lines each
- **Clear boundaries**: Can explain purpose in one sentence
- **Testability**: >80% test coverage achievable
- **Extensibility**: Adding features doesn't require modifying core

## See Also

- [Testing Guide](TESTING_GUIDE.md) - Detailed testing documentation
- [Testing Conventions](TESTING_CONVENTIONS.md) - Test organization and best practices
- [LSP Architecture](lsp-architecture.md) - LSP implementation details
- [Doctor Architecture](doctor-architecture.md) - Doctor command implementation
