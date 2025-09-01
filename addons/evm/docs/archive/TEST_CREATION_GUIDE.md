# EVM Addon Test Creation Guide

This guide explains how to create new tests for the EVM addon following our established patterns.

## Quick Start: Creating a New Test

### Step 1: Create the Fixture File

Create a `.tx` runbook file in the appropriate fixture directory:

```bash
# For integration tests
addons/evm/fixtures/integration/[category]/your_test.tx

# Categories:
# - transactions/     # ETH transfers, gas estimation, etc.
# - deployments/      # Contract deployment tests
# - errors/          # Error handling scenarios
# - abi/            # ABI encoding/decoding
# - view_functions/ # View/pure function calls
# - create2/        # CREATE2 deployment
```

Example fixture (`fixtures/integration/transactions/simple_transfer.tx`):
```hcl
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

signer "sender" "evm::secret_key" {
    secret_key = input.sender_private_key
}

action "transfer" "evm::send_eth" {
    recipient_address = input.recipient_address
    amount = 1000000000000000000  # 1 ETH
    signer = signer.sender
    confirmations = 0
}

output "tx_hash" {
    value = action.transfer.tx_hash
}
```

### Step 2: Create the Test File

Create a test file with `_tests.rs` suffix in `src/tests/integration/`:

```rust
//! Simple ETH transfer tests using txtx framework

#[cfg(test)]
mod simple_transfer_tests {
    use crate::tests::project_test_harness::ProjectTestHarness;
    use crate::tests::integration::anvil_harness::AnvilInstance;
    use std::path::PathBuf;
    
    #[test]
    fn test_simple_eth_transfer() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test - Anvil not installed");
            return;
        }
        
        println!("💸 Testing ETH transfer");
        
        // Load fixture from filesystem
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/transactions/simple_transfer.tx");
        
        // Create test harness with Anvil
        let harness = ProjectTestHarness::from_fixture(&fixture_path)
            .with_anvil()  // Automatically sets up local blockchain
            .with_input("recipient_address", "0x70997970C51812dc3A010C7d01b50e0d17dc79C8");
        
        // Setup and execute
        harness.setup().expect("Failed to setup project");
        let result = harness.execute_runbook()
            .expect("Failed to execute runbook");
        
        // Verify results
        assert!(result.success, "Transfer should succeed");
        assert!(result.outputs.contains_key("tx_hash"), "Should have tx_hash output");
        
        println!("✅ Transfer successful: {}", 
            result.outputs.get("tx_hash").unwrap().as_string().unwrap());
        
        // Cleanup
        harness.cleanup();
    }
}
```

### Step 3: Add to Module

Add your test module to `src/tests/integration/mod.rs`:

```rust
pub mod simple_transfer_tests;  // Add this line
```

## Test Patterns

### Pattern 1: Basic Test with Anvil

```rust
#[test]
fn test_something_with_blockchain() {
    // 1. Check Anvil availability
    if !AnvilInstance::is_available() {
        eprintln!("⚠️  Skipping - Anvil not installed");
        return;
    }
    
    // 2. Load fixture
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/integration/category/test.tx");
    
    // 3. Create harness with Anvil
    let harness = ProjectTestHarness::from_fixture(&fixture_path)
        .with_anvil();
    
    // 4. Execute and verify
    harness.setup().expect("Failed to setup");
    let result = harness.execute_runbook()
        .expect("Failed to execute");
    
    assert!(result.success);
    
    // 5. Cleanup
    harness.cleanup();
}
```

### Pattern 2: Test with Dynamic Inputs

```rust
#[test]
fn test_with_custom_inputs() {
    let harness = ProjectTestHarness::from_fixture(&fixture_path)
        .with_anvil()
        .with_input("amount", "1000000000000000000")  // 1 ETH
        .with_input("gas_price", "20000000000")       // 20 Gwei
        .with_input("custom_abi", serde_json::to_string(&abi).unwrap());
    
    // Rest of test...
}
```

### Pattern 3: Error Testing

```rust
#[test]
fn test_error_handling() {
    let harness = ProjectTestHarness::from_fixture(&fixture_path)
        .with_anvil();
    
    harness.setup().expect("Failed to setup");
    
    // Expect failure
    let result = harness.execute_runbook();
    assert!(result.is_err(), "Should fail with error");
    
    let error_msg = result.unwrap_err();
    assert!(error_msg.contains("insufficient funds"), 
        "Error should mention insufficient funds: {}", error_msg);
}
```

### Pattern 4: Testing Without Anvil

```rust
#[test]
fn test_pure_computation() {
    // For tests that don't need blockchain (e.g., ABI encoding)
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/integration/abi/encode_test.tx");
    
    let harness = ProjectTestHarness::from_fixture(&fixture_path);
    // Note: No .with_anvil() call
    
    harness.setup().expect("Failed to setup");
    let result = harness.execute_runbook()
        .expect("Failed to execute");
    
    assert_eq!(result.outputs.get("encoded").unwrap().as_string().unwrap(),
        "0xabcdef...", "Encoding should match expected");
}
```

