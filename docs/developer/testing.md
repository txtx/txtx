# Txtx Testing Guide

This guide covers testing strategies and tools for txtx development, including unit tests, integration tests, and the test utilities framework.

## Test Organization

```console
txtx/
├── crates/
│   ├── txtx-core/           # Core functionality tests
│   │   └── src/
│   │       └── validation/  # Unit tests for validators
│   ├── txtx-cli/            # CLI and feature tests
│   │   ├── src/
│   │   │   └── cli/
│   │   │       ├── linter_impl/tests/  # Linter unit tests
│   │   │       └── lsp/tests/          # LSP unit tests
│   │   └── tests/           # Integration tests
│   │       ├── linter_tests_builder.rs
│   │       └── lsp_tests_builder.rs
│   └── txtx-test-utils/     # Testing utilities
│       ├── src/             # Test helpers and builders
│       └── tests/           # Tests for the test utilities
```

## Quick Start

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test suites
cargo test --package txtx-cli        # CLI tests only
cargo test --package txtx-core       # Core tests only
cargo test --package txtx-test-utils # Test utility tests

# Run with justfile shortcuts (recommended)
just cli-unit          # CLI unit tests
just cli-int           # CLI integration tests
just lint-unit         # Linter unit tests
just lint-int          # Linter integration tests
just lsp-unit          # LSP unit tests
just lsp-int           # LSP integration tests
```

### Cargo Test Aliases

We use a consistent naming pattern for test aliases: `test-[scope]-[type]-[target]`

**Pattern Components**:
- **scope**: The crate being tested (e.g., `cli`, `core`, `addon-kit`)
- **type**: Either `unit` or `int` (integration)
- **target**: Optional specific module or test file

**Unit Test Aliases**:

```bash
cargo test-cli-unit           # All unit tests in txtx-cli
cargo test-cli-unit-linter   # Only linter module unit tests
cargo test-cli-unit-lsp       # Only LSP module unit tests
cargo test-core-unit          # All unit tests in txtx-core
cargo test-addon-kit-unit     # All unit tests in txtx-addon-kit
```

**Integration Test Aliases**:

```bash
cargo test-cli-int            # All integration tests for txtx-cli
cargo test-cli-int-linter     # Original linter integration tests
cargo test-cli-int-linter-new # New linter tests using RunbookBuilder
cargo test-cli-int-lsp        # LSP integration tests
```

**Convenience Aliases**:

```bash
cargo test-cli                # All CLI tests (unit + integration)
cargo build-cli               # Build CLI without supervisor UI
cargo build-cli-release       # Release build without supervisor UI
```

**Note**: All CLI test aliases use `--no-default-features --features cli` to avoid building the supervisor UI, which significantly increases build time and requires specific build tools only available to maintainers.

### Measuring Test Coverage

```bash
# Generate HTML coverage report
just coverage

# Coverage for CI (JSON format)
just coverage-ci

# Coverage for specific test
just coverage-test <test_name>
```

## Test Utilities (txtx-test-utils)

The `txtx-test-utils` crate provides powerful testing tools for validation and execution testing.

### RunbookBuilder

A fluent API for constructing test runbooks:

```rust
use txtx_test_utils::{RunbookBuilder, assert_validation_error};

#[test]
fn test_undefined_signer() {
    let result = RunbookBuilder::new()
        .addon("evm", vec![("chain_id", "1")])
        .action("deploy", "evm::deploy_contract")
        .input("signer", "signer.undefined")  // Reference undefined signer
        .validate();

    assert_validation_error!(result, "undefined");
}
```

### When to Use RunbookBuilder vs Integration Tests

**Use RunbookBuilder** for:
- Unit testing HCL syntax validation
- Testing basic semantic errors (unknown namespaces, action types)
- Quick validation tests that focus on runbook structure
- Reducing boilerplate in test code

**Use Integration Tests** for:
- **Linter-specific validation**: Undefined signers, invalid field access, cross-references
- **Multi-file runbooks**: Testing file imports and includes
- **Command behavior**: Testing exact error messages, line numbers, JSON output
- **Flow validation**: Testing flow variables and flow-specific rules
- **Full validation pipeline**: When you need the complete linter analysis

**Example Decision**:

```rust
// ✅ Use RunbookBuilder for basic validation
#[test]
fn test_unknown_namespace() {
    let result = RunbookBuilder::new()
        .action("test", "invalid::action")
        .validate();
    assert_validation_error!(result, "Unknown addon namespace");
}

