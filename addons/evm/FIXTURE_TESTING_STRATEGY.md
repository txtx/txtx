# EVM Fixture Testing Strategy

## Overview

This document outlines a comprehensive testing strategy for the EVM addon that provides:
- Efficient test execution using Anvil snapshots/reverts
- Automatic output generation based on runbook parsing
- Named test accounts for easy reference
- Template-based test fixtures
- Confirmation handling for testing blockchain finality

## Core Components

### 1. Named Test Accounts

Instead of dealing with raw addresses and private keys, we provide 26 named accounts derived from a deterministic mnemonic:

```rust
pub struct NamedAccounts {
    pub alice: TestAccount,
    pub bob: TestAccount,
    pub charlie: TestAccount,
    pub david: TestAccount,
    pub eve: TestAccount,
    pub frank: TestAccount,
    pub grace: TestAccount,
    pub heidi: TestAccount,
    pub ivan: TestAccount,
    pub judy: TestAccount,
    pub karen: TestAccount,
    pub larry: TestAccount,
    pub mallory: TestAccount,
    pub nancy: TestAccount,
    pub oscar: TestAccount,
    pub peggy: TestAccount,
    pub quincy: TestAccount,
    pub robert: TestAccount,
    pub sybil: TestAccount,
    pub trent: TestAccount,
    pub ursula: TestAccount,
    pub victor: TestAccount,
    pub walter: TestAccount,
    pub xavier: TestAccount,
    pub yvonne: TestAccount,
    pub zed: TestAccount,
}
```

Usage in runbooks:
```hcl
signer "alice" "evm::secret_key" {
    secret_key = input.alice_secret  # Automatically provided
}

action "transfer" "evm::send_eth" {
    from = input.alice_address
    to = input.bob_address
    amount = "1000000000000000000"
    signer = signer.alice
}
```

Usage in tests:
```rust
let fixture = FixtureBuilder::new("test_transfer")
    .with_template("basic")
    .build().await?;

// Accounts are automatically available
assert_eq!(fixture.accounts.alice.address, "0x70997970C51812dc3A010C7d01b50e0d17dc79C8");
```

### 2. Anvil Pool with Snapshot/Revert

Single Anvil instance with snapshot/revert for test isolation:

```rust
pub struct AnvilPool {
    instance: AnvilInstance,
    snapshots: HashMap<String, String>,  // test_name -> snapshot_id
}

// Each test gets isolated state
async fn test_scenario() {
    let mut pool = AnvilPool::shared().await;
    let handle = pool.get_handle("test_name").await?;
    
    // Test runs with clean state
    // Automatic revert on drop
}
```

Key features:
- **Efficiency**: Single Anvil process for all tests
- **Isolation**: Each test starts from clean snapshot
- **Speed**: Snapshot/revert is much faster than process restart
- **Confirmations**: Built-in block mining for confirmation testing

### 3. Intelligent Output Generation

Leverage txtx's parsing to automatically generate comprehensive outputs:

```rust
pub struct RunbookParser {
    content: String,
    parsed: ParsedRunbook,
}

impl RunbookParser {
    pub fn parse(content: &str) -> Result<Self> {
        // Use txtx's parser to understand the runbook structure
        let parsed = txtx_core::parser::parse_runbook(content)?;
        Ok(Self { content: content.to_string(), parsed })
    }
    
    pub fn extract_actions(&self) -> Vec<ActionInfo> {
        // Extract all actions from the parsed runbook
    }
    
    pub fn generate_test_outputs(&self) -> String {
        // Generate comprehensive output blocks based on actions
    }
}
```

Generated output structure:
```hcl
// Automatically generated for each action
output "deploy_token_output" {
    value = {
        tx_hash = action.deploy_token.tx_hash
        contract_address = action.deploy_token.contract_address
        logs = action.deploy_token.logs
        gas_used = action.deploy_token.gas_used
    }
}

// Aggregate test output
output "test_output" {
    value = {
        actions = {
            deploy_token = output.deploy_token_output.value
            transfer = output.transfer_output.value
        }
        accounts = {
            alice_balance = evm::get_balance(input.alice_address)
            bob_balance = evm::get_balance(input.bob_address)
        }
        metadata = {
            block_number = evm::get_block_number()
            timestamp = evm::get_block_timestamp()
        }
    }
}
```

### 4. Template System

Pre-built templates for common test scenarios:

```
fixtures/templates/
├── basic/
│   ├── txtx.yml.tmpl
│   ├── runbooks/
│   │   └── main.tx.tmpl
│   └── config.toml
├── defi/
│   ├── contracts/
│   │   ├── Token.sol
│   │   └── DEX.sol
│   └── runbooks/
│       ├── deploy.tx.tmpl
│       └── interact.tx.tmpl
└── nft/
    └── ...
```

