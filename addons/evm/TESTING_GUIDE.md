# EVM Addon Testing Guide

This guide covers the testing infrastructure for the txtx EVM addon, including the fixture-based testing system, error handling patterns, and best practices.

## Table of Contents

1. [Fixture-Based Testing](#fixture-based-testing)
2. [Error Handling Tests](#error-handling-tests)
3. [Integration Tests](#integration-tests)
4. [Running Tests](#running-tests)
5. [Writing New Tests](#writing-new-tests)
6. [Common Patterns](#common-patterns)

## Fixture-Based Testing

The fixture builder provides isolated test environments for EVM runbooks with automatic setup and teardown.

### Quick Start

```rust
use crate::tests::fixture_builder::*;

#[tokio::test]
async fn test_my_feature() {
    // Create a test fixture
    let mut fixture = FixtureBuilder::new("test_name")
        .with_environment("testing")
        .build()
        .await
        .unwrap();
    
    // Add a runbook
    fixture.add_runbook("my_runbook", RUNBOOK_CONTENT).unwrap();
    
    // Execute it
    fixture.execute_runbook("my_runbook").await.unwrap();
    
    // Check outputs
    let outputs = fixture.get_outputs("my_runbook").unwrap();
    assert!(outputs.contains_key("action_result"));
}
```

### Key Features

- **Isolated Anvil instances** with snapshot/revert
- **26 named test accounts** (alice through zed)
- **Automatic output generation** for all actions
- **Built from source** - always tests current code
- **HCL parsing** via txtx-core

### Available Helpers

```rust
use crate::tests::fixture_builder::helpers::*;

// Extract values from outputs
let tx_hash = get_string_output(&outputs, "transfer_result", "tx_hash");
let success = get_bool_output(&outputs, "transfer_result", "success");

// Assert action success
assert_action_success(&outputs, "transfer");

// Assert transaction has valid hash
let hash = assert_has_tx_hash(&outputs, "transfer");

// Assert deployment has contract address
let address = assert_has_contract_address(&outputs, "deploy");
```

## Error Handling Tests

The EVM addon uses error-stack for comprehensive error handling with context preservation.

### Testing Error Cases

```rust
#[test]
fn test_invalid_address() {
    let result = string_to_address("invalid");
    assert!(result.is_err());
    
    let report = result.unwrap_err();
    assert!(report.contains::<EvmError>());
}
```

### Error Context Patterns

```rust
// Add context to errors
result.attach_printable("Processing transaction")
      .attach(TransactionContext { hash, from, to })
      .change_context(EvmError::Transaction)?;

// Test with context
let ctx = TransactionContext::new(hash, from, to);
let result = process_with_context(ctx);
assert!(result.unwrap_err().contains::<TransactionContext>());
```

## Integration Tests

### Contract Deployment

```rust
#[tokio::test]
async fn test_contract_deployment() {
    let mut fixture = FixtureBuilder::new("deploy_test").build().await.unwrap();
    
    // Add contract
    fixture.add_contract("SimpleStorage", contracts::SIMPLE_STORAGE).unwrap();
    
    // Deploy runbook
    let runbook = templates::deploy_contract("SimpleStorage", "alice");
    fixture.add_runbook("deploy", &runbook).unwrap();
    
    // Execute and verify
    fixture.execute_runbook("deploy").await.unwrap();
    let outputs = fixture.get_outputs("deploy").unwrap();
    
    let address = assert_has_contract_address(&outputs, "deploy");
    println!("Contract deployed at: {}", address);
}
```

### Transaction Testing

```rust
#[tokio::test]
async fn test_eth_transfer() {
    let mut fixture = FixtureBuilder::new("transfer_test").build().await.unwrap();
    
    let runbook = templates::eth_transfer("alice", "bob", "1000000000000000000");
    fixture.add_runbook("transfer", &runbook).unwrap();
    
    fixture.execute_runbook("transfer").await.unwrap();
    assert_action_success(&fixture.get_outputs("transfer").unwrap(), "transfer");
}
```

## Running Tests

### All Tests
```bash
cargo test --package txtx-addon-network-evm
```

### Specific Test Categories
```bash
# Fixture builder tests
cargo test --package txtx-addon-network-evm fixture_builder

# Error handling tests
cargo test --package txtx-addon-network-evm error_handling

# Integration tests (requires txtx built)
cargo test --package txtx-addon-network-evm integration -- --ignored
```

### With Output
```bash
cargo test --package txtx-addon-network-evm -- --nocapture
```

## Writing New Tests

### 1. Choose the Right Test Type

- **Unit tests**: For isolated function testing
- **Fixture tests**: For runbook execution testing
- **Integration tests**: For end-to-end scenarios

### 2. Use the Fixture Builder

```rust
#[tokio::test]
async fn test_new_feature() {
    let fixture = FixtureBuilder::new("test_new_feature")
        .with_environment("testing")
        .with_confirmations(1)
        .build()
        .await
        .unwrap();
    
    // Your test logic here
}
```

### 3. Follow Naming Conventions

- Test functions: `test_<feature>_<scenario>`
- Fixtures: `test_<feature>`
- Runbooks: Descriptive action names

### 4. Use Helpers

```rust
use crate::tests::fixture_builder::helpers::*;

// Use predefined contracts
fixture.add_contract("Counter", contracts::COUNTER).unwrap();

// Use template generators
let runbook = templates::eth_transfer("alice", "bob", "1 ETH");

// Use assertion helpers
assert_action_success(&outputs, "transfer");
```

## Common Patterns

### Test Isolation

```rust
// Take checkpoint before test
let checkpoint = fixture.checkpoint().await.unwrap();

// Run test operations
fixture.execute_runbook("test").await.unwrap();

// Revert for clean state
fixture.revert(&checkpoint).await.unwrap();
```

### Multiple Actions

```rust
let runbook = r#"
action "setup" "evm::send_eth" { ... }
action "test" "evm::call_contract" { ... }
action "verify" "evm::get_balance" { ... }
"#;

fixture.execute_runbook("multi_action").await.unwrap();

// All actions generate outputs
assert!(outputs.contains_key("setup_result"));
assert!(outputs.contains_key("test_result"));
assert!(outputs.contains_key("verify_result"));
```

### Error Handling

```rust
// Test expected failures
let result = fixture.execute_runbook("should_fail").await;
assert!(result.is_err());

// Verify error type
let err = result.unwrap_err();
assert!(err.to_string().contains("expected error"));
```

### Contract Interactions

```rust
// Deploy contract
fixture.execute_runbook("deploy").await.unwrap();
let address = get_string_output(&outputs, "deploy_result", "contract_address");

// Interact with deployed contract
let interact_runbook = format!(r#"
action "call" "evm::call_contract" {{
    contract_address = "{}"
    function = "setValue"
    args = ["42"]
}}
"#, address);
```

## Best Practices

1. **Clean State**: Always start with a fresh fixture
2. **Descriptive Names**: Use clear test and action names
3. **Check Outputs**: Verify both success and actual values
4. **Use Snapshots**: For test isolation in shared Anvil
5. **Document Intent**: Add comments explaining test purpose
6. **Handle Errors**: Test both success and failure paths
7. **Reuse Code**: Use helpers and templates

## Troubleshooting

### Test Failures

1. Check Anvil is running: `ps aux | grep anvil`
2. Verify txtx is built: `cargo build --package txtx-cli`
3. Check test isolation: Ensure proper snapshot/revert
4. Review outputs: Add `--nocapture` to see details

### Common Issues

- **Port conflicts**: Anvil manager handles port allocation
- **Binary not found**: Executor builds from source
- **Parse errors**: Uses txtx-core HCL parser
- **State pollution**: Use checkpoints for isolation

## Advanced Topics

### Custom Test Helpers

Create domain-specific helpers in `helpers.rs`:

```rust
pub fn assert_token_balance(
    outputs: &HashMap<String, Value>,
    action: &str,
    expected: u128
) {
    let balance = get_int_output(outputs, action, "balance")
        .expect("Should have balance");
    assert_eq!(balance, expected as i128);
}
```

### Test Data Management

Store test data in fixtures:

```
fixtures/
├── contracts/
│   ├── complex_token.sol
│   └── defi_vault.sol
├── runbooks/
│   ├── complex_scenario.tx
│   └── stress_test.tx
└── data/
    ├── large_dataset.json
    └── test_accounts.json
```

### Performance Testing

```rust
#[tokio::test]
async fn test_performance() {
    let start = std::time::Instant::now();
    
    // Run test
    fixture.execute_runbook("perf_test").await.unwrap();
    
    let duration = start.elapsed();
    assert!(duration.as_secs() < 10, "Test too slow: {:?}", duration);
}
```

## Contributing

When adding new test infrastructure:

1. Update this guide with new patterns
2. Add examples in `example_test.rs`
3. Document in fixture builder README
4. Ensure backward compatibility
5. Add integration tests for new features