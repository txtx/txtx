# EVM Test Fixtures

This directory contains txtx runbook files (`.tx`) used for testing the EVM addon.

## Directory Structure

```
fixtures/
├── runbooks/           # Test runbook files
│   ├── integration/    # Integration test runbooks
│   ├── errors/        # Error scenario runbooks
│   └── codec/         # Codec test runbooks
├── foundry/           # Foundry project templates
│   ├── src/           # Solidity source files
│   └── out/           # Compiled artifacts
└── signers/           # Signer configurations
```

## Test Runbooks

Test runbooks are `.tx` files that define test scenarios. They use the standard txtx runbook format:

```hcl
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

action "test_action" "evm::some_action" {
    // Action configuration
}

output "result" {
    value = action.test_action.result
}
```

## Usage in Tests

Tests use `ProjectTestHarness` to execute these runbooks:

```rust
let harness = ProjectTestHarness::new_foundry_from_fixture(
    "integration/simple_send_eth.tx"
)
.with_anvil()
.with_input("key", "value");

harness.setup().unwrap();
let result = harness.execute_runbook().unwrap();
```

## Adding New Test Fixtures

1. Create a `.tx` file in the appropriate subdirectory
2. Define the test scenario using txtx runbook syntax
3. Use `input.variable_name` for parameterized values
4. Create a test in the corresponding test file that uses the fixture

## Current Fixtures

### Integration Tests
- `simple_send_eth.tx` - Basic ETH transfer
- `simple_send_eth_with_env.tx` - ETH transfer with environment config
- `deploy_contract.tx` - Contract deployment

### Error Tests
- `insufficient_funds.tx` - Test insufficient funds error
- `invalid_address.tx` - Test invalid address error

### Codec Tests
- `invalid_hex.tx` - Test invalid hex encoding

---

_For test architecture details, see [TEST_ARCHITECTURE.md](../../../TEST_ARCHITECTURE.md)_

_For migration guide, see [TEST_MIGRATION_GUIDE.md](../../../TEST_MIGRATION_GUIDE.md)_