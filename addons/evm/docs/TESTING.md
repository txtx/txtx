# EVM Addon Testing Guide

## Overview

The EVM addon uses a fixture-based testing system built on top of txtx's runbook execution. Tests are written using real runbooks that execute through the txtx framework, ensuring integration testing at every level.

## Quick Start

### Basic Test Structure

```rust
#[tokio::test]
async fn test_eth_transfer() {
    // ARRANGE: Set up fixture
    let mut fixture = FixtureBuilder::new("test_transfer")
        .with_runbook("main", r#"
            addon "evm" {
                chain_id = input.chain_id
                rpc_api_url = input.rpc_url
            }
            
            action "transfer" "evm::send_eth" {
                from = input.alice_address
                to = input.bob_address
                amount = "1000000000000000000"
                signer = input.alice_signer
            }
        "#)
        .build()
        .await
        .expect("Failed to build fixture");
    
    // ACT: Execute runbook
    fixture.execute_runbook("main").await
        .expect("Failed to execute transfer");
    
    // ASSERT: Verify results
    let outputs = fixture.get_outputs("main").unwrap();
    assert!(outputs.contains_key("transfer_result"));
}
```

## FixtureBuilder API

### Creating Fixtures

```rust
let fixture = FixtureBuilder::new("test_name")
    .with_environment("testing")           // Environment name
    .with_confirmations(3)                 // Block confirmations
    .with_parameter("key", "value")        // Add parameters
    .with_runbook("name", content)         // Add runbook
    .with_contract("Token", source)        // Add Solidity contract
    .with_template("template_name")        // Use predefined template
    .build()
    .await?;
```

### Executing Runbooks

```rust
// Execute main runbook
fixture.execute_runbook("main").await?;

// Execute with specific confirmations
fixture.execute_with_confirmations("deploy", 6).await?;

// Get outputs
let outputs = fixture.get_outputs("main");
let specific_output = fixture.get_output("main", "contract_address");
```

## Test Patterns

### 1. Inline Runbook Tests

Best for simple, self-contained tests:

```rust
#[tokio::test]
async fn test_abi_encoding() {
    let runbook = r#"
        addon "evm" { chain_id = 1 }
        
        action "encode" "evm::encode_abi" {
            types = ["address", "uint256"]
            values = ["0x742d...", 123]
        }
        
        output "encoded" {
            value = action.encode.result
        }
    "#;
    
    let mut fixture = FixtureBuilder::new("test_encoding")
        .with_runbook("main", runbook)
        .build().await?;
    
    fixture.execute_runbook("main").await?;
    // ... assertions
}
```

### 2. Fixture File Tests

For complex scenarios, load from fixture files:

```rust
#[tokio::test]
async fn test_complex_contract() {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/integration/complex_contract.tx");
    
    let content = fs::read_to_string(&fixture_path)?;
    
    let mut fixture = FixtureBuilder::new("test_complex")
        .with_runbook("main", &content)
        .with_parameter("initial_supply", "1000000")
        .build().await?;
    
    fixture.execute_runbook("main").await?;
    // ... assertions
}
```

### 3. Multi-Stage Tests

For workflows with multiple steps:

```rust
#[tokio::test]
async fn test_token_workflow() {
    let mut fixture = FixtureBuilder::new("test_workflow")
        .with_contract("Token", TOKEN_SOURCE)
        .build().await?;
    
    // Stage 1: Deploy
    fixture.add_runbook("deploy", DEPLOY_RUNBOOK)?;
    fixture.execute_runbook("deploy").await?;
    let contract_address = fixture.get_output("deploy", "address");
    
    // Stage 2: Initialize
    fixture.add_runbook("init", INIT_RUNBOOK)?;
    fixture.execute_runbook("init").await?;
    
    // Stage 3: Test operations
    fixture.add_runbook("transfer", TRANSFER_RUNBOOK)?;
    fixture.execute_runbook("transfer").await?;
    
    // Verify final state
    // ... assertions
}
```

## Anvil Management

### Singleton Pattern

The test infrastructure uses a singleton Anvil instance:

