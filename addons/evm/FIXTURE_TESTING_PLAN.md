# EVM Fixture-Based Testing System

## Overview

A comprehensive testing framework that leverages txtx's parsing capabilities to automatically augment runbooks with test outputs, uses Anvil's snapshot/revert for test isolation, and provides a clean API for writing tests.

## Architecture

### 1. Core Components

```
┌─────────────────────────────────────────────────────────────┐
│                         Test Runner                          │
│  ┌────────────┐  ┌─────────────┐  ┌──────────────────┐    │
│  │ AnvilPool  │  │FixtureBuilder│ │ Output Augmenter  │    │
│  │            │  │              │  │                   │    │
│  │ Single     │  │ Template     │  │ Parse runbook     │    │
│  │ Instance   │──│ System       │──│ Extract actions   │    │
│  │ Snapshot/  │  │              │  │ Inject outputs    │    │
│  │ Revert     │  │              │  │                   │    │
│  └────────────┘  └─────────────┘  └──────────────────┘    │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
                    ┌──────────────────┐
                    │   Test Fixture    │
                    │                   │
                    │ - Execute runbook │
                    │ - Mine blocks     │
                    │ - Read outputs    │
                    │ - Assert results  │
                    └──────────────────┘
```

### 2. Anvil Pool Management

**Key Features:**
- Single Anvil instance shared across tests
- Snapshot/revert for test isolation
- Block mining for confirmations
- No process bouncing

```rust
pub struct AnvilPool {
    instance: AnvilInstance,
    snapshots: HashMap<String, String>,  // test_name -> snapshot_id
}

// Each test gets a handle with its own snapshot
pub struct AnvilHandle {
    snapshot_id: String,
    url: String,
    accounts: Vec<TestAccount>,
}
```

### 3. Output Augmentation System

**Automatic Output Injection:**
- Parse runbook to extract all actions
- Generate appropriate output blocks based on action types
- Inject both individual and aggregated outputs

```hcl
# Original runbook
action "deploy_token" "evm::deploy_contract" {
    contract = evm::get_contract_from_foundry_project("Token")
    signer = signer.deployer
}

# Auto-injected outputs
output "deploy_token_output" {
    value = {
        tx_hash = action.deploy_token.tx_hash
        contract_address = action.deploy_token.contract_address
        logs = action.deploy_token.logs
        gas_used = action.deploy_token.gas_used
    }
}

output "test_output" {
    value = {
        actions = {
            deploy_token = output.deploy_token_output.value
        }
        environment = {
            chain_id = addon.evm.chain_id
            block_number = evm::get_block_number()
        }
    }
}
```

### 4. Template System

```
fixtures/
├── templates/
│   ├── foundry-basic/
│   │   ├── txtx.yml.tmpl
│   │   ├── src/
│   │   │   └── {{contract_name}}.sol.tmpl
│   │   ├── runbooks/
│   │   │   └── deploy.tx.tmpl
│   │   └── foundry.toml
│   └── foundry-defi/
│       └── ...
└── outputs/           # Test execution outputs (gitignored)
    ├── test_simple_deployment/
    │   └── runs/
    │       └── testing/
    │           └── deploy_2025-08-31--16-00-07.output.json
    └── test_complex_scenario/
```

## Implementation Plan

### Phase 1: Core Infrastructure ✅ (Week 1)

1. **AnvilPool with Snapshot/Revert**
   - [x] Single Anvil instance management
   - [x] Snapshot/revert RPC calls
   - [x] Block mining for confirmations
   - [x] Test isolation via snapshots

2. **Runbook Parser Integration**
   - [x] Parse runbook to extract actions
   - [x] Identify action types and names
   - [x] Generate appropriate output structures

3. **Output Augmenter**
   - [x] Auto-inject individual action outputs
   - [x] Auto-inject aggregated test output
   - [x] Handle different action types (deploy, call, send_eth)

### Phase 2: Template System (Week 2)

1. **Template Engine**
   - [ ] Handlebars-style variable substitution
   - [ ] Template validation
   - [ ] Pre-built templates for common scenarios

2. **Fixture Builder**
   - [ ] Load and process templates
   - [ ] Parameter substitution
   - [ ] Contract and runbook injection

### Phase 3: Test Execution (Week 3)

1. **Test Fixture Runtime**
   - [ ] Execute runbooks via txtx CLI
   - [ ] Parse output JSON from runs/testing/
   - [ ] Provide assertion helpers
   - [ ] Checkpoint/restore for scenarios

