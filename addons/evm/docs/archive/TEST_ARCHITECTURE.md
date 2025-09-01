# EVM Test Architecture

## Overview

The EVM addon test architecture centers around the `ProjectTestHarness`, which creates complete txtx project environments for testing runbooks through the actual txtx framework.

## Core Components

### ProjectTestHarness (`project_test_harness.rs`)

The main test harness that creates and manages test projects:

```rust
pub struct ProjectTestHarness {
    pub temp_dir: TempDir,              // Temporary test directory
    pub project_path: PathBuf,          // Project root path
    pub framework: CompilationFramework, // Foundry or Hardhat
    pub inputs: HashMap<String, String>, // Runbook inputs
    pub runbook_content: String,        // The runbook to test
    pub runbook_name: String,           // Runbook filename
    pub anvil: Option<AnvilInstance>,   // Optional Anvil instance
}
```

#### Key Methods

- `new_foundry()` / `new_hardhat()` - Create harness with specific framework
- `with_input()` - Add input values for the runbook
- `with_anvil()` - Spawn local Anvil instance for testing
- `setup()` - Create project structure (txtx.yml, contracts, etc.)
- `execute_runbook()` - Execute runbook through txtx-core

### Test Project Structure

When `setup()` is called, it creates:

```
temp_dir/
├── txtx.yml                 # Project configuration
├── runbooks/
│   ├── test.tx              # The test runbook
│   └── signers.testing.tx  # Test signers
└── out/ (or artifacts/)     # Compilation outputs
    └── Contract.json        # Contract artifacts
```

### Integration with txtx-core

The `execute_runbook()` method integrates with txtx-core to actually execute runbooks:

1. Parses runbook using `RunbookSources`
2. Builds contexts with addon lookup
3. Executes through `start_unsupervised_runbook_runloop`
4. Collects and returns outputs

## Test Fixtures

Test fixtures are `.tx` runbook files stored in `src/tests/fixtures/runbooks/`:

```
fixtures/runbooks/
├── integration/
│   ├── simple_send_eth.tx
│   ├── simple_send_eth_with_env.tx
│   └── deploy_contract.tx
├── errors/
│   ├── insufficient_funds.tx
│   └── invalid_address.tx
└── codec/
    └── invalid_hex.tx
```

### Example Test Runbook

```hcl
# simple_send_eth.tx
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

signer "sender" "evm::secret_key" {
    secret_key = input.sender_private_key
}

action "transfer" "evm::send_eth" {
    from = input.sender_address
    to = input.recipient_address
    amount = "1000000000000000000"
    signer = signer.sender
}

output "tx_hash" {
    value = action.transfer.tx_hash
}
```

## Test Patterns

### Basic Test Pattern

```rust
#[test]
fn test_eth_transfer() {
    // Create harness with runbook
    let harness = ProjectTestHarness::new_foundry_from_fixture(
        "integration/simple_send_eth.tx"
    )
    .with_anvil()
    .with_input("sender_address", ANVIL_ACCOUNTS[0])
    .with_input("sender_private_key", ANVIL_KEYS[0])
    .with_input("recipient_address", ANVIL_ACCOUNTS[1]);
    
    // Setup project structure
    harness.setup().expect("Setup should succeed");
    
    // Execute runbook
    let result = harness.execute_runbook()
        .expect("Execution should succeed");
    
    // Verify outputs
    assert!(result.outputs.contains_key("tx_hash"));
}
```

### Error Testing Pattern

```rust
#[test]
fn test_insufficient_funds() {
    let harness = ProjectTestHarness::new_foundry_from_fixture(
        "errors/insufficient_funds.tx"
    )
    .with_anvil();
    
    harness.setup().expect("Setup should succeed");
    
    let result = harness.execute_runbook();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("insufficient funds"));
}
```

## Anvil Integration

The `AnvilInstance` provides local blockchain for testing:

```rust
pub struct AnvilInstance {
    pub process: Child,
    pub url: String,
    pub chain_id: u32,
    pub accounts: Vec<TestAccount>,
}
```

Tests can use Anvil by calling `.with_anvil()` on the harness, which:
1. Spawns Anvil process
2. Configures test accounts
3. Passes RPC URL as input to runbook

## Known Limitations

1. **Signer Initialization**: Some signer configurations cause panics at runtime
2. **Action Execution**: Full action execution through txtx-core needs investigation
3. **State Verification**: Chain state verification after execution not fully implemented

## Test Organization

Tests are organized by functionality:

- `integration/` - Integration tests using full txtx flow
- `codec_tests.rs` - Type conversion tests (to be migrated)
- `error_handling_tests.rs` - Error scenario tests (to be migrated)
- `transaction_tests.rs` - Transaction tests (to be migrated)
- `txtx_runbook_tests.rs` - Tests already using txtx

## Future Improvements

1. Fix signer initialization issues
2. Implement full chain state verification
3. Add performance benchmarking
4. Create test generators for common patterns

---

_For migration guide, see [TEST_MIGRATION_GUIDE.md](./TEST_MIGRATION_GUIDE.md)_

_For current status, see [TEST_MIGRATION_TRACKER.md](./TEST_MIGRATION_TRACKER.md)_