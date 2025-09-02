# EVM Test Fixture Builder

A comprehensive fixture-based testing system for txtx EVM runbooks that provides isolated test environments with automatic output generation and Anvil blockchain integration.

## Overview

The fixture builder creates isolated test environments for running txtx runbooks with:
- Managed Anvil blockchain instances with snapshot/revert capability
- Automatic test output generation for runbook actions
- 26 pre-configured named test accounts
- Integration with txtx-core's HCL parser
- Always builds from source to ensure testing current code

## Architecture

### Core Components

1. **FixtureBuilder** (`mod.rs`)
   - Entry point for creating test fixtures
   - Manages temp directories and project structure
   - Auto-generates `txtx.yml` configuration
   - Provides fluent API for configuration

2. **AnvilManager** (`anvil_manager.rs`)
   - Singleton pattern for shared Anvil instance
   - Snapshot/revert for test isolation
   - TCP-based health checking
   - Automatic port management

3. **RunbookParser** (`runbook_parser.rs`)
   - Leverages txtx-core's HCL parser
   - Extracts actions from runbooks
   - Auto-generates output blocks for testing
   - Creates test metadata

4. **Executor** (`executor.rs`)
   - Builds txtx CLI from source
   - Executes runbooks in unsupervised mode
   - Captures and parses JSON outputs
   - Handles error cases gracefully

5. **NamedAccounts** (`accounts.rs`)
   - 26 deterministic accounts (alice through zed)
   - Derived from test mnemonic
   - EIP-55 checksum addresses
   - Easy access by name

## Usage

### Basic Test Fixture

```rust
#[tokio::test]
async fn test_basic_fixture() {
    let fixture = FixtureBuilder::new("my_test")
        .with_environment("testing")
        .with_confirmations(0)
        .build()
        .await
        .expect("Failed to build fixture");
    
    // Fixture provides:
    // - fixture.project_dir: Path to test project
    // - fixture.rpc_url: Anvil RPC endpoint
    // - fixture.anvil_handle: Access to accounts and chain
}
```

### Running a Runbook

```rust
#[tokio::test]
async fn test_eth_transfer() {
    let mut fixture = FixtureBuilder::new("test_transfer")
        .build()
        .await
        .unwrap();
    
    let runbook = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

signer "alice" "evm::private_key" {
    private_key = input.alice_secret
}

action "transfer" "evm::send_eth" {
    recipient_address = input.bob_address
    amount = 1000000000000000000  // 1 ETH in wei - must be integer, not string!
    signer = signer.alice
    confirmations = 0
}
"#;
    
    fixture.add_runbook("transfer", runbook).unwrap();
    fixture.execute_runbook("transfer").await.unwrap();
    
    // Outputs are automatically generated and available
    let outputs = fixture.get_outputs("transfer").unwrap();
    assert!(outputs.contains_key("transfer_result"));
}
```

### Test Isolation with Snapshots

```rust
#[tokio::test]
async fn test_with_isolation() {
    let mut fixture = FixtureBuilder::new("test_isolation")
        .build()
        .await
        .unwrap();
    
    // Execute some operations
    fixture.execute_runbook("setup").await.unwrap();
    
    // Take a checkpoint
    let checkpoint = fixture.checkpoint().await.unwrap();
    
    // Execute more operations
    fixture.execute_runbook("test").await.unwrap();
    
    // Revert to checkpoint for clean state
    fixture.revert(&checkpoint).await.unwrap();
}
```

### Using Named Accounts

The fixture automatically provides 26 test accounts with predictable addresses:

```rust
let accounts = fixture.anvil_handle.accounts();

// Access by name
let alice = accounts.get("alice").unwrap();
let bob = accounts.get("bob").unwrap();

// All accounts have:
// - address: Ethereum address
// - private_key: Private key for signing
```

Account names: alice, bob, charlie, dave, eve, frank, grace, heidi, ivan, judy, karl, lisa, mike, nancy, oscar, peggy, quinn, rupert, sybil, trent, ursula, victor, walter, xander, yara, zed

## Automatic Output Generation

The parser automatically adds output blocks for each action in your runbook:

1. **Individual action outputs**: `{action_name}_result`
2. **Aggregate test output**: `test_output` containing all results
3. **Test metadata**: `test_metadata` with action types and descriptions

Example generated outputs:
```hcl
output "transfer_result" {
  value = action.transfer.result
}

output "test_output" {
  value = {
    transfer_result = action.transfer.result
  }
}

output "test_metadata" {
  value = {
    transfer = {
      type = "evm::send_eth"
      description = "Transfer 1 ETH"
    }
  }
}
```

## Configuration Options

### FixtureBuilder Options

- `with_environment(env)`: Set the environment (default: "testing")
- `with_confirmations(n)`: Set block confirmations to wait
- `with_template(name)`: Apply a template (future feature)
- `with_parameter(key, value)`: Add custom parameters
- `with_contract(name, source)`: Add a Solidity contract
- `with_runbook(name, content)`: Add a runbook
- `with_anvil_manager(manager)`: Use shared Anvil instance

### Environment Variables

The fixture automatically sets up:
- `chain_id`: Anvil chain ID (31337)
- `rpc_url`: Anvil RPC endpoint
- `{name}_address`: Address for each named account
- `{name}_secret`: Private key for each named account

## Implementation Details

### Anvil Management

- Single shared Anvil instance per test run
- Automatic port selection (default: 8548)
- Health checking via TCP connect
- Graceful cleanup on drop

### Source-Based Testing

The executor always builds txtx from the current source code rather than using potentially outdated binaries. This ensures tests always run against the code being developed.

### HCL Parsing

Uses `txtx-core`'s `RawHclContent::from_string()` for parsing, ensuring consistency with the actual txtx runtime and proper handling of all HCL syntax.

## Testing Best Practices

1. **Use meaningful test names**: Helps identify failures and debug issues
2. **Clean up resources**: Fixtures automatically clean up, but be mindful of snapshots
3. **Test in isolation**: Use checkpoints/reverts for test independence
4. **Verify outputs**: Check both success and actual values returned
5. **Use named accounts**: Predictable and easy to reference

## Future Enhancements

- [ ] Template system for common test patterns
- [ ] Advanced output validation helpers
- [ ] Contract compilation integration
- [ ] Gas usage tracking and assertions
- [ ] Event log verification
- [ ] Multi-chain testing support

## Contributing

When adding new test fixtures:
1. Follow the existing patterns for consistency
2. Document any new features or patterns
3. Ensure proper cleanup in Drop implementations
4. Add integration tests for new functionality
5. Update this README with new capabilities