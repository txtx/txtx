# EVM Addon Development Guide

## Overview

This guide covers development practices for contributing to the txtx EVM addon, including adding new actions, implementing features, and maintaining code quality.

## Code Organization

```
addons/evm/
├── src/
│   ├── commands/
│   │   ├── actions/        # Action implementations
│   │   ├── functions/      # Pure functions
│   │   └── signers/        # Signer implementations
│   ├── codec/              # Encoding/decoding logic
│   │   ├── abi/           # ABI encoding/decoding
│   │   └── transaction/    # Transaction building
│   ├── contracts/          # Contract management
│   ├── errors.rs          # Error types and handling
│   ├── rpc/               # RPC client implementation
│   └── tests/             # Test infrastructure
├── fixtures/              # Test fixtures
└── docs/                  # Documentation
```

## Adding a New Action

### 1. Define the Action Structure

```rust
// src/commands/actions/my_action.rs
use txtx_addon_kit::types::commands::Action;

pub struct MyAction;

impl Action for MyAction {
    fn name(&self) -> &str {
        "my_action"
    }
    
    fn description(&self) -> &str {
        "Description of what this action does"
    }
    
    fn parameters(&self) -> Vec<Parameter> {
        vec![
            Parameter::required("input_param", Type::String),
            Parameter::optional("optional_param", Type::Integer),
        ]
    }
}
```

### 2. Implement Action Logic

```rust
impl MyAction {
    pub fn run(args: Args) -> Result<Value, Report<EvmError>> {
        // Extract parameters with context
        let input = args.get_string("input_param")
            .change_context(EvmError::Config)
            .attach_printable("input_param is required")?;
        
        // Perform action logic
        let result = perform_operation(input)
            .change_context(EvmError::Transaction)
            .attach(TransactionContext {
                action: "my_action",
                details: format!("Processing {}", input),
            })?;
        
        // Return result
        Ok(Value::object(hashmap! {
            "result" => result,
            "timestamp" => Utc::now().timestamp(),
        }))
    }
}
```

### 3. Register the Action

```rust
// src/lib.rs
impl ProvideAction for EvmAddon {
    fn get_action(&self, name: &str) -> Option<Box<dyn Action>> {
        match name {
            "my_action" => Some(Box::new(MyAction)),
            // ... other actions
            _ => None,
        }
    }
}
```

### 4. Write Tests

```rust
// src/tests/integration/my_action_tests.rs
#[tokio::test]
async fn test_my_action() {
    let mut fixture = FixtureBuilder::new("test_my_action")
        .with_runbook("main", r#"
            addon "evm" { chain_id = 1 }
            
            action "test" "evm::my_action" {
                input_param = "test_value"
            }
            
            output "result" {
                value = action.test.result
            }
        "#)
        .build().await?;
    
    fixture.execute_runbook("main").await?;
    
    let result = fixture.get_output("main", "result");
    assert_eq!(result, expected_value);
}
```

## Error Handling Patterns

### Using error-stack

Always provide rich context for errors:

```rust
use error_stack::{Report, ResultExt};

fn process_transaction(tx: Transaction) -> Result<Receipt, Report<EvmError>> {
    // Change context to appropriate error type
    validate_transaction(&tx)
        .change_context(EvmError::Transaction)?;
    
    // Attach structured context
    send_transaction(tx)
        .change_context(EvmError::Rpc)
        .attach(TransactionContext {
            from: tx.from,
            to: tx.to,
            value: tx.value,
            gas: tx.gas,
        })
        .attach_printable(format!("Failed to send transaction"))?;
    
    Ok(receipt)
}
```

### Error Context Types

Use appropriate context attachments:

- **TransactionContext**: For transaction-related errors
- **RpcContext**: For RPC communication errors
- **ContractContext**: For contract interaction errors
- **ConfigContext**: For configuration/validation errors

### Converting to Diagnostics

Errors automatically convert to Diagnostics for txtx:

```rust
impl From<Report<EvmError>> for Diagnostic {
    fn from(report: Report<EvmError>) -> Self {
        // Automatic conversion with full context preservation
    }
}
```

## Contract Integration

### Adding Contract Support

1. **Add Contract Source**
   ```rust
   // src/contracts/templates/MyContract.sol
   pragma solidity ^0.8.0;
   
   contract MyContract {
       // Contract implementation
   }
   ```