## Fixture Best Practices

### 1. Reuse Existing Fixtures

Before creating a new fixture, check if an existing one can be reused:

```rust
// ✅ Good - reuse with parameters
let harness = ProjectTestHarness::from_fixture("simple_eth_transfer.tx")
    .with_input("recipient", "0xCustomAddress...")
    .with_input("amount", "500000000000000000");

// ❌ Bad - create duplicate fixture for minor variation
// Don't create simple_eth_transfer_half_eth.tx
```

### 2. Use Input Variables

```hcl
# Good - uses inputs for dynamic values
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

# Bad - hardcoded values
addon "evm" {
    chain_id = 31337
    rpc_api_url = "http://localhost:8545"
}
```

### 2. Document Your Fixtures

```hcl
# Test: Verify CREATE2 address calculation is deterministic
# This fixture tests that CREATE2 produces the same address
# when called multiple times with the same parameters

variable "salt" {
    value = input.salt
    description = "Salt for CREATE2 deployment"
}
```

### 3. Use Meaningful Output Names

```hcl
# Good - descriptive output names
output "deployed_contract_address" {
    value = action.deploy.contract_address
}

output "deployment_gas_used" {
    value = action.deploy.gas_used
}

# Bad - generic names
output "result" {
    value = action.deploy.contract_address
}
```

## Directory Structure

```
addons/evm/
├── fixtures/
│   ├── README.md                    # Fixture documentation
│   ├── integration/                 # Fixtures that execute on blockchain
│   │   ├── transactions/
│   │   │   ├── simple_transfer.tx
│   │   │   └── batch_transfer.tx
│   │   ├── deployments/
│   │   │   └── contract_deploy.tx
│   │   └── errors/
│   │       └── insufficient_funds.tx
│   └── parsing/                     # Minimal fixtures for parse-only tests
│       ├── basic_send_eth.tx
│       ├── basic_deploy.tx
│       └── basic_call.tx
├── src/
│   └── tests/
│       ├── integration/
│       │   ├── mod.rs
│       │   ├── transaction_tests.rs  # Uses fixtures
│       │   └── deployment_tests.rs   # Uses fixtures
│       └── project_test_harness.rs   # Test framework
└── TEST_CREATION_GUIDE.md           # This file
```

## Running Tests

```bash
# Run all EVM tests
cargo test --package txtx-addon-network-evm

# Run specific test
cargo test --package txtx-addon-network-evm test_simple_eth_transfer

# Run with output
cargo test --package txtx-addon-network-evm -- --nocapture

# Test fixture directly with CLI
txtx run fixtures/integration/transactions/simple_transfer.tx \
  --input chain_id=31337 \
  --input rpc_url=http://localhost:8545
```

## Common Test Utilities

### ProjectTestHarness

- `from_fixture(&Path)` - Load runbook from filesystem
- `with_anvil()` - Start local blockchain
- `with_input(key, value)` - Add input variable
- `setup()` - Initialize test environment
- `execute_runbook()` - Run the txtx runbook
- `cleanup()` - Clean up temp files

### AnvilInstance

- `is_available()` - Check if Anvil is installed
- `spawn()` - Start new Anvil instance
- Provides test accounts with private keys

## Troubleshooting

### Test Fails to Find Fixture

```rust
// Make sure to use CARGO_MANIFEST_DIR
let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("fixtures/integration/...");
```

### Anvil Not Available

```bash
# Install Foundry (includes Anvil)
curl -L https://foundry.paradigm.xyz | bash
foundryup
```

### Test Cleanup Issues

Always call `harness.cleanup()` at the end of tests, or use the test harness's built-in cleanup on drop.

## Migration from Old Pattern

If you have tests using inline runbooks:

```rust
// OLD - Don't do this
let runbook = r#"
addon "evm" {
    chain_id = 31337
    ...
}
"#;
let harness = ProjectTestHarness::new_foundry("test.tx", runbook);
```

Convert to:

```rust
// NEW - Use filesystem fixtures
let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("fixtures/integration/category/test.tx");
let harness = ProjectTestHarness::from_fixture(&fixture_path);
```

## Summary

1. **Always use filesystem fixtures** - Never inline runbooks in test code
2. **Follow naming conventions** - Test files end with `_tests.rs`
3. **Organize fixtures by category** - Use the established directory structure
4. **Use input variables** - Make fixtures reusable with different inputs
5. **Test with CLI** - Fixtures can be run directly with `txtx run`

This pattern ensures tests are maintainable, discoverable, and can be tested both through Rust tests and the txtx CLI.