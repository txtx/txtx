# txtx Testing Guide

This guide helps you write and run tests for the txtx project efficiently.

## Quick Start

### Running Tests with Justfile (Recommended)

The project includes a `justfile` that provides streamlined test commands with automatic warning suppression and proper feature flags:

```bash
# Component-specific test commands
just dr         # Run all doctor tests (unit + integration)
just dr-unit    # Run doctor unit tests only
just dr-int     # Run doctor integration tests only

just lsp        # Run all LSP tests (unit + integration)
just lsp-unit   # Run LSP unit tests only
just lsp-int    # Run LSP integration tests only

just cli        # Run all CLI tests (unit + integration)
just cli-unit   # Run CLI unit tests only
just cli-int    # Run CLI integration tests only

# Other useful commands
just test-by-name test_undefined_variable
just test-match doctor::analyzer
just watch-dev  # Watch mode with auto-rerun
```

See the [Development Guide](DEVELOPMENT_GUIDE.md) for complete justfile documentation.

### Running Tests with Cargo

```bash
# Run all tests quickly (excludes supervisor UI and problematic packages)
cargo test-quick

# Run tests for a specific package
cargo test-cli              # CLI tests only, no supervisor UI
cargo test --package txtx-core

# Run specific test by name
cargo test test_undefined_variable

# Run tests with output
cargo test-quick -- --nocapture
```

### Common Test Commands

| Command | Description |
|---------|-------------|
| `just dr` | Run all doctor tests with warning suppression |
| `just dr-unit` | Run doctor unit tests only |
| `just dr-int` | Run doctor integration tests only |
| `just lsp` | Run all LSP tests with warning suppression |
| `just lsp-unit` | Run LSP unit tests only |
| `just lsp-int` | Run LSP integration tests only |
| `just cli` | Run all CLI tests with warning suppression |
| `just cli-unit` | Run CLI unit tests only |
| `just cli-int` | Run CLI integration tests only |
| `cargo test-quick` | Run all tests excluding UI and stacks |
| `cargo test-cli` | Test CLI without supervisor UI |
| `cargo build-cli` | Build CLI without supervisor UI |
| `cargo test --package <name>` | Test specific package |

## Writing Tests

### Using RunbookBuilder (Recommended)

The `RunbookBuilder` provides a simple API for creating test scenarios with HCL validation:

```rust
use txtx_test_utils::{RunbookBuilder, assert_validation_error, assert_success};

#[test]
fn test_undefined_variable() {
    let result = RunbookBuilder::new()
        .addon("std", vec![])
        .action("test", "std::print")
            .input("message", "input.undefined_var")
        .validate();
        
    assert_validation_error!(result, "undefined");
}
```

#### RunbookBuilder Capabilities and Limitations

**What RunbookBuilder CAN test:**

- HCL syntax validation
- Basic semantic errors (unknown namespaces, invalid action types)
- Runbook structure and composition
- Environment variable presence (but not usage validation)

**What RunbookBuilder CANNOT test:**

- Doctor command's enhanced validation:
  - Undefined signer references
  - Invalid action output field access
  - Cross-references between actions
  - Flow variable validation
  - Input/environment variable usage validation
- Multi-file runbook imports
- Exact error messages and line numbers
- Command-specific behavior (doctor, LSP, etc.)

**When to use integration tests instead:**

```rust
// Use integration tests for doctor-specific validation
#[test]
fn test_doctor_catches_undefined_signer() {
    // This requires running the actual doctor command
    let output = Command::new("txtx")
        .arg("doctor")
        .arg("test.tx")
        .output()
        .unwrap();
    
    // Doctor catches errors that RunbookBuilder doesn't
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout)
        .contains("undefined signer"));
}
```

### Multi-file Runbook Tests

```rust
#[test]
fn test_multi_file_runbook() {
    let result = RunbookBuilder::new()
        .with_file("main.tx", r#"
            include "flows.tx"
            variable "api_key" { value = env.API_KEY }
        "#)
        .with_file("flows.tx", r#"
            action "test" "core::print" { 
                message = input.api_key 
            }
        "#)
        .with_environment("test", vec![
            ("API_KEY", "secret123")
        ])
        .execute();
        
    assert!(result.success);
}
```

### Testing with Mocks (Coming Soon)

```rust
#[test]
fn test_blockchain_interaction() {
    let mock = MockBlockchain::new()
        .with_account("0x123", 1000000)
        .with_gas_price(100);
        
    let result = RunbookBuilder::new()
        .with_content(include_str!("../fixtures/deploy.tx"))
        .with_mock("ethereum", mock)
        .execute();
        
    assert!(result.success);
}
```

## Test Organization

### Directory Structure

```console
crates/txtx-core/
├── src/
│   └── parser/
│       └── mod.rs
└── tests/          # Unit tests next to code
    └── parser/
        └── test_validation.rs

tests/              # Integration tests
├── fixtures/       # Test data files
└── integration/    # Cross-crate tests
```

### Test Categories

1. **Unit Tests**: Test individual functions/modules
   - Place in `tests/` subdirectory next to code
   - Use `#[cfg(test)]` modules for private API tests

2. **Integration Tests**: Test multiple components
   - Place in workspace-level `tests/` directory
   - Use `RunbookBuilder` for complex scenarios

3. **Snapshot Tests**: Test complex outputs
   - Use for doctor command output
   - Use for LSP responses
   - Review changes with `cargo insta review`

## Common Patterns

### Testing Validation Errors

```rust
#[test]
fn test_validation_error() {
    let runbook = r#"
        action "deploy" "evm::deploy_contract" {
            signer = undefined_signer
        }
    "#;
    
    let result = validate_runbook(runbook);
    assert_error!(result, ValidationError::UndefinedSigner { 
        name: "undefined_signer".to_string() 
    });
}
```

### Testing Parser Errors

```rust
#[test]
fn test_parse_error() {
    let invalid = r#"
        action "test" {  // Missing construct type
            foo = "bar"
        }
    "#;
    
    let result = parse_runbook(invalid);
    assert!(result.is_err());
}
```

## Troubleshooting

### Build Failures

If you see supervisor UI build errors:

```bash
# Use CLI-only commands
cargo test-cli
cargo build-cli
```

### Slow Tests

For faster iteration:

```bash
# Run only your specific test
cargo test test_my_function

# Skip integration tests
cargo test --lib
```

### Test Output

To see println! output:

```bash
cargo test -- --nocapture
```

## Best Practices

1. **Use Descriptive Names**: `test_undefined_variable_in_action_input` not `test1`
2. **Test One Thing**: Each test should verify a single behavior
3. **Use Builders**: Prefer `RunbookBuilder` over string manipulation
4. **Mock External Dependencies**: Don't rely on network/filesystem
5. **Keep Tests Fast**: Mock slow operations
6. **Use Snapshots**: For complex outputs that change frequently

## Next Steps

- Check existing tests in `crates/txtx-core/src/tests/` for examples
- See [`crates/txtx-test-utils/examples/enhanced_builder_example.rs`](/crates/txtx-test-utils/examples/enhanced_builder_example.rs) for comprehensive RunbookBuilder usage patterns including:
  - Basic runbook construction
  - Environment-aware runbooks with manifests
  - Multi-action workflows with dependencies
  - Cross-chain deployment scenarios
  - Validation modes comparison
  - Complex DeFi workflow examples
- Join #testing channel for help and discussions
