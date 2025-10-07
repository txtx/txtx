# txtx Developer Guide

## Documentation

**For API documentation, module structure, and code details, use:**

```bash
cargo doc --open --no-deps
```

This guide covers only development workflows, testing strategies, and project conventions not captured in the Rust documentation.

## Development Setup

### Prerequisites

- Rust toolchain (see rust-toolchain.toml)
- `just` command runner: `cargo install just`
- `cargo-llvm-cov` for coverage: `cargo install cargo-llvm-cov`

### Quick Start

```bash
# Show available commands
just

# Run tests
just cli-unit        # CLI unit tests
just lint-unit       # Linter unit tests
just lsp-unit        # LSP unit tests

# Generate coverage report
just coverage
```

## Build Configuration

### Building without Supervisor UI

The supervisor UI requires privileged build tools. For development, use:

```bash
just build  # Alias for: cargo build --package txtx-cli --no-default-features --features cli
```

## Testing Strategy

### Test Organization

- Unit tests: Next to implementation in `src/`
- Integration tests: In `tests/` directories
- Fixtures: In `tests/fixtures/`

### Running Tests

```bash
# Unit tests
just cli-unit        # All CLI unit tests
just lint-unit       # Linter unit tests
just lsp-unit        # LSP unit tests

# Integration tests
just cli-int         # CLI integration tests
just lint-int        # Linter integration tests
just lsp-int         # LSP integration tests

# Specific test
just test <test_name>

# With output visible
just test-verbose <test_name>

# With coverage
just coverage
```

### Test Coverage Goals

Critical modules requiring high coverage:

- `cli/linter_impl/analyzer/rules.rs` - Validation rules
- `cli/linter_impl/analyzer/visitor.rs` - AST traversal
- `validation/hcl_validator.rs` - Core validation logic

## Code Style

### Rust Philosophy

- Self-documenting code through clear naming and types
- Comments only where they add value beyond what code expresses
- Doc comments for public APIs
- Avoid redundant inline comments

### Example

```rust
// ❌ Redundant
// Create validation context with all necessary data
let mut context = ValidationContext::new(content.to_string(), file_path.to_string_lossy());

// ✅ Clear without comment
let mut context = ValidationContext::new(content.to_string(), file_path.to_string_lossy());

// ✅ Value-adding comment
pub full_name: &'a str, // e.g., "input.my_var"
```

## Project Structure

### Key Directories

- `crates/txtx-cli/src/cli/linter_impl/` - Linter implementation
- `crates/txtx-cli/src/cli/lsp/` - Language Server Protocol
- `crates/txtx-core/src/validation/` - Core validation logic
- `addons/` - Network-specific addon implementations

### Architecture Decisions

See `docs/adr/` for Architecture Decision Records documenting key design choices.

## Contributing

### Adding a Validation Rule

1. Implement `ValidationRule` trait in `analyzer/rules.rs`
2. Add to `get_default_rules()` or `get_strict_rules()`
3. Add tests in the impl module
4. Update integration tests if needed

### Workflow

1. Make changes
2. Run `just lint-unit` to verify linter tests
3. Run `just cli-unit` for full test suite
4. Ensure documentation builds: `just doc`

## Common Issues

### Build Errors

- "No such file or directory": You're building with supervisor UI. Use `just build`
- Deprecation warnings: Expected from dependencies, suppressed in justfile commands

### Test Failures

- Check if you need to run from project root
- Ensure test fixtures exist in `tests/fixtures/`
- For coverage, ensure `cargo-llvm-cov` is installed

## Additional Resources

- [Architecture Decision Records](docs/adr/) - Design decisions and rationale
- [Validation Architecture](docs/developer/VALIDATION_ARCHITECTURE.md) - Deep dive into validation system design
- [Testing Guide](docs/developer/TESTING_GUIDE.md) - Testing documentation
- [Testing Conventions](docs/developer/TESTING_CONVENTIONS.md) - Test writing standards
- Generated Rust docs: `cargo doc --open --no-deps`
