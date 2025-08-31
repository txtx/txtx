# EVM Test Infrastructure Documentation

## Overview
Comprehensive documentation of the test infrastructure for the txtx EVM addon, including test organization, fixtures, patterns, and best practices.

## Test Organization

### Directory Structure
```
addons/evm/
├── src/tests/
│   ├── integration/         # Tests requiring Anvil/real node
│   ├── project_test_harness.rs  # Main test framework
│   ├── error_handling_tests.rs  # Error scenario tests
│   └── codec_tests.rs       # Unit tests for encoding/decoding
├── fixtures/
│   ├── integration/         # Integration test fixtures
│   │   ├── transactions/    # Transaction-related fixtures
│   │   ├── deployments/     # Contract deployment fixtures
│   │   ├── abi/            # ABI interaction fixtures
│   │   ├── errors/         # Error scenario fixtures
│   │   └── unicode_storage.tx  # Unicode support fixture
│   └── parsing/            # Parse-only test fixtures
└── src/contracts/          # Solidity contracts for testing
```

## Test Categories

### 1. Unit Tests
- **Location**: `src/tests/codec_tests.rs`
- **Purpose**: Test individual components without external dependencies
- **Examples**: 
  - ABI encoding/decoding
  - Address validation
  - Hex conversion utilities

### 2. Integration Tests
- **Location**: `src/tests/integration/`
- **Purpose**: Test with real Ethereum node (Anvil)
- **Key Files**:
  - `deployment_tests.rs` - Contract deployment scenarios
  - `transaction_tests.rs` - Transaction execution
  - `view_function_tests.rs` - Read-only contract calls
  - `unicode_storage_tests.rs` - International character support
  - `insufficient_funds_tests.rs` - Error handling

### 3. Error Handling Tests
- **Location**: `src/tests/error_handling_tests.rs`
- **Purpose**: Verify proper error detection and reporting
- **Coverage**:
  - Insufficient funds errors
  - Invalid hex encoding
  - Missing signers
  - Contract function errors
  - RPC connection failures

## Test Fixtures

### Fixture Organization
Fixtures are `.tx` runbook files used by multiple tests:

