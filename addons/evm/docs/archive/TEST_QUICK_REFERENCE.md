# EVM Test Quick Reference

## Creating a New Test - Checklist

- [ ] 1. Create fixture: `fixtures/integration/[category]/test_name.tx`
- [ ] 2. Create test file: `src/tests/integration/test_name_tests.rs`
- [ ] 3. Add module to: `src/tests/integration/mod.rs`
- [ ] 4. Run test: `cargo test --package txtx-addon-network-evm test_name`

## Copy-Paste Test Template

```rust
//! Description of what this test does

#[cfg(test)]
mod my_feature_tests {
    use crate::tests::project_test_harness::ProjectTestHarness;
    use crate::tests::integration::anvil_harness::AnvilInstance;
    use std::path::PathBuf;
    
    #[test]
    fn test_my_feature() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test - Anvil not installed");
            return;
        }
        
        println!("🧪 Testing my feature");
        
        // Load fixture
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/CATEGORY/my_test.tx");
        
        // Create harness
        let harness = ProjectTestHarness::from_fixture(&fixture_path)
            .with_anvil()
            .with_input("key", "value");
        
        // Execute
        harness.setup().expect("Failed to setup");
        let result = harness.execute_runbook()
            .expect("Failed to execute");
        
        // Verify
        assert!(result.success, "Test should succeed");
        println!("✅ Test passed");
        
        // Cleanup
        harness.cleanup();
    }
}
```

## Copy-Paste Fixture Template

```hcl
# Test: Description of what this fixture tests
# Category: transactions/deployments/errors/abi/etc

addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

signer "test_signer" "evm::secret_key" {
    secret_key = input.private_key
}

variable "my_var" {
    value = input.my_value
    description = "Description of variable"
}

action "my_action" "evm::action_type" {
    # Action configuration
    signer = signer.test_signer
    confirmations = 0
}

output "result" {
    value = action.my_action.result
}
```

## Common Patterns

### Test with Anvil
```rust
let harness = ProjectTestHarness::from_fixture(&fixture_path)
    .with_anvil();  // Starts local blockchain
```

### Test with Custom Inputs
```rust
let harness = ProjectTestHarness::from_fixture(&fixture_path)
    .with_anvil()
    .with_input("amount", "1000000000000000000")
    .with_input("gas_price", "20000000000");
```

### Error Test
```rust
let result = harness.execute_runbook();
assert!(result.is_err(), "Should fail");
assert!(result.unwrap_err().contains("expected error"));
```

### Get Output Values
```rust
let tx_hash = result.outputs.get("tx_hash")
    .and_then(|v| v.as_string())
    .expect("Should have tx_hash");
```

## Fixture Categories

| Category | Use For | Example |
|----------|---------|---------|
| `transactions/` | ETH transfers, gas tests | `simple_transfer.tx` |
| `deployments/` | Contract deployment | `deploy_contract.tx` |
| `errors/` | Error scenarios | `insufficient_funds.tx` |
| `abi/` | Encoding/decoding | `encode_function.tx` |
| `view_functions/` | Read-only calls | `view_call.tx` |
| `create2/` | CREATE2 deployment | `deterministic_deploy.tx` |

## Commands

```bash
# Run all tests
cargo test --package txtx-addon-network-evm

# Run specific test
cargo test --package txtx-addon-network-evm my_test_name

# Run with output
cargo test --package txtx-addon-network-evm -- --nocapture

# Test fixture directly
txtx run fixtures/integration/category/test.tx \
  --input chain_id=31337 \
  --input rpc_url=http://localhost:8545

# Start Anvil for manual testing
anvil

# Check if test compiles
cargo check --package txtx-addon-network-evm
```

## File Naming Rules

| Type | Pattern | Example |
|------|---------|---------|
| Test file | `*_tests.rs` | `transfer_tests.rs` |
| Fixture | `*.tx` | `simple_transfer.tx` |
| Harness | No suffix | `anvil_harness.rs` |
| Utils | No suffix | `test_utils.rs` |

## Common Assertions

```rust
// Success/failure
assert!(result.success, "Should succeed");
assert!(result.is_err(), "Should fail");

// Output exists
assert!(result.outputs.contains_key("tx_hash"));

// Output value
assert_eq!(
    result.outputs.get("value").unwrap().as_string().unwrap(),
    "expected_value"
);

// Error contains
assert!(error_msg.contains("insufficient funds"));

// Transaction hash format
assert!(tx_hash.starts_with("0x"));
assert_eq!(tx_hash.len(), 66); // 0x + 64 hex chars
```

## Debugging Tips

```rust
// Print outputs
println!("Outputs: {:?}", result.outputs);

// Print specific output
if let Some(value) = result.outputs.get("key") {
    println!("Value: {:?}", value);
}

// Print error with context
let error = result.unwrap_err();
println!("Error: {}", error);

// Keep temp directory on failure (check project_test_harness.rs)
// The harness will preserve temp dir if test fails
```

## Links

- [Full Test Creation Guide](./TEST_CREATION_GUIDE.md)
- [Naming Convention](./TEST_NAMING_CONVENTION.md)
- [Fixture README](./fixtures/README.md)
- [Migration Tracker](./TEST_MIGRATION_TRACKER.md)