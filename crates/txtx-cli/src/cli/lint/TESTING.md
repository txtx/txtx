# Linter Testing Guidelines

This document outlines testing best practices for the txtx linter.

## Table of Contents

1. [Test Organization](#test-organization)
2. [Test Patterns](#test-patterns)
3. [Assertion Helpers](#assertion-helpers)
4. [Testing Checklist](#testing-checklist)
5. [Property-Based Testing](#property-based-testing)

## Test Organization

### Unit Tests

Unit tests live in the same file as the code they test, within `#[cfg(test)]` modules:

```rust
// In validator.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linter_new_with_valid_config() {
        // Arrange
        let config = LinterConfig::new(...);

        // Act
        let result = Linter::new(&config);

        // Assert
        assert!(result.is_ok());
    }
}
```

### Integration Tests

Integration tests live in `tests/linter_tests_builder.rs` and test the full linter pipeline:

```rust
#[test]
fn test_lint_with_circular_dependency() {
    let result = RunbookBuilder::new()
        .with_content(content)
        .validate();

    assert_circular_dependency!(result);
}
```

## Test Patterns

### AAA Pattern (Arrange-Act-Assert)

All tests must follow the AAA pattern with clear comments:

```rust
#[test]
fn test_cli_override_warns_when_overriding_global_env() {
    // Arrange - manifest defines API_KEY in global environment
    let manifest = create_test_manifest_with_env(vec![
        ("global", vec![("API_KEY", "global-value")]),
    ]);

    // Act - provide same input via CLI
    let result = linter.validate_content(content, "test.tx", manifest, None);

    // Assert - should warn about CLI override
    assert_violation_count!(result, "cli_input_override", 1);
}
```

### Naming Conventions

- Test names should be descriptive: `test_<what>_<condition>_<expected_result>`
- Use full words, not abbreviations
- Group related tests in modules

**Good examples:**
- `test_cli_override_warns_when_overriding_global_env`
- `test_config_file_not_found_returns_none`
- `test_error_message_includes_variable_name`

**Bad examples:**
- `test_override` (too vague)
- `test_cfg_err` (abbreviations unclear)
- `test1`, `test2` (non-descriptive)

## Assertion Helpers

Use the provided assertion macros for cleaner, more maintainable tests.

### Basic Assertions

```rust
// Check validation passes
assert_validation_passes!(result);

// Check for specific error
assert_validation_error!(result, "UNDEFINED_VAR");

// Check for error with specific code
assert_has_error_code!(result, "undefined_input");

// Check for warning with specific code
assert_has_warning_code!(result, "input_naming_convention");
```

### Advanced Assertions

```rust
// Assert no violations with given code
assert_no_violations!(result, "cli_input_override");

// Assert exact count
assert_violation_count!(result, "input_naming_convention", 2);

// Assert violation at specific location
assert_violation_at!(result, "undefined_input", 42);
assert_violation_at!(result, "undefined_input", 42, 15); // line, column

// Assert message contains text
assert_violation_message_contains!(result, "input_naming_convention", "hyphens");
```

### When NOT to Use Assertion Helpers

Don't use assertion helpers for:
- Testing library behavior (strum derives, serialization) - these are **tautology tests**
- Simple boolean checks where `assert!()` is clearer
- One-off assertions that won't be reused

## Testing Checklist

When adding a new validation rule, ensure you have:

- [ ] **Unit test for the rule function** - Test the rule in isolation
- [ ] **Integration test with valid input** - Verify the rule doesn't fire false positives
- [ ] **Integration test with invalid input** - Verify the rule detects violations
- [ ] **Edge case tests** - Test boundary conditions
- [ ] **Error message quality test** - Verify messages are clear and actionable
- [ ] **Configuration test** - Test that the rule respects configuration settings

### Example: Adding a New Rule

```rust
// 1. Rule function (in rules.rs)
fn validate_my_new_rule(ctx: &ValidationContext) -> Option<ValidationIssue> {
    // Implementation
}

// 2. Unit test for rule logic
#[test]
fn test_my_new_rule_detects_issue() {
    // Arrange
    let ctx = create_test_context(...);

    // Act
    let result = validate_my_new_rule(&ctx);

    // Assert
    assert!(result.is_some());
    assert_eq!(result.unwrap().rule, CliRuleId::MyNewRule);
}

// 3. Integration test (in linter_tests_builder.rs)
#[test]
fn test_lint_with_my_new_rule_violation() {
    // Arrange
    let content = r#"..."#;

    // Act
    let result = RunbookBuilder::new()
        .with_content(content)
        .validate();

    // Assert
    assert_has_warning_code!(result, "my_new_rule");
}

// 4. Error message quality test (in validator.rs)
#[test]
fn test_my_new_rule_message_is_actionable() {
    // Test that error message includes helpful context
}
```

## Property-Based Testing

Use property-based tests (proptest) to verify invariants across many generated test cases:

```rust
proptest! {
    /// Property: Valid snake_case names should never trigger warnings
    #[test]
    fn valid_snake_case_always_passes(name in valid_snake_case_name()) {
        let ctx = create_test_context(name.clone());
        let result = validate_naming_convention(&ctx);
        prop_assert!(result.is_none());
    }
}
```

### When to Use Property Tests

- **Invariant testing** - Properties that should always hold
- **Round-trip testing** - Serialization/deserialization
- **Fuzz testing** - Find edge cases with random inputs
- **Relationship testing** - e.g., suggested fixes actually fix issues

### When NOT to Use Property Tests

- Specific business logic with known test cases
- Tests that require complex setup
- One-off scenarios that don't represent a general property

## Anti-Patterns to Avoid

### ❌ Tautology Tests

Don't test library behavior:

```rust
// BAD - Testing strum derive
#[test]
fn test_severity_to_string() {
    assert_eq!(Severity::Error.to_string(), "error");
}
```

This tests the `strum` library, not our code.

### ❌ Weak Assertions

Don't use count-based assertions when you can check specific errors:

```rust
// BAD
assert!(result.errors.len() >= 2);

// GOOD
assert_has_error_code!(result, "undefined_input");
assert_has_error_code!(result, "invalid_parameter");
```

### ❌ Testing Implementation Details

Test behavior, not implementation:

```rust
// BAD - Testing internal structure
assert_eq!(linter.config.cli_inputs.len(), 2);

// GOOD - Testing observable behavior
assert_violation_count!(result, "cli_input_override", 2);
```

### ❌ Missing AAA Comments

Always include Arrange-Act-Assert comments:

```rust
// BAD
#[test]
fn test_something() {
    let x = create_thing();
    let result = do_action(x);
    assert_eq!(result, expected);
}

// GOOD
#[test]
fn test_something() {
    // Arrange
    let x = create_thing();

    // Act
    let result = do_action(x);

    // Assert
    assert_eq!(result, expected);
}
```

## Code Coverage

Target minimum coverage levels:
- **Critical paths**: 100% (validation rules, error handling)
- **Business logic**: 90%+
- **Utility code**: 80%+

Run coverage with:
```bash
cargo llvm-cov test --package txtx-cli --bin txtx --no-default-features --features cli --html
```

## Test Performance

- Keep unit tests fast (< 10ms each)
- Use `#[ignore]` for slow integration tests
- Run property tests with reasonable case counts (100-1000)

## Review Checklist

Before merging tests:
- [ ] All tests follow AAA pattern
- [ ] Test names are descriptive
- [ ] No tautology tests
- [ ] Assertions use helper macros where applicable
- [ ] Error messages are tested for quality
- [ ] Property tests verify meaningful invariants
- [ ] Tests are fast and don't block CI
