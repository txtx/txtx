# Test Harness Interface Documentation

## Overview
The test harness provides an ergonomic interface for testing txtx runbooks with structured outputs and assertions. The key insight is that we don't need to modify txtx - we can use the Report<...> for success/failure and read any needed data from the temp folder.

## Core Design Principles

1. **Structured Test Logs in Runbooks**: Create custom output objects in runbooks that organize test data
2. **Path-Based Access**: Access nested values using dot notation (e.g., `"actions.send_eth.tx_hash"`)
3. **Object Comparison**: Compare complex nested objects with partial field matching
4. **Event Extraction**: Parse transaction logs to verify events

## API Reference

### Basic Assertions

```rust
// Get an output value by name
harness.get_output("tx_hash") -> Option<Value>

// Get a value at a specific path in the test_log output
harness.get_log_path("actions.send_eth.tx_hash") -> Option<Value>

// Assert a path matches expected value
harness.assert_log_path("validation.amount_correct", Value::Bool(true), "Amount should be correct")

// Check if an action succeeded
harness.action_succeeded("send_eth") -> bool
```

### Object Comparison

```rust
// Build expected objects
let expected = ExpectedValueBuilder::new()
    .with_string("tx_hash", "0x123")
    .with_bool("success", true)
    .with_integer("gas_used", 21000)
    .build();

// Compare entire objects
let result = actual.compare_with(&expected);
assert!(result.matches);

// Compare only specific fields
actual.compare_fields(&expected, &["tx_hash", "success"])

// Assert object at path matches
harness.assert_log_object("actions.send_eth", expected_builder);
```

### Event Extraction

```rust
// Extract events from a receipt
let events = extract_events_from_receipt(&receipt_value);

// Filter events
let transfers = filter_events_by_name(&events, "Transfer");
let contract_events = filter_events_by_address(&events, "0x123...");
```

## Runbook Pattern: Structured Test Logs

Create a `test_log` output in your runbook that organizes all test data:

```hcl
output "test_log" {
    value = {
        # Test metadata
        test_metadata = {
            test_name = "eth_transfer_test"
            timestamp = "2024-08-31"
            chain_id = input.chain_id
        }
        
        # Input values for verification
        inputs = {
            sender = input.sender_address
            recipient = input.recipient
            amount = input.amount
        }
        
        # Action results
        actions = {
            send_eth = {
                executed = true
                tx_hash = action.send_eth.tx_hash
                success = action.send_eth.success
                gas_used = action.send_eth.gas_used
                receipt = action.send_eth.receipt
            }
        }
        
        # Validation checks
        validation = {
            amount_correct = (input.amount == 1000000000000000000)
            recipient_valid = (input.recipient != "")
            tx_hash_present = (action.send_eth.tx_hash != "")
        }
        
        # Events (if extracting from receipt)
        events = {
            transfer_count = length(action.send_eth.receipt.logs)
            # More complex event parsing can be done
        }
    }
}
```

## Test Example

```rust
#[test]
fn test_eth_transfer() {
    let harness = ProjectTestHarness::new_from_fixture("eth_transfer_with_test_log.tx")
        .with_anvil();
    
    harness.setup().unwrap();
    let result = harness.execute_runbook().unwrap();
    
    // Basic success check
    assert!(result.success);
    
    // Check structured validations
    assert_eq!(
        harness.get_log_path("validation.amount_correct"),
        Some(Value::Bool(true))
    );
    
    // Compare action results
    let expected_action = ExpectedValueBuilder::new()
        .with_bool("executed", true)
        .with_bool("success", true);
    
    harness.assert_log_object("actions.send_eth", expected_action);
    
    // Extract and verify events
    if let Some(receipt) = harness.get_log_path("actions.send_eth.receipt") {
        let events = extract_events_from_receipt(&receipt);
        let transfers = filter_events_by_name(&events, "Transfer");
        assert_eq!(transfers.len(), 1);
    }
}
```

## Benefits

1. **No txtx Modification Required**: Uses existing outputs and temp folder
2. **Readable Tests**: Clear assertions with meaningful paths
3. **Flexible Validation**: Compare entire objects or specific fields
4. **Event Support**: Extract and verify blockchain events
5. **Debugging Support**: Structured logs make it easy to debug failures

## Implementation Status

✅ **Completed**:
- Value comparison with nested paths
- Object comparison with partial field matching
- Expected value builder for ergonomic test writing
- Event extraction from receipts
- Test fixtures demonstrating patterns

🚧 **In Progress**:
- Actual runbook execution (currently mocked)
- Reading state from .txtx folder
- JSON output parsing

## Next Steps

1. Fix the unsupervised execution hanging issue
2. Implement actual state reading from temp folder
3. Add more event decoders for common contracts
4. Create test templates for common scenarios