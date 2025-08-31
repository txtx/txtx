# EVM Addon Test Fixtures

This directory contains txtx runbook fixtures used for testing the EVM addon. Fixtures are organized by category and designed for reuse across multiple tests.

## Directory Structure

```
fixtures/
├── integration/           # Integration test fixtures (execute on blockchain)
│   ├── create2/          # CREATE2 deployment tests
│   ├── transactions/     # Transaction tests (transfers, gas, etc.)
│   ├── deployments/      # Contract deployment tests
│   ├── abi/             # ABI encoding/decoding tests
│   ├── errors/          # Error handling tests
│   └── view_functions/  # View/pure function tests
├── parsing/              # Parse-only test fixtures (no execution)
│   ├── basic_send_eth.tx # Minimal ETH transfer
│   ├── basic_deploy.tx   # Minimal deployment
│   └── basic_call.tx     # Minimal contract call
└── README.md
```

## Fixture Reusability

Fixtures are designed to be reused across multiple tests with different parameters:

### Example: One Fixture, Multiple Uses

```rust
// Test 1: Basic execution test
let harness = ProjectTestHarness::from_fixture(&fixture_path)
    .with_anvil();

// Test 2: Parse-only test (no blockchain)
let harness = ProjectTestHarness::from_fixture(&fixture_path);
// Just verify parsing, no execution

// Test 3: Same fixture with custom inputs
let harness = ProjectTestHarness::from_fixture(&fixture_path)
    .with_anvil()
    .with_input("recipient", "0xCustomAddress...")
    .with_input("amount", "2000000000000000000");
```

### Reusability Matrix

| Fixture | Used By Tests | Purpose |
|---------|--------------|---------|
| `simple_eth_transfer.tx` | transfer tests, parsing tests | Basic ETH transfer pattern |
| `minimal_contract.tx` | deployment tests, parsing tests | Simplest deployment |
| `deploy_and_interact.tx` | interaction tests, integration tests | Full deploy + call flow |
| `parsing/basic_*.tx` | All parse-only tests | Minimal parsing validation |

## Benefits

1. **Discoverability** - All runbooks in one place
2. **Reusability** - Same runbook can be used by multiple tests
3. **Maintainability** - Single source of truth for each runbook
4. **CLI Testing** - Can test runbooks directly with `txtx` CLI
5. **Documentation** - Serves as examples for users

## Adding New Fixtures

When adding a new test fixture:

1. Choose the appropriate category directory
2. Create a descriptive `.tx` file
3. Use `input` variables for dynamic values
4. Document the fixture purpose in comments
5. Reference from tests using `ProjectTestHarness::from_fixture()`

## Testing Fixtures Directly

Fixtures can be tested directly with the txtx CLI:

```bash
# Test a fixture runbook
txtx run fixtures/integration/create2/address_calculation.tx

# With inputs
txtx run fixtures/integration/deployments/simple_deploy.tx \
  --input chain_id=31337 \
  --input rpc_url=http://localhost:8545
```