# EVM Test Catalog Analysis

## Executive Summary
- **Total Test Files**: 30+ files
- **Tests with Inline Runbooks**: 12 files (39 inline runbooks)
- **Existing Fixtures**: 23 fixtures
- **Redundancy Level**: HIGH - Many tests duplicate similar scenarios

## Test Categories and Redundancies

### 1. ETH Transfer Tests (HIGH REDUNDANCY)

#### Existing Fixtures:
- `simple_eth_transfer.tx` - Basic transfer with balance checks
- `custom_gas_transfer.tx` - Transfer with custom gas settings
- `legacy_transaction.tx` - Legacy transaction type
- `batch_transactions.tx` - Multiple transfers
- `insufficient_funds_transfer.tx` - Error case

#### Tests Using These Patterns:
- `txtx_eth_transfer_tests.rs` - Can use `simple_eth_transfer.tx`
- `debug_eth_transfer_tests.rs` - Can use `simple_eth_transfer.tx`
- `migrated_transaction_tests.rs` - Multiple tests can use existing fixtures
- `runbook_execution_tests::test_runbook_format_for_send_eth` - Inline, redundant
- `txtx_runbook_tests::test_evm_send_eth_runbook_parses` - Inline, redundant

**Consolidation Opportunity**: 5+ tests can share 3-4 fixtures

### 2. Contract Deployment Tests (HIGH REDUNDANCY)

#### Existing Fixtures:
- `minimal_contract.tx` - Simple deployment
- `constructor_args.tx` - Deployment with constructor
- `deploy_and_interact.tx` - Deploy + call pattern
- `onchain_deployment.tx` - CREATE2 deployment

#### Tests with Inline Runbooks:
- `migrated_deployment_tests.rs` - 7 tests, most have inline runbooks that duplicate fixtures
  - `test_minimal_contract_deployment_txtx` - Duplicates `minimal_contract.tx`
  - `test_constructor_args_deployment_txtx` - Duplicates `constructor_args.tx`
  - `test_complex_constructor_deployment_txtx` - Inline, could use `constructor_args.tx`
  - `test_storage_contract_deployment_txtx` - Inline, similar to existing
  - `test_factory_pattern_deployment_txtx` - Could use fixture
  - `test_deployment_with_interaction_txtx` - Duplicates `deploy_and_interact.tx`

- `project_harness_integration_tests.rs` - 8 tests with inline runbooks
  - `test_foundry_contract_deployment` - Inline, duplicates deployment pattern
  - `test_hardhat_contract_deployment` - Inline, duplicates deployment pattern

**Consolidation Opportunity**: 10+ deployment tests can use 4 fixtures

### 3. ABI/Codec Tests (MODERATE REDUNDANCY)

#### Existing Fixtures:
- `complex_types.tx` - Complex ABI types

#### Tests with Inline Runbooks:
- `migrated_abi_tests.rs` - 2 tests
  - `test_complex_abi_encoding` - Inline, partially overlaps with `complex_types.tx`
  - `test_abi_edge_cases` - Inline, unique edge cases

- `codec_integration_tests.rs` - 7 tests
  - Multiple inline runbooks for primitive types, structs, arrays
  - Could consolidate into 2-3 fixtures

**Consolidation Opportunity**: 9 tests can use 3-4 fixtures

### 4. Error Handling Tests (MODERATE REDUNDANCY)

#### Existing Fixtures:
- `insufficient_funds_transfer.tx` - Insufficient funds
- `insufficient_gas.tx` - Gas errors
- `invalid_function_call.tx` - Function not found
- `invalid_hex_address.tx` - Invalid hex
- `missing_signer.tx` - Missing signer

#### Tests with Inline Runbooks:
- `insufficient_funds_tests.rs` - Has duplicate test (`test_insufficient_funds_for_gas` appears twice!)
- `migrated_error_tests.rs` - 8 tests
  - Most inline runbooks duplicate existing error fixtures
  - `test_insufficient_funds_error` - Duplicates `insufficient_funds_transfer.tx`
  - `test_function_not_found_error` - Duplicates `invalid_function_call.tx`
  - `test_invalid_hex_codec_error` - Duplicates `invalid_hex_address.tx`
  - `test_signer_key_not_found_error` - Duplicates `missing_signer.tx`

**Consolidation Opportunity**: 8+ error tests can use existing 5 fixtures

### 5. View/Pure Function Tests (LOW REDUNDANCY)

#### Existing Fixtures:
- `test_view_function.tx` - View function detection
- `state_changing_function.tx` - State-changing detection

#### Tests:
- `view_function_tests.rs` - Well organized, uses fixtures appropriately

**Status**: ✅ Already well-organized

### 6. Unicode Tests (LOW REDUNDANCY)

#### Existing Fixtures:
- `unicode_storage.tx` - Various Unicode characters
- `unicode_edge_cases.tx` - Edge cases

#### Tests:
- `unicode_storage_tests.rs` - Recently updated to use fixtures

**Status**: ✅ Already well-organized

## Redundancy Analysis

### Duplicate Test Names
- **CRITICAL**: `test_insufficient_funds_for_gas` appears twice in `insufficient_funds_tests.rs`!

