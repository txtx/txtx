# EVM Integration Test Migration Specifications

## Overview
The EVM addon has 20 integration test files that currently use the deprecated `MigrationHelper` pattern. These tests need to be migrated to use the new `FixtureBuilder` system or rewritten using direct Anvil instance management.

## Test Categories and Requirements

### 1. ABI Encoding/Decoding Tests
**Files:** `abi_encoding_tests.rs`, `abi_decoding_tests.rs`

**Test Requirements:**
- [ ] Test encoding of basic types (uint256, address, bool, bytes32)
- [ ] Test encoding of arrays (dynamic and fixed-size)
- [ ] Test encoding of tuples and complex nested structures
- [ ] Test encoding with function signatures
- [ ] Test packed encoding for efficiency
- [ ] Test decoding of function return values
- [ ] Test decoding of event logs
- [ ] Test error handling for malformed ABI data

**Migration Strategy:**
- These tests primarily test pure functions, may not need full fixture setup
- Consider using unit tests instead of integration tests where appropriate
- For tests requiring contract interaction, use FixtureBuilder with minimal contracts

### 2. Transaction Management Tests
**Files:** `transaction_management_tests.rs`, `transaction_types_tests.rs`, `advanced_transaction_tests.rs`

**Test Requirements:**
- [ ] Test nonce management and auto-incrementing
- [ ] Test gas estimation for transfers
- [ ] Test gas estimation for contract deployments
- [ ] Test EIP-1559 transactions with dynamic fees
- [ ] Test legacy transaction format
- [ ] Test batch transaction processing
- [ ] Test transaction replacement (speed-up/cancel)
- [ ] Test different transaction types (Type 0, Type 2)
- [ ] Test transaction signing with different key types

**Migration Strategy:**
- Use FixtureBuilder with Anvil for real transaction testing
- Create helper methods for common transaction patterns
- Ensure proper snapshot/revert for test isolation

### 3. Contract Interaction Tests
**Files:** `contract_interaction_tests.rs`, `view_function_tests.rs`, `function_selector_tests.rs`

**Test Requirements:**
- [ ] Test contract deployment with constructor args
- [ ] Test calling state-changing functions
- [ ] Test calling view/pure functions
- [ ] Test multi-call patterns
- [ ] Test function selector generation
- [ ] Test overloaded function handling
- [ ] Test fallback and receive functions
- [ ] Test contract-to-contract calls

**Migration Strategy:**
- Use FixtureBuilder with test contracts
- Create reusable Solidity test contracts
- Test both Foundry and Hardhat compilation paths

### 4. Error Handling Tests
**Files:** `error_handling_tests.rs`, `comprehensive_error_tests.rs`, `insufficient_funds_tests.rs`

**Test Requirements:**
- [ ] Test revert reasons from contracts
- [ ] Test custom error types (Solidity 0.8+)
- [ ] Test out-of-gas scenarios
- [ ] Test insufficient funds errors
- [ ] Test invalid nonce errors
- [ ] Test network connectivity errors
- [ ] Test RPC error responses
- [ ] Test transaction failure recovery

**Migration Strategy:**
- Create contracts that deliberately fail
- Test error propagation through the stack
- Verify error messages are user-friendly

### 5. Event and Log Tests
**Files:** `event_log_tests.rs`

**Test Requirements:**
- [ ] Test event emission from contracts
- [ ] Test indexed vs non-indexed parameters
- [ ] Test multiple events in one transaction
- [ ] Test event filtering by topics
- [ ] Test log decoding with ABI
- [ ] Test anonymous events

**Migration Strategy:**
- Create contracts with various event types
- Use FixtureBuilder to deploy and interact
- Verify logs in transaction receipts

### 6. Gas and Cost Tests
**Files:** `gas_estimation_tests.rs`, `transaction_cost_tests.rs`

**Test Requirements:**
- [ ] Test accurate gas estimation for simple transfers
- [ ] Test gas estimation for complex contract calls
- [ ] Test transaction cost calculation (gas * price)
- [ ] Test EIP-1559 cost calculations
- [ ] Test gas limit vs gas used
- [ ] Test refund mechanisms

**Migration Strategy:**
- Compare estimated vs actual gas usage
- Test with different network conditions
- Verify cost calculations match receipts