// ❌ Use integration test for linter-specific checks
#[test]
fn test_undefined_signer_reference() {
    // This needs the full linter command to catch the error
    let output = Command::new("txtx")
        .arg("lint")
        .arg("fixture.tx")
        .output()
        .unwrap();
    // Linter catches undefined signer refs that RunbookBuilder doesn't
}
```

**Note**: RunbookBuilder uses `txtx_core::validation::hcl_validator` which provides HCL parsing but not the full linter analysis.

### Validation Testing

Test different validation modes:

```rust
// Basic HCL validation
let result = builder.validate();

// Full manifest validation with environment
let result = builder
    .with_environment("production", vec![
        ("API_KEY", "test-key"),
        ("API_URL", "https://api.test.com"),
    ])
    .set_current_environment("production")
    .validate();

// Linter validation 
let result = builder.validate_with_linter(manifest, Some("production".to_string()));
```

### Test Assertions

Convenient assertion macros:

```rust
use txtx_test_utils::{assert_success, assert_validation_error};

// Assert validation passes
assert_success!(result);

// Assert specific error is present
assert_validation_error!(result, "undefined signer");

// Custom assertions
assert!(result.errors.iter().any(|e| e.message.contains("invalid")));
```

## Writing Unit Tests

### Testing Validators

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use txtx_core::validation::{hcl_validator, ValidationResult};

    #[test]
    fn test_validates_action_parameters() {
        let content = r#"
            action "send" "evm::send_eth" {
                invalid_param = "value"
            }
        "#;

        let mut result = ValidationResult::new();
        let _ = hcl_validator::validate_with_hcl_and_addons(
            content,
            &mut result,
            "test.tx",
            addon_specs,
        );

        assert!(!result.errors.is_empty());
        assert!(result.errors[0].message.contains("invalid_param"));
    }
}
```

### Testing LSP Handlers

```rust
#[cfg(test)]
mod tests {
    use lsp_types::{Position, TextDocumentIdentifier};

    #[tokio::test]
    async fn test_go_to_definition() {
        let workspace = setup_test_workspace();

        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier::new(url),
                position: Position::new(10, 15),
            },
            ..Default::default()
        };

        let result = handle_goto_definition(&workspace, params).await;
        assert!(result.is_some());
    }
}
```

## Writing Integration Tests

### Linter Integration Tests

Create in `tests/linter_tests_builder.rs`:

```rust
use txtx_test_utils::RunbookBuilder;
use std::process::Command;

#[test]
fn test_linter_cli_undefined_signer() {
    // Create test file
    let content = RunbookBuilder::new()
        .action("deploy", "evm::deploy_contract")
        .input("signer", "signer.undefined")
        .build_content();

    std::fs::write("test.tx", content).unwrap();

    // Run linter
    let output = Command::new("cargo")
        .args(&["run", "--", "lint", "test.tx"])
        .output()
        .unwrap();

    // Check output
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("undefined signer"));

    // Cleanup
    std::fs::remove_file("test.tx").unwrap();
}
```

### LSP Integration Tests

```rust
#[tokio::test]
async fn test_lsp_diagnostics_flow() {
    let (client, server) = setup_test_lsp().await;

    // Open document
    client.did_open(TextDocumentItem {
        uri: Url::from_file_path("test.tx").unwrap(),
        language_id: "txtx".to_string(),
        version: 1,
        text: "invalid content",
    }).await;

    // Wait for diagnostics
    let diagnostics = client.receive_diagnostics().await;
    assert!(!diagnostics.is_empty());
    assert_eq!(diagnostics[0].severity, Some(DiagnosticSeverity::ERROR));
}
```

## Testing Patterns

### 1. Table-Driven Tests

```rust
use test_case::test_case;

#[test_case("signer.undefined", "undefined signer" ; "undefined signer")]
#[test_case("action.missing.output", "invalid output" ; "invalid output")]
#[test_case("env.MISSING", "environment variable" ; "missing env var")]
fn test_validation_errors(reference: &str, expected_error: &str) {
    let result = RunbookBuilder::new()
        .variable("test", reference)
        .validate();

    assert_validation_error!(result, expected_error);
}
```

### 2. Fixture-Based Testing

```rust
fn test_fixtures() {
    let fixtures_dir = Path::new("fixtures");

    for entry in fs::read_dir(fixtures_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension() == Some(OsStr::new("tx")) {
            let content = fs::read_to_string(&path).unwrap();
            let result = validate_content(&content);

            // Check for expected results file
            let expected_path = path.with_extension("expected");
            if expected_path.exists() {
                let expected = fs::read_to_string(&expected_path).unwrap();
                assert_eq!(format!("{:?}", result), expected);
            }
        }
    }
}
```