### Most Redundant Patterns
1. **Simple ETH Transfer**: Appears in 5+ different test files
2. **Basic Contract Deployment**: Appears in 7+ test files
3. **Insufficient Funds Error**: Tested in 3+ places
4. **ABI Encoding of Primitives**: Multiple similar tests

### Tests That Can Be Deleted/Merged
1. Parse-only tests in `txtx_runbook_tests.rs` - Redundant with actual execution tests
2. Duplicate error tests in `migrated_error_tests.rs` - Use existing error fixtures
3. Simple deployment tests in `migrated_deployment_tests.rs` - Use existing deployment fixtures

## Consolidation Plan

### Phase 1: Remove Duplicates (Immediate)
1. Fix duplicate `test_insufficient_funds_for_gas` function
2. Remove parse-only tests that duplicate execution tests
3. Update tests to use existing fixtures where exact matches exist

### Phase 2: Extract Unique Patterns (Priority)
Extract inline runbooks that represent unique patterns not covered by existing fixtures:
1. Factory pattern deployment
2. Complex constructor with multiple types
3. Specific codec edge cases (overflow, underflow)
4. Multi-action transaction sequences
5. Custom error scenarios not yet covered

### Phase 3: Create Consolidated Fixtures (Optimization)
Create parameterized fixtures that can handle variations:
1. `deployment_patterns.tx` - Handles simple, constructor, factory patterns
2. `transfer_patterns.tx` - Handles simple, custom gas, legacy, batch
3. `codec_patterns.tx` - Handles all primitive and complex type encoding
4. `error_patterns.tx` - Comprehensive error scenarios

## Fixture Mapping

### Tests → Existing Fixtures Mapping

| Test File | Test Function | Should Use Fixture | Action |
|-----------|--------------|-------------------|---------|
| `migrated_deployment_tests.rs` | `test_minimal_contract_deployment_txtx` | `minimal_contract.tx` | Update |
| `migrated_deployment_tests.rs` | `test_constructor_args_deployment_txtx` | `constructor_args.tx` | Update |
| `migrated_deployment_tests.rs` | `test_deployment_with_interaction_txtx` | `deploy_and_interact.tx` | Update |
| `migrated_error_tests.rs` | `test_insufficient_funds_error` | `insufficient_funds_transfer.tx` | Update |
| `migrated_error_tests.rs` | `test_function_not_found_error` | `invalid_function_call.tx` | Update |
| `migrated_error_tests.rs` | `test_invalid_hex_codec_error` | `invalid_hex_address.tx` | Update |
| `migrated_error_tests.rs` | `test_signer_key_not_found_error` | `missing_signer.tx` | Update |
| `txtx_runbook_tests.rs` | `test_evm_send_eth_runbook_parses` | `basic_send_eth.tx` | Update |
| `txtx_runbook_tests.rs` | `test_evm_deploy_contract_runbook_parses` | `basic_deploy.tx` | Update |
| `runbook_execution_tests.rs` | `test_runbook_format_for_send_eth` | `simple_eth_transfer.tx` | Update |

### Inline Runbooks Requiring New Fixtures

| Test | Pattern | Proposed Fixture |
|------|---------|-----------------|
| `test_complex_constructor_deployment_txtx` | Multiple constructor args | `complex_constructor.tx` |
| `test_factory_pattern_deployment_txtx` | Factory deployment | `factory_pattern.tx` |
| `test_encode_struct` | Struct encoding | `codec_struct.tx` |
| `test_multi_action_runbook` | Action dependencies | `multi_action_sequence.tx` |

## Statistics

### Current State
- **Total Tests**: 111
- **Tests with Inline Runbooks**: ~40
- **Tests Using Fixtures**: 13 (11%)
- **Redundant Tests**: ~25-30 (22-27%)

### After Consolidation
- **Expected Total Tests**: ~85-90 (after removing duplicates)
- **Expected Fixture Count**: 25-30 (slight increase)
- **Expected Fixture Reuse**: 3-4 tests per fixture average
- **Expected Migration**: 60-70% using fixtures

## Action Items

### Immediate (Fix Bugs)
1. ✅ Remove duplicate `test_insufficient_funds_for_gas` function
2. ✅ Fix test counts in migration tracker

### High Priority (Remove Redundancy)
1. Update `migrated_deployment_tests.rs` to use existing deployment fixtures
2. Update `migrated_error_tests.rs` to use existing error fixtures
3. Remove parse-only tests from `txtx_runbook_tests.rs`

### Medium Priority (Extract Unique)
1. Extract factory pattern to fixture
2. Extract complex constructor patterns
3. Extract multi-action sequences
4. Create consolidated codec fixtures

### Low Priority (Optimize)
1. Create parameterized fixtures for variations
2. Update documentation
3. Clean up unused code

## Conclusion

The test suite has significant redundancy, with many tests duplicating the same scenarios. By consolidating to use existing fixtures and removing duplicates, we can:
1. Reduce test count by ~20-25%
2. Improve maintainability
3. Ensure consistent testing patterns
4. Make tests more readable and focused

The highest impact changes are:
1. Fixing the duplicate function bug
2. Updating deployment and error tests to use existing fixtures
3. Removing parse-only tests that duplicate execution tests