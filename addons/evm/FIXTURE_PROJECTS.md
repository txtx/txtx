# EVM Fixture Projects

This document describes the fixture projects available for testing complex EVM scenarios.

## Overview

Fixture projects are complete txtx projects with:
- Solidity contracts (`src/`)
- Compiled artifacts (`out/`)
- Runbooks (`runbooks/`)
- Project configuration (`txtx.yml`, `foundry.toml`)

These projects can be used for integration tests that require:
- Contract compilation
- Complex deployment scenarios
- Multi-contract interactions
- Full project lifecycle testing

## Available Projects

### 1. simple-storage
**Location:** `src/tests/fixtures/foundry/`

**Contracts:**
- `SimpleStorage.sol` - Basic storage contract with struct, mapping, and array operations
- `Another.sol` - Additional contract for multi-contract scenarios

**Runbooks:**
- `simple-storage.tx` - Deploys SimpleStorage using CREATE2 and calls retrieve()

**Use Cases:**
- Testing `evm::get_contract_from_foundry_project()` function
- CREATE2 deployment with deterministic addresses
- Contract function calls
- Constructor argument handling

## Adding New Fixture Projects

To add a new fixture project:

1. Create a new directory under `src/tests/fixtures/`:
   ```
   src/tests/fixtures/my-project/
   ├── src/              # Solidity contracts
   ├── runbooks/         # Test runbooks
   ├── foundry.toml      # Foundry configuration
   └── txtx.yml          # Project configuration
   ```

2. Add Solidity contracts in `src/`

3. Compile contracts:
   ```bash
   cd src/tests/fixtures/my-project
   forge build
   ```

4. Create test runbooks in `runbooks/`

5. Configure `txtx.yml` with appropriate environments

## Using Fixture Projects in Tests

```rust
use crate::tests::project_test_harness::ProjectTestHarness;

#[test]
fn test_with_fixture_project() {
    let fixture_path = "src/tests/fixtures/foundry";
    let mut harness = ProjectTestHarness::new_from_fixture(
        fixture_path,
        "simple-storage.tx"
    );
    
    // Run the test
    let result = harness.run_tx();
    assert!(result.is_ok());
}
```

## Fixture Project Requirements

Each fixture project should:
1. Be self-contained with all necessary contracts
2. Include pre-compiled artifacts in `out/`
3. Have at least one runbook demonstrating the contracts
4. Use environment variables for sensitive data (API keys, etc.)
5. Document any special setup requirements

## Current Limitations

- Fixture projects currently require manual contract compilation
- The `ProjectTestHarness::new_from_fixture()` method needs to be implemented
- Environment variable handling in fixture projects needs refinement