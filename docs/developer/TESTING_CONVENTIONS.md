# Testing Conventions for txtx

This document outlines the testing conventions and cargo aliases used in the txtx project.

## Test Organization

### Unit Tests

- **Location**: Within `src/` directories alongside the code they test
- **Purpose**: Test individual functions, modules, and components in isolation
- **Naming**: Follow Rust conventions (`#[cfg(test)] mod tests`)

### Integration Tests  

- **Location**: In `tests/` directories at the crate level
- **Purpose**: Test complete workflows and interactions between components
- **Naming**: Descriptive test file names (e.g., `doctor_tests.rs`, `lsp_hover_test.rs`)

## Cargo Test Aliases

We use a consistent naming pattern for test aliases: `test-[scope]-[type]-[target]`

### Pattern Components

- **scope**: The crate being tested (e.g., `cli`, `core`, `addon-kit`)
- **type**: Either `unit` or `int` (integration)
- **target**: Optional specific module or test file

### Available Aliases

#### Unit Test Aliases

```bash
cargo test-cli-unit           # All unit tests in txtx-cli
cargo test-cli-unit-doctor    # Only doctor module unit tests
cargo test-cli-unit-lsp       # Only LSP module unit tests
cargo test-core-unit          # All unit tests in txtx-core
cargo test-addon-kit-unit     # All unit tests in txtx-addon-kit
```

#### Integration Test Aliases

```bash
cargo test-cli-int            # All integration tests for txtx-cli
cargo test-cli-int-doctor     # Original doctor integration tests
cargo test-cli-int-doctor-new # New doctor tests using RunbookBuilder
cargo test-cli-int-lsp        # LSP integration tests
```

#### Convenience Aliases

```bash
cargo test-cli                # All CLI tests (unit + integration)
cargo build-cli               # Build CLI without supervisor UI
cargo build-cli-release       # Release build without supervisor UI
```

## Examples

### Testing a specific module

```bash
# Run only doctor unit tests
cargo test-cli-unit-doctor

# Run only doctor integration tests
cargo test-cli-int-doctor
```

### Testing during development

```bash
# Quick test run without supervisor UI build
cargo test-cli-unit

# Test the new RunbookBuilder API
cargo test-cli-int-doctor-new
```

### Running specific test patterns

```bash
# Run a specific test by name
cargo test-cli-unit test_input_defined_rule

# Run tests matching a pattern
cargo test-cli-int validation
```

## RunbookBuilder vs Integration Tests

### When to Use RunbookBuilder

The `RunbookBuilder` in `txtx-test-utils` is ideal for:

- Unit testing HCL syntax validation
- Testing basic semantic errors (unknown namespaces, action types)
- Quick validation tests that focus on runbook structure
- Reducing boilerplate in test code

### When to Use Integration Tests

Keep integration tests for scenarios that RunbookBuilder cannot handle:

- **Doctor-specific validation**: Undefined signers, invalid field access, cross-references
- **Multi-file runbooks**: Testing file imports and includes
- **Command behavior**: Testing exact error messages, line numbers, JSON output
- **Flow validation**: Testing flow variables and flow-specific rules
- **Full validation pipeline**: When you need the complete doctor analysis

### Example Decision

```rust
// ✅ Use RunbookBuilder for basic validation
#[test]
fn test_unknown_namespace() {
    let result = RunbookBuilder::new()
        .action("test", "invalid::action")
        .validate();
    assert_validation_error!(result, "Unknown addon namespace");
}

// ❌ Use integration test for doctor-specific checks
#[test]
fn test_undefined_signer_reference() {
    // This needs the full doctor command to catch the error
    let output = Command::new("txtx")
        .arg("doctor")
        .arg("fixture.tx")
        .output()
        .unwrap();
    // Doctor catches undefined signer refs that RunbookBuilder doesn't
}
```

## Notes

- All CLI test aliases use `--no-default-features --features cli` to avoid building the supervisor UI
- The supervisor UI is an optional dependency that significantly increases build time
- Use the specific aliases to run only the tests you need during development
- RunbookBuilder uses `txtx_core::validation::hcl_validator` which provides HCL parsing but not the full doctor analysis