### 3. Snapshot Testing

```rust
use insta::assert_snapshot;

#[test]
fn test_error_formatting() {
    let result = RunbookBuilder::new()
        .action("invalid", "unknown::action")
        .validate();

    // Snapshot the formatted error output
    assert_snapshot!(format_validation_errors(&result));
}
```

## Performance Testing

### Benchmarking Validation

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_validation(c: &mut Criterion) {
    let content = std::fs::read_to_string("large_runbook.tx").unwrap();

    c.bench_function("validate large runbook", |b| {
        b.iter(|| {
            validate_content(black_box(&content))
        });
    });
}

criterion_group!(benches, benchmark_validation);
criterion_main!(benches);
```

## Test Coverage

### Generating Coverage Reports

```bash
# Install cargo-llvm-cov
cargo install cargo-llvm-cov

# Generate coverage report
cargo llvm-cov --html

# Open report
open target/llvm-cov/html/index.html
```

### Coverage Guidelines

#### Coverage Targets

- **Critical modules**: 95%+ line coverage required
  - `visitor.rs`, `violation_collector.rs`, `helpers.rs`
  - `violation.rs`, `rule_helpers.rs`, `location_helpers.rs`
- **Core validation logic**: 80%+ coverage minimum
- **Test utilities**: Coverage not required

#### Coverage Philosophy

1. **Meaningful Tests Over Metrics**: Write tests that validate actual behavior and catch regressions, not just to hit coverage numbers
2. **Indirect Coverage Is Valid**: Modules tested through integration tests count toward coverage
3. **Don't Test Test Infrastructure**: Skip test helpers, mocks, and fixtures
4. **Focus on Business Logic**: Prioritize validation rules, transformations, and error handling

#### What Not to Test

- Generated code (derive macros, build.rs output)
- Simple getters/setters that cannot fail
- Test helper implementations
- Trivial `Default` implementations
- Constants and type aliases

#### Using Coverage Tools

The `just coverage` command generates an HTML report showing line and function coverage percentages using cargo-llvm-cov.

Example workflow:

```bash
# Generate HTML coverage report
just coverage

# Generate JSON coverage for CI
just coverage-ci

# Generate coverage for specific test
just coverage-test my_test_name
```

## Debugging Tests

### Using Print Debugging

```rust
#[test]
fn test_complex_validation() {
    let result = complex_validation();

    // Debug print the entire result
    dbg!(&result);

    // Pretty print specific fields
    eprintln!("Errors: {:#?}", result.errors);

    assert!(result.success);
}
```

### Using RUST_BACKTRACE

```bash
# Get full backtrace on test failure
RUST_BACKTRACE=1 cargo test failing_test

# Get full backtrace with line numbers
RUST_BACKTRACE=full cargo test failing_test
```

### Using Test Logging

```rust
use env_logger;

#[test]
fn test_with_logging() {
    // Initialize logger for tests
    let _ = env_logger::builder().is_test(true).try_init();

    log::debug!("Starting test");
    // Test code...
    log::info!("Test completed");
}
```

Run with:

```bash
RUST_LOG=debug cargo test test_with_logging -- --nocapture
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Test

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run tests
        run: |
          cargo test --all-features
          cargo test --package txtx-cli --no-default-features --features cli
      - name: Run linter tests
        run: cargo test-cli-linter
      - name: Run LSP tests
        run: cargo test-cli-lsp
```

## Best Practices

1. **Test Naming**: Use descriptive names that explain what's being tested

   ```rust
   test_undefined_signer_returns_error()  // Good
   test_1()  // Bad
   ```

2. **Test Independence**: Each test should be independent

   ```rust
   // Use fresh builders for each test
   let builder = RunbookBuilder::new();
   ```

3. **Test Data**: Use minimal, focused test data

   ```rust
   // Good: Only includes what's needed for the test
   .signer("test", "evm::private_key", vec![])

   // Bad: Includes unnecessary complexity
   .signer("test", "evm::private_key", vec![
       ("unnecessary_field1", "value1"),
       ("unnecessary_field2", "value2"),
   ])
   ```

4. **Assertions**: Be specific about what you're testing

   ```rust
   // Good: Specific assertion
   assert_validation_error!(result, "undefined signer 'deployer'");

   // Bad: Too general
   assert!(!result.success);
   ```

5. **Cleanup**: Always clean up test files and resources

   ```rust
   #[test]
   fn test_with_file() {
       let test_file = "test_output.tx";

       // Test code...

       // Cleanup
       let _ = std::fs::remove_file(test_file);
   }
   ```
