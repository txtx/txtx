# EVM Test Harness Documentation

## Overview

The EVM test harness provides a comprehensive testing framework for executing txtx runbooks and validating their outputs. It creates isolated test environments, executes runbooks via the txtx CLI, and provides utilities for asserting on outputs and blockchain state.

## Architecture

### Core Components

1. **ProjectTestHarness** (`src/tests/test_harness/mod.rs`)
   - Main test harness struct that manages test lifecycle
   - Creates temporary project directories
   - Configures and executes runbooks
   - Reads and parses output JSON files

2. **Assertions Module** (`src/tests/test_harness/assertions.rs`)
   - `ValueComparison` trait for comparing nested txtx Values
   - `ExpectedValueBuilder` for constructing expected values
   - `ComparisonResult` for detailed assertion failures

3. **Events Module** (`src/tests/test_harness/events.rs`)
   - Utilities for extracting and parsing EVM transaction logs
   - `ParsedEvent` struct for decoded event data
   - Helper functions for filtering events

## Usage

### Basic Test Setup

```rust
use crate::tests::test_harness::ProjectTestHarness;

#[test]
fn test_simple_runbook() {
    // Create harness with runbook content
    let runbook_content = r#"
        output "result" {
            value = "hello"
        }
    "#;
    
    let harness = ProjectTestHarness::new_with_content("test.tx", runbook_content);
    
    // Setup project structure
    harness.setup().expect("Failed to setup");
    
    // Execute runbook
    let result = harness.execute_runbook().expect("Failed to execute");
    
    // Assert on outputs
    assert_eq!(
        harness.get_output("result"),
        Some(Value::String("hello".to_string()))
    );
}
```

### Testing with Anvil

```rust
#[test]
fn test_eth_transfer() {
    let harness = ProjectTestHarness::new_foundry_from_fixture("integration/send_eth.tx")
        .with_anvil()  // Spawns local Anvil instance
        .with_input("amount", "1000000000000000000");  // 1 ETH
    
    harness.setup().expect("Failed to setup");
    let result = harness.execute_runbook().expect("Failed to execute");
    
    // Check transaction hash was generated
    assert!(harness.get_output("tx_hash").is_some());
}
```

### Structured Test Logs

The harness supports a structured logging pattern where runbooks output a `test_log` object:

```rust
// In runbook:
output "test_log" {
    value = {
        inputs = {
            sender = input.sender_address
            amount = input.amount
        }
        actions = {
            send_eth = {
                tx_hash = action.send_eth.tx_hash
                success = action.send_eth.success
            }
        }
    }
}

// In test:
harness.assert_log_path(
    "actions.send_eth.success",
    Value::Bool(true),
    "Transaction should succeed"
);
```

## Test Execution Flow

1. **Project Setup**
   - Creates temp directory
   - Generates `txtx.yml` configuration
   - Creates `runbooks/` directory structure
   - Sets up compilation framework (Foundry/Hardhat)

2. **Runbook Execution**
   - Builds txtx CLI binary if needed
   - Runs `txtx run <runbook> --env testing --output-json runs -u`
   - Captures execution status and errors

3. **Output Reading**
   - Reads JSON from `runs/testing/<runbook>_<timestamp>.output.json`
   - Parses nested value structures
   - Converts JSON to txtx Value types

4. **Cleanup**
   - Automatically cleans up temp directories on success
   - Preserves failed test directories for debugging

## Key Features

### Compilation Framework Support

The harness supports both Foundry and Hardhat projects:

```rust
// Foundry (default)
let harness = ProjectTestHarness::new_foundry("test.tx");

// Hardhat
let harness = ProjectTestHarness::new_hardhat("test.tx");
```

### Input Management

Inputs can be provided via:
- Constructor: `with_input("key", "value")`
- Environment variables in txtx.yml
- Command-line `--input` flags

### Anvil Integration

When `.with_anvil()` is called:
- Spawns a local Anvil instance on an available port
- Automatically configures RPC URL and chain ID
- Provides test accounts with private keys
- Cleans up Anvil process on test completion

### Output Access Methods

```rust
// Get direct output value
let value = harness.get_output("output_name");

// Get nested value using path notation
let nested = harness.get_log_path("path.to.nested.value");

// Assert on values
harness.assert_log_path("path", expected_value, "assertion message");

// Check action success
if harness.action_succeeded("deploy_contract") {
    // Action completed successfully
}
```

## Test Fixtures

Fixtures are stored in `fixtures/integration/` and demonstrate common patterns:

- `simple_send_eth.tx` - Basic ETH transfer
- `deploy_contract.tx` - Contract deployment
- `contract_interaction.tx` - Calling contract functions
- `eth_transfer_with_test_log.tx` - Structured logging example

## Environment Configuration

The test harness generates the following txtx.yml structure:

```yaml
---
name: test-project
id: test-project
runbooks:
  - name: <runbook_name>
    id: <runbook_name>
    description: Test runbook
    location: runbooks/<runbook_name>.tx
environments:
  global:
    confirmations: 0
  testing:
    confirmations: 0
    # Additional inputs configured here
```

## Debugging Failed Tests

When a test fails, the temporary directory is preserved. The path is printed in the test output:

```
Preserving failed test directory: /tmp/.tmpXXXXXX
```

You can inspect:
- `runbooks/` - The generated runbook files
- `runs/testing/` - Output JSON files
- `txtx.yml` - Project configuration
- `.txtx/` - Execution state (if present)

## Best Practices

1. **Use Structured Logs**: Output a `test_log` object with organized data
2. **Test in Isolation**: Each test gets its own temp directory
3. **Validate Outputs**: Always check that expected outputs exist
4. **Handle Async Operations**: Use proper confirmations settings
5. **Clean Resources**: The harness automatically cleans up, but check for leaks

## Limitations

- Requires txtx CLI binary to be built
- Anvil must be installed for blockchain tests
- Forge must be installed for Foundry projects
- Tests run in unsupervised mode (no interactive UI)

## Future Enhancements

- [ ] Support for multiple runbook execution in sequence
- [ ] State snapshot and rollback capabilities
- [ ] Parallel test execution support
- [ ] Mock mode for faster unit tests
- [ ] Integration with txtx LSP for validation