### 7. Transaction Lifecycle Tests
**Files:** `transaction_signing_tests.rs`, `transaction_simulation_tests.rs`, `check_confirmations_tests.rs`

**Test Requirements:**
- [ ] Test transaction signing with different key formats
- [ ] Test transaction simulation before sending
- [ ] Test confirmation counting
- [ ] Test pending transaction handling
- [ ] Test transaction receipt retrieval
- [ ] Test block confirmation waiting

**Migration Strategy:**
- Use Anvil's mining capabilities for confirmations
- Test both instant and delayed mining modes
- Verify transaction pool behavior

### 8. Deployment Tests
**Files:** `comprehensive_deployment_tests.rs`, `migrated_deployment_tests.rs`

**Test Requirements:**
- [ ] Test basic contract deployment
- [ ] Test deployment with constructor arguments
- [ ] Test CREATE2 deterministic deployment
- [ ] Test deployment gas estimation
- [ ] Test deployment transaction receipt
- [ ] Test contract verification after deployment
- [ ] Test proxy contract patterns

**Migration Strategy:**
- Use various contract sizes and complexity
- Test deployment failure scenarios
- Verify deployed bytecode matches expectations

## Implementation Plan

### Phase 1: Infrastructure Setup
1. Create shared test utilities module
2. Implement test contract library
3. Create fixture templates for common patterns
4. Setup helper functions for assertions

### Phase 2: Core Function Tests
1. Migrate ABI encoding/decoding tests (pure functions)
2. Migrate view function tests (read-only)
3. Create comprehensive test contracts

### Phase 3: Transaction Tests
1. Migrate transaction management tests
2. Migrate signing and simulation tests
3. Implement gas estimation tests

### Phase 4: Contract Interaction Tests
1. Migrate deployment tests
2. Migrate contract interaction tests
3. Implement event log tests

### Phase 5: Error Handling Tests
1. Migrate error handling tests
2. Create failure scenario contracts
3. Implement recovery mechanisms

### Phase 6: Advanced Features
1. Migrate confirmation tests
2. Implement batch operation tests
3. Add performance benchmarks

## Test Patterns to Establish

### Pattern 1: Simple Fixture Execution
```rust
#[tokio::test]
async fn test_simple_operation() {
    let mut fixture = FixtureBuilder::new("test_name")
        .with_runbook("main", RUNBOOK_CONTENT)
        .build()
        .await
        .unwrap();
    
    fixture.execute_runbook("main").await.unwrap();
    
    let output = fixture.get_output("main", "result").unwrap();
    assert_eq!(output, expected_value);
}
```

### Pattern 2: Contract Interaction
```rust
#[tokio::test]
async fn test_contract_interaction() {
    let mut fixture = FixtureBuilder::new("test_name")
        .with_contract("TestContract", CONTRACT_SOURCE)
        .with_runbook("deploy", DEPLOY_RUNBOOK)
        .with_runbook("interact", INTERACT_RUNBOOK)
        .build()
        .await
        .unwrap();
    
    // Deploy contract
    fixture.execute_runbook("deploy").await.unwrap();
    let address = fixture.get_output("deploy", "contract_address").unwrap();
    
    // Interact with contract
    fixture.execute_runbook("interact").await.unwrap();
    let result = fixture.get_output("interact", "result").unwrap();
    assert_eq!(result, expected);
}
```

### Pattern 3: Error Testing
```rust
#[tokio::test]
async fn test_error_handling() {
    let mut fixture = FixtureBuilder::new("test_error")
        .with_runbook("failing", FAILING_RUNBOOK)
        .build()
        .await
        .unwrap();
    
    let result = fixture.execute_runbook("failing").await;
    assert!(result.is_err());
    
    let error = result.unwrap_err();
    assert!(error.to_string().contains("expected error message"));
}
```

## Success Criteria

1. All 20 test files compile without errors
2. Test coverage remains at or above current levels
3. Tests run faster due to singleton Anvil instance
4. No test flakiness from resource conflicts
5. Clear error messages when tests fail
6. Easy to add new tests following established patterns

## Notes

- Priority should be given to tests that verify critical functionality
- Some tests may be better as unit tests rather than integration tests
- Consider creating a test data module for reusable contracts and fixtures
- Document any tests that are intentionally not migrated with reasoning