2. **Compile Integration**
   ```rust
   // src/contracts/mod.rs
   pub fn compile_my_contract() -> Result<CompiledContract> {
       let source = include_str!("templates/MyContract.sol");
       compile_contract("MyContract", source)
   }
   ```

3. **Create Deployment Action**
   ```rust
   pub struct DeployMyContract;
   
   impl Action for DeployMyContract {
       fn run(args: Args) -> Result<Value> {
           let compiled = compile_my_contract()?;
           let address = deploy_contract(compiled, args)?;
           Ok(Value::string(address))
       }
   }
   ```

## Testing Best Practices

### Test Structure

Follow ARRANGE/ACT/ASSERT pattern:

```rust
#[tokio::test]
async fn test_feature() {
    // ARRANGE: Set up test environment
    let fixture = create_test_fixture().await;
    
    // ACT: Execute the operation
    let result = perform_operation(&fixture).await;
    
    // ASSERT: Verify the results
    assert_eq!(result, expected);
}
```

### Test Data

Use deterministic test data:

```rust
// Use named accounts for consistency
let alice = accounts.alice.address_string();
let bob = accounts.bob.address_string();

// Use specific values for reproducibility
let amount = "1000000000000000000"; // Exactly 1 ETH
let gas_limit = 21000;
```

### Error Testing

Test both success and failure cases:

```rust
#[tokio::test]
async fn test_error_handling() {
    // Test expected failure
    let result = operation_that_should_fail().await;
    assert!(result.is_err());
    
    // Verify error message
    let error = result.unwrap_err();
    assert!(error.to_string().contains("expected error"));
}
```

## Performance Guidelines

### Optimization Tips

1. **Cache Compiled Contracts**
   ```rust
   lazy_static! {
       static ref COMPILED_CACHE: Mutex<HashMap<String, CompiledContract>> = 
           Mutex::new(HashMap::new());
   }
   ```

2. **Reuse RPC Connections**
   ```rust
   // Use connection pooling
   let provider = Provider::new_with_client(client);
   ```

3. **Batch Operations**
   ```rust
   // Batch multiple calls
   let results = provider.batch_request(requests).await?;
   ```

### Resource Management

- Use RAII patterns for cleanup
- Implement Drop for resources
- Track process IDs for external processes

## Code Style

### Formatting

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt --check
```

### Linting

```bash
# Run clippy
cargo clippy --all-targets --all-features

# Fix clippy warnings
cargo clippy --fix
```

### Documentation

Document all public APIs:

```rust
/// Sends ETH from one address to another.
/// 
/// # Arguments
/// * `from` - Sender address
/// * `to` - Recipient address
/// * `amount` - Amount in wei
/// 
/// # Returns
/// Transaction hash on success
pub fn send_eth(from: Address, to: Address, amount: U256) -> Result<H256> {
    // Implementation
}
```

## Debugging

### Enable Debug Logging

```bash
RUST_LOG=debug cargo test test_name -- --nocapture
```

### Use Debug Assertions

```rust
debug_assert!(condition, "Condition failed: {}", details);
```

### Trace Execution

```rust
use tracing::{debug, info, warn, error};

#[instrument]
fn process_transaction(tx: Transaction) {
    debug!("Processing transaction: {:?}", tx);
    // ...
}
```

## Contributing

### Process

1. Fork the repository
2. Create a feature branch
3. Make changes with tests
4. Run test suite
5. Submit pull request

### Checklist

Before submitting PR:

- [ ] Tests pass: `cargo test --package txtx-addon-network-evm`
- [ ] Code formatted: `cargo fmt`
- [ ] No clippy warnings: `cargo clippy`
- [ ] Documentation updated
- [ ] CHANGELOG entry added

### Commit Messages

Follow conventional commits:

```
feat(evm): add new action for token transfers
fix(evm): correct gas estimation for complex calls
docs(evm): update testing guide
test(evm): add integration tests for CREATE2
refactor(evm): simplify transaction building
```

## Maintenance

### Updating Dependencies

```bash
# Update Cargo.toml
cargo update

# Test with new dependencies
cargo test
```

### Breaking Changes

When making breaking changes:

1. Document in CHANGELOG
2. Update migration guide
3. Bump major version
4. Notify downstream users

### Deprecation

Mark deprecated features:

```rust
#[deprecated(since = "0.5.0", note = "Use new_function instead")]
pub fn old_function() {
    // ...
}
```