# Fixture Consolidation Plan

## Analysis Results

After examining the inline runbooks, we can consolidate fixtures to reduce duplication and improve maintainability.

## Consolidation Opportunities

### 1. Parse-Only Tests (txtx_runbook_tests.rs)
These 5 tests only verify that runbooks parse correctly, they don't execute them.
- **Current**: 5 separate inline runbooks
- **Proposed**: Create 3 simple fixtures in `fixtures/parsing/`:
  - `basic_send_eth.tx` - Minimal send_eth for parsing tests
  - `basic_deploy.tx` - Minimal deployment for parsing tests  
  - `basic_call.tx` - Minimal contract call for parsing tests

### 2. ETH Transfer Tests
- **Existing fixtures can be reused**:
  - `simple_eth_transfer.tx` - Can be used by multiple transfer tests
  - `custom_gas_transfer.tx` - For gas customization tests
  - `legacy_transaction.tx` - For legacy tx type tests
- **No new fixtures needed**

### 3. Contract Deployment Tests
- **Existing fixtures can be reused**:
  - `minimal_contract.tx` - Basic deployment (3 tests can use this)
  - `constructor_args.tx` - Deployment with constructor (2 tests can use this)
  - `deploy_and_interact.tx` - Full deploy + interact flow
- **No new fixtures needed**

### 4. Contract Interaction Tests
- **Existing fixtures can be reused**:
  - `deploy_and_interact.tx` - Deploy and call pattern
  - `state_changing_function.tx` - View vs state-changing differentiation
- **May need**:
  - `complex_abi_calls.tx` - For ABI encoding edge cases

## Reusability Matrix

| Fixture | Can Be Used By Tests |
|---------|---------------------|
| `simple_eth_transfer.tx` | test_simple_eth_transfer, test_evm_send_eth_runbook_parses* |
| `minimal_contract.tx` | test_deploy_minimal_contract, test_evm_deploy_contract_runbook_parses* |
| `deploy_and_interact.tx` | test_deploy_and_interact, test_evm_call_contract_runbook_parses* |
| `constructor_args.tx` | test_deploy_with_constructor_args, test_complex_deployment |

*With minor modifications or input parameters

## Benefits of Consolidation

1. **Reduced Duplication**: ~11 inline runbooks → ~3-4 new fixtures (rest reuse existing)
2. **Single Source of Truth**: Changes to contract deployment pattern update all tests
3. **Better Test Coverage**: Same fixture tested in multiple contexts
4. **Easier Maintenance**: Fewer files to maintain
5. **Documentation**: Each fixture becomes a canonical example

## Implementation Strategy

### Phase 1: Create Parsing-Specific Fixtures
Create minimal fixtures specifically for parse-only tests:
```
fixtures/
├── parsing/           # New: Minimal fixtures for parse tests
│   ├── send_eth.tx
│   ├── deploy.tx
│   └── call.tx
```

### Phase 2: Update Tests to Reuse Fixtures
Modify tests to use existing fixtures with input parameters:
```rust
// Instead of inline runbook
let harness = ProjectTestHarness::from_fixture(&fixture_path)
    .with_input("contract_address", "0x...")
    .with_input("function_name", "retrieve");
```

### Phase 3: Create Specialized Fixtures Only When Needed
Only create new fixtures for truly unique test cases:
- Complex ABI encoding scenarios
- Error edge cases
- Special protocol interactions

## Example: Reusing simple_eth_transfer.tx

```rust
// Test 1: Basic transfer test
let harness = ProjectTestHarness::from_fixture("simple_eth_transfer.tx")
    .with_anvil();

// Test 2: Parse-only test (no Anvil)
let harness = ProjectTestHarness::from_fixture("simple_eth_transfer.tx");
// Just verify it parses, don't execute

// Test 3: Transfer with custom recipient
let harness = ProjectTestHarness::from_fixture("simple_eth_transfer.tx")
    .with_anvil()
    .with_input("recipient", "0xCustomAddress...");
```

## Metrics

- **Current**: 21 inline runbooks + 13 fixture files = 34 total
- **After Consolidation**: ~16-18 fixture files (50% reduction)
- **Reuse Factor**: Each fixture used by 2-3 tests average

## Next Steps

1. Identify which inline runbooks are truly unique vs variations
2. Create the parsing-specific fixtures directory
3. Update tests to use parameterized fixtures
4. Document which fixtures are canonical examples
5. Remove redundant inline runbooks