2. **Test Utilities**
   - [ ] Event extraction and parsing
   - [ ] Gas tracking
   - [ ] Balance checking

### Phase 4: Developer Experience (Week 4)

1. **Test Macros**
   - [ ] `#[fixture_test]` attribute macro
   - [ ] Parametrized test support
   - [ ] Automatic setup/teardown

2. **Documentation & Examples**
   - [ ] Comprehensive guide
   - [ ] Example tests for common patterns
   - [ ] Template creation guide

## Usage Examples

### Simple Test

```rust
#[tokio::test]
async fn test_token_deployment() {
    let mut fixture = FixtureBuilder::new("token_deployment")
        .with_template("foundry-basic")
        .with_parameter("initial_supply", "1000000")
        .build()
        .await?;
    
    fixture.execute_runbook("deploy").await?;
    
    // Auto-generated outputs make assertions easy
    assert!(fixture.get_tx_hash("deploy_token").is_some());
    assert!(fixture.get_contract_address("deploy_token").is_some());
    
    let logs = fixture.get_logs("deploy_token");
    assert_eq!(logs[0].name, "Transfer");
}
```

### Test with Confirmations

```rust
#[tokio::test]
async fn test_with_confirmations() {
    let mut fixture = FixtureBuilder::new("confirmations_test")
        .with_template("foundry-basic")
        .with_confirmations(6)  // Auto-mines 6 blocks
        .build()
        .await?;
    
    fixture.execute_runbook("deploy").await?;
    
    // Confirmations were automatically handled
    assert!(fixture.get_output("deploy", "deploy_output")
        .unwrap()
        .get_path("confirmed")
        .unwrap()
        .as_bool()
        .unwrap());
}
```

### Scenario Testing with Snapshots

```rust
#[tokio::test]
async fn test_multiple_scenarios() {
    let mut fixture = FixtureBuilder::new("scenarios")
        .with_template("foundry-defi")
        .build()
        .await?;
    
    // Setup initial state
    fixture.execute_runbook("setup").await?;
    let checkpoint = fixture.checkpoint().await?;
    
    // Scenario 1
    fixture.execute_runbook("happy_path").await?;
    fixture.assert_all_successful();
    
    // Revert to checkpoint
    fixture.restore(checkpoint.clone()).await?;
    
    // Scenario 2 with clean state
    fixture.execute_runbook("edge_case").await?;
    fixture.assert_output("test_output.edge_case_handled", Value::Bool(true));
}
```

## Key Benefits

1. **Efficiency**: Single Anvil instance with snapshot/revert instead of process bouncing
2. **Automation**: Automatic output generation from parsed runbooks
3. **Isolation**: Each test gets clean state via snapshots
4. **Flexibility**: Templates for common patterns, custom runbooks for unique tests
5. **Debugging**: Test outputs preserved in named directories
6. **Confirmations**: Automatic block mining when needed
7. **Type Safety**: Leverages txtx's parsing for correct output structure

## File Organization

```
addons/evm/
├── src/
│   └── tests/
│       ├── fixture_system/
│       │   ├── mod.rs           # Main module
│       │   ├── anvil_pool.rs    # Anvil management
│       │   ├── augmenter.rs     # Output injection
│       │   ├── builder.rs       # Fixture builder
│       │   ├── runtime.rs       # Test execution
│       │   └── templates.rs     # Template engine
│       └── fixtures/
│           ├── deployment_tests.rs
│           ├── defi_tests.rs
│           └── error_tests.rs
├── fixtures/
│   ├── templates/               # Reusable templates
│   └── outputs/                 # Test outputs (gitignored)
└── FIXTURE_TESTING_GUIDE.md    # User documentation
```

## Configuration

```yaml
# test_config.yml
anvil:
  pool_size: 1              # Single instance with snapshots
  default_port: 8545
  mnemonic: "test test..." # Deterministic accounts

defaults:
  confirmations: 0
  environment: "testing"
  preserve_on_failure: true

templates:
  search_paths:
    - "fixtures/templates"
    - "fixtures/custom"
```

## Next Steps

1. Implement AnvilPool with snapshot/revert ✅
2. Create runbook parser and output augmenter ✅
3. Build fixture runtime with txtx CLI integration
4. Create initial templates
5. Write example tests
6. Document patterns and best practices