```rust
// Automatically managed - starts on first test
let manager = get_anvil_manager().await?;

// Each test gets isolated snapshot
let handle = manager.get_handle("test_name").await?;
```

### Test Isolation

Each test runs in its own snapshot:

1. Test starts → snapshot created
2. Test runs → changes are isolated
3. Test ends → revert to clean state
4. Next test → starts from clean state

### Named Accounts

26 deterministic test accounts are available:

```rust
let accounts = fixture.anvil_handle.accounts();

// Access specific accounts
accounts.alice  // 0xf39fd6e51aad88f6...
accounts.bob    // 0x70997970c51812dc...
// ... through accounts.zed

// Use in runbooks via inputs
fixture.execute_with_inputs(hashmap! {
    "alice_address" => accounts.alice.address_string(),
    "alice_secret" => accounts.alice.secret_string(),
});
```

## Writing Effective Tests

### Best Practices

1. **Use ARRANGE/ACT/ASSERT pattern**
   ```rust
   // ARRANGE: Set up test environment
   let fixture = FixtureBuilder::new("test").build().await?;
   
   // ACT: Execute the operation
   fixture.execute_runbook("main").await?;
   
   // ASSERT: Verify the results
   assert_eq!(output, expected);
   ```

2. **Keep tests focused**
   - Test one behavior per test
   - Use descriptive test names
   - Avoid complex setup in individual tests

3. **Use inline runbooks for simple tests**
   - Easier to understand test intent
   - No external file dependencies
   - Better for documentation

4. **Load fixtures for complex scenarios**
   - Reusable test scenarios
   - Easier to maintain complex setups
   - Can be shared across tests

### Error Testing

Test error conditions explicitly:

```rust
#[tokio::test]
async fn test_insufficient_funds() {
    let mut fixture = FixtureBuilder::new("test_error")
        .with_runbook("main", TRANSFER_RUNBOOK)
        .with_parameter("amount", "999999999999999999999999")
        .build().await?;
    
    let result = fixture.execute_runbook("main").await;
    
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("insufficient funds"));
}
```

## Debugging Tests

### Enable Verbose Output

```bash
# Run with output
cargo test test_name -- --nocapture

# Run with debug logging
RUST_LOG=debug cargo test test_name
```

### Preserve Test Directories

```rust
let fixture = FixtureBuilder::new("test")
    .preserve_on_failure(true)  // Keep temp dir on failure
    .build().await?;
```

### Check Anvil Logs

```bash
# Check if Anvil is running
pgrep -a anvil

# View Anvil PID file
cat /tmp/txtx_test_anvil.pid
```

## Test Organization

### Directory Structure

```
addons/evm/
├── src/tests/
│   ├── fixture_builder/     # Test infrastructure
│   ├── integration/          # Integration tests
│   └── test_utils/          # Test utilities
└── fixtures/
    ├── integration/         # Integration test fixtures
    ├── contracts/           # Test contracts
    └── templates/           # Reusable templates
```

### Test Categories

1. **Unit Tests**: In source files, test individual functions
2. **Integration Tests**: In `src/tests/integration/`, test actions
3. **Infrastructure Tests**: In `src/tests/test_utils/`, test helpers
4. **Example Tests**: In `src/tests/fixture_builder/`, demonstrate usage

## Running Tests

```bash
# Run all EVM tests
cargo test --package txtx-addon-network-evm

# Run specific test
cargo test --package txtx-addon-network-evm test_name

# Run with single thread (for debugging)
cargo test --package txtx-addon-network-evm -- --test-threads=1

# Run only integration tests
cargo test --package txtx-addon-network-evm integration::
```

## Troubleshooting

### Common Issues

1. **Anvil not starting**: Ensure Foundry is installed
2. **Port conflicts**: Tests use ports 9545-9549, ensure they're free
3. **Lingering Anvil processes**: Run cleanup test: `cargo test zzz_cleanup_anvil`
4. **Compilation errors**: Run `cargo build` first to see clearer errors

### Cleanup

If tests leave Anvil processes:

```bash
# Check PID file
cat /tmp/txtx_test_anvil.pid

# Manual cleanup (only kills test Anvil)
cargo test --package txtx-addon-network-evm zzz_cleanup_anvil
```