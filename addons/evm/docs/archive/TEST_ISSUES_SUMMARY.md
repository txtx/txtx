# EVM Test Suite Issues Summary

## Overview
After investigating the test suite, here are the main issues causing test failures:

## 1. Missing Actions/Commands
Many test fixtures reference actions that don't exist in the EVM addon:
- `evm::decode_abi` - Used in abi_decoding_tests but doesn't exist
- `evm::call_contract_function` - Referenced in some tests but doesn't exist
- `evm::encode_abi` - May be missing or incorrectly named

**Existing actions are:**
- `evm::send_eth`
- `evm::check_confirmations`
- `evm::sign_transaction`
- `evm::eth_call`
- `evm::deploy_contract`
- `evm::call_contract`

## 2. Test Harness Issues

### Missing Output Collection
The test harness in `test_harness/mod.rs` doesn't collect outputs from runbook execution:
```rust
// TODO: Collect outputs once we understand the correct structure
let outputs = HashMap::new();
```

### Input Mapping Issues
Tests expect different input names than what the harness provides:
- Harness provides: `sender_private_key`, `recipient_address`
- Fixtures expect: `private_key`, `recipient`, `deployer_private_key`

## 3. Fixture Issues

### Missing Fixtures
- `simple_send_eth_with_env.tx` - Created but needs validation
- `constructor_validation.tx` - Referenced but doesn't exist
- Many fixtures in `fixtures/integration/` reference non-existent actions

### Incorrect Fixture Content
- Fixtures using `signer "x" "evm::private_key"` instead of `"evm::secret_key"`
- Fixtures expecting actions that don't exist

## 4. Test Categories

### Passing Tests (67 total)
- All `codec::tests` - Unit tests for encoding/decoding
- `tests::error_preservation_tests` - Our newly added tests

### Failing Tests (Most integration tests)
- **Configuration errors**: Missing inputs, wrong signer types
- **Missing actions**: Tests trying to use non-existent commands
- **Output validation**: Tests expecting outputs that aren't collected

### Potentially Hanging Tests
- None identified - tests fail quickly with clear error messages
- Anvil spawning works correctly

## 5. Root Causes

1. **Incomplete Implementation**: Many test fixtures were written for features that haven't been implemented yet
2. **Test Harness Limitations**: The harness doesn't properly collect outputs or handle all input scenarios
3. **Naming Mismatches**: Inconsistent naming between test expectations and actual implementation

## Recommendations

### Immediate Fixes
1. Fix input name mapping in test harness
2. Implement output collection in test harness
3. Update fixtures to use correct signer type (`evm::secret_key`)
4. Add commonly used input defaults (amount, gas, etc.)

### Medium-term Fixes
1. Either implement missing actions or remove/update tests that use them
2. Standardize input naming conventions across all tests
3. Create a test fixture validation tool

### Long-term Fixes
1. Complete implementation of ABI encoding/decoding actions
2. Improve test harness to better simulate full txtx execution
3. Add integration test documentation

## Test Statistics
- **Total tests**: ~260
- **Passing**: ~70 (mostly unit tests)
- **Failing**: ~190 (mostly integration tests)
- **Root issue**: Missing functionality and test harness limitations