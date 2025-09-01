# Test File Naming Convention

## Standard Naming Rules

All test files in the EVM addon follow these naming conventions:

### 1. Test Files
Files containing `#[test]` functions should use the `_tests.rs` suffix (plural):
- ✅ `codec_tests.rs`
- ✅ `transaction_tests.rs`
- ✅ `deployment_tests.rs`
- ❌ `codec_test.rs` (avoid singular)
- ❌ `codec.rs` (needs suffix)

### 2. Test Utilities and Harnesses
Helper modules that don't contain tests themselves should have NO suffix:
- ✅ `anvil_harness.rs` - Provides Anvil instance management
- ✅ `project_test_harness.rs` - Provides test project setup
- ✅ `runbook_test_utils.rs` - Utility functions for tests
- ❌ `anvil_test_harness.rs` (redundant "test" in name)

### 3. Module Files
- `mod.rs` - Standard Rust module files, no suffix needed

## Migration Status Naming

During the test migration from Alloy to txtx framework, avoid temporary naming:
- ❌ `migrated_transaction_tests.rs` - Remove "migrated" prefix after stabilization
- ✅ `transaction_tests.rs` - Final name after migration

## Directory Structure

```
src/tests/
├── mod.rs                          # Module definition
├── anvil_test_harness.rs          # Test harness (no suffix)
├── project_test_harness.rs        # Test harness (no suffix)
├── runbook_test_utils.rs          # Utilities (no suffix)
├── codec_tests.rs                 # Test file (plural suffix)
├── error_handling_tests.rs        # Test file (plural suffix)
├── integration/
│   ├── mod.rs                     # Module definition
│   ├── anvil_harness.rs          # Harness (no suffix)
│   ├── deployment_tests.rs       # Test file (plural suffix)
│   └── transaction_tests.rs      # Test file (plural suffix)
```

## Rationale

1. **Consistency**: All test files use the same `_tests.rs` pattern
2. **Clarity**: Easy to distinguish test files from utilities
3. **Rust Convention**: Follows common Rust project patterns
4. **Searchability**: Can easily find all tests with `*_tests.rs`

## Implementation

All test files were standardized to this convention in commit [commit-hash].
When adding new test files, please follow these conventions.