### 5. Confirmation Testing

Built-in support for testing with confirmations:

```rust
impl TestFixture {
    pub async fn execute_with_confirmations(&mut self, runbook: &str, confirmations: u32) -> Result<()> {
        // Execute runbook
        self.execute_runbook(runbook).await?;
        
        // Mine blocks
        self.anvil.mine_blocks(confirmations).await?;
        
        // Verify confirmations were processed
        Ok(())
    }
}
```

## Implementation Plan

### Phase 1: Core Infrastructure (Week 1)
- [x] Document testing strategy
- [ ] Implement NamedAccounts with deterministic derivation
- [ ] Create AnvilPool with snapshot/revert
- [ ] Build RunbookParser using txtx-core

### Phase 2: Output Generation (Week 1-2)
- [ ] Parse runbooks to extract actions
- [ ] Generate action-specific outputs
- [ ] Create aggregate test output
- [ ] Add metadata collection

### Phase 3: Fixture System (Week 2)
- [ ] Implement FixtureBuilder
- [ ] Create template loading system
- [ ] Add parameter substitution
- [ ] Build test execution flow

### Phase 4: Test Helpers (Week 2-3)
- [ ] Create assertion utilities
- [ ] Add event extraction
- [ ] Implement checkpoint/restore
- [ ] Build confirmation helpers

### Phase 5: Templates & Documentation (Week 3)
- [ ] Create basic template
- [ ] Create DeFi template
- [ ] Create NFT template
- [ ] Write comprehensive docs
- [ ] Add example tests

## Usage Examples

### Basic Test
```rust
#[tokio::test]
async fn test_simple_transfer() {
    let mut fixture = TestFixture::new("simple_transfer").await?;
    
    // Alice and Bob accounts are automatically available
    fixture.execute_runbook("transfer").await?;
    
    // Check the auto-generated outputs
    assert!(fixture.get_output("transfer_output.tx_hash").is_some());
    assert_eq!(
        fixture.get_output("test_output.accounts.alice_balance"),
        Some(Value::String("9000000000000000000"))  // 9 ETH after sending 1
    );
}
```

### DeFi Scenario Test
```rust
#[tokio::test]
async fn test_defi_scenarios() {
    let mut fixture = TestFixture::new("defi")
        .with_template("defi")
        .build().await?;
    
    // Deploy contracts
    fixture.execute_runbook("deploy").await?;
    
    // Create checkpoint
    let checkpoint = fixture.checkpoint().await?;
    
    // Scenario 1: Add liquidity
    fixture.execute_runbook("add_liquidity").await?;
    assert!(fixture.action_succeeded("add_liquidity"));
    
    // Revert for scenario 2
    fixture.restore(checkpoint).await?;
    
    // Scenario 2: Test slippage
    fixture.execute_runbook("test_slippage").await?;
}
```

### Confirmation Test
```rust
#[tokio::test]
async fn test_with_confirmations() {
    let mut fixture = TestFixture::new("confirmations").await?;
    
    // Deploy with 6 confirmations
    fixture.execute_with_confirmations("deploy", 6).await?;
    
    // Verify deployment was confirmed
    assert_eq!(
        fixture.get_output("deploy_output.confirmations"),
        Some(Value::Integer(6))
    );
}
```

## Benefits

1. **Readable Tests**: Named accounts make tests self-documenting
2. **Fast Execution**: Snapshot/revert instead of process restarts
3. **Automatic Outputs**: No manual output block writing
4. **Type Safety**: Strongly typed account access
5. **Isolation**: Each test runs in clean state
6. **Debugging**: Failed tests preserve their state
7. **Confirmation Testing**: Built-in block mining support

## Configuration

### Test Configuration File
```toml
# test.toml
[anvil]
mnemonic = "test test test test test test test test test test test junk"
port = 8545
chain_id = 31337

[defaults]
confirmations = 0
gas_price = "20000000000"
gas_limit = "3000000"

[accounts]
initial_balance = "10000"  # ETH per account
```

## Debugging

When a test fails:
1. The test directory is preserved in `fixtures/outputs/<test_name>/`
2. Anvil state can be inspected via snapshot
3. Output JSON files show all action results
4. Runbook with injected outputs is saved for inspection

## Best Practices

1. **Use Named Accounts**: Always use alice, bob, etc. instead of raw addresses
2. **Checkpoint Often**: Take snapshots before complex operations
3. **Test in Isolation**: Each test should be independent
4. **Verify Outputs**: Check auto-generated outputs for completeness
5. **Handle Confirmations**: Test with various confirmation counts
6. **Clean Up**: Let the framework handle cleanup automatically