#### Integration Fixtures
- **transactions/** - ETH transfers, token transfers, batch operations
- **deployments/** - Simple contracts, proxy patterns, factory patterns
- **abi/** - Function calls, event filtering, encoding tests
- **errors/** - Various error scenarios

#### Example Fixture
```hcl
# fixtures/integration/transactions/simple_transfer.tx
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

signer "sender" "evm::secret_key" {
    secret_key = input.sender_private_key
}

action "transfer" "evm::send_eth" {
    recipient_address = input.recipient
    amount = input.amount
    signer = signer.sender
    confirmations = 0
}

output "tx_hash" {
    value = action.transfer.tx_hash
}
```

### Fixture Best Practices
1. **Use input variables** for test parameterization
2. **Keep fixtures focused** on single scenarios
3. **Document expected outcomes** in comments
4. **Reuse fixtures** across multiple tests when possible

## Test Harness

### ProjectTestHarness
The main testing framework providing:

```rust
// Create test with Foundry compilation
let mut harness = ProjectTestHarness::new_foundry("test.tx", runbook_content)
    .with_anvil();  // Spawn Anvil instance

// Setup project environment
harness.setup().expect("Setup failed");

// Execute runbook
let result = harness.execute_runbook()
    .expect("Execution failed");

// Verify outputs
assert!(result.outputs.contains_key("tx_hash"));

// Cleanup
harness.cleanup();
```

### Key Features
- **Automatic Anvil management** - Spawns and manages local blockchain
- **Compilation support** - Foundry and Hardhat frameworks
- **Input injection** - Pass test parameters to runbooks
- **Output validation** - Access runbook outputs for assertions
- **Cleanup handling** - Automatic temporary directory cleanup

## Test Patterns

### 1. Fixture-Based Testing
```rust
#[test]
fn test_with_fixture() {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/integration/example.tx");
    
    let runbook_content = std::fs::read_to_string(&fixture_path)
        .expect("Failed to read fixture");
    
    let mut harness = ProjectTestHarness::new_foundry("test.tx", runbook_content)
        .with_anvil();
    // ... test implementation
}
```

### 2. Inline Runbook Testing
```rust
#[test]
fn test_with_inline_runbook() {
    let runbook = r#"
        addon "evm" {
            chain_id = input.chain_id
            rpc_api_url = input.rpc_url
        }
        # ... runbook content
    "#;
    
    let mut harness = ProjectTestHarness::new_foundry("test.tx", runbook.to_string())
        .with_anvil();
    // ... test implementation
}
```

### 3. Error Scenario Testing
```rust
#[test]
fn test_error_scenario() {
    // ... setup
    let result = harness.execute_runbook();
    
    assert!(result.is_err(), "Should fail with error");
    let error_msg = result.unwrap_err();
    assert!(error_msg.contains("expected error"), 
            "Error should contain expected message");
}
```

## Running Tests

### All Tests
```bash
cargo test --package txtx-addon-network-evm
```

### Specific Test Categories
```bash
# Unit tests only
cargo test --package txtx-addon-network-evm --lib

# Integration tests (requires Anvil)
cargo test --package txtx-addon-network-evm --test '*integration*'

# Error handling tests
cargo test --package txtx-addon-network-evm error_handling

# Unicode support tests
cargo test --package txtx-addon-network-evm unicode_storage
```

### With Output
```bash
cargo test --package txtx-addon-network-evm -- --nocapture
```

## Test Development Guidelines

### 1. Test Naming
- Use descriptive names: `test_<scenario>_<expected_outcome>`
- Group related tests in modules
- Prefix with test category: `test_deployment_`, `test_error_`, etc.

### 2. Assertions
- Always include meaningful assertion messages
- Test both success and failure paths
- Verify specific output values, not just success/failure

### 3. Anvil Dependency
- Always check `AnvilInstance::is_available()`
- Provide skip messages for missing dependencies
- Never mark Anvil tests as `#[ignore]`

### 4. Fixture Management
- Store fixtures in appropriate subdirectories
- Document fixture purpose and usage
- Consider consolidation for similar scenarios

## Common Issues and Solutions

### Issue: Tests fail with "Anvil not found"
**Solution**: Install Foundry
```bash
curl -L https://foundry.paradigm.xyz | bash
foundryup
```

### Issue: Compilation errors with Unicode
**Solution**: Ensure proper UTF-8 encoding in source files and use raw strings for Unicode content

### Issue: Flaky tests due to timing
**Solution**: Use proper confirmation waiting and avoid hard-coded delays

### Issue: Test isolation problems
**Solution**: Each test should use its own Anvil instance and cleanup properly

## Documentation Files

### Test-Related Documentation
1. **TEST_CREATION_GUIDE.md** - How to create new tests
2. **TEST_QUICK_REFERENCE.md** - Common test patterns and snippets
3. **TEST_MIGRATION_TRACKER.md** - Progress tracking for test migration
4. **FIXTURE_CONSOLIDATION_PLAN.md** - Strategy for fixture organization
5. **ERROR_FIXTURES.md** - Documentation of error test fixtures
6. **UNICODE_SUPPORT.md** - Unicode character handling documentation
7. **TEST_INFRASTRUCTURE.md** - This file

## Future Improvements

### Planned Enhancements
1. **Parallel test execution** - Run integration tests in parallel
2. **Gas usage tracking** - Add gas consumption assertions
3. **Performance benchmarks** - Measure and track performance
4. **Fuzz testing** - Add property-based testing for edge cases
5. **Cross-chain testing** - Test with multiple chain configurations

### Technical Debt
1. Complete migration of remaining inline runbooks
2. Consolidate duplicate test scenarios
3. Add more comprehensive error scenarios
4. Improve test execution speed

## Conclusion

The EVM addon test infrastructure provides comprehensive coverage through unit tests, integration tests, and error scenarios. The fixture-based approach enables maintainable and reusable test scenarios, while the ProjectTestHarness framework simplifies test creation and execution. Continuous improvements focus on coverage, performance, and developer experience.