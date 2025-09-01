# EVM Addon Architecture

## Overview

The txtx EVM addon provides comprehensive support for Ethereum and EVM-compatible blockchains. It uses modern Rust patterns, error-stack for rich error handling, and a fixture-based testing system.

## Core Components

### 1. Error Handling with error-stack

The EVM addon pioneered the error-stack integration pattern for txtx addons, providing rich error context while maintaining compatibility with txtx-core's `Diagnostic` interface.

#### Error Types
```rust
#[derive(Debug, Error)]
pub enum EvmError {
    #[error("RPC error")]
    Rpc,
    #[error("Transaction error")]
    Transaction,
    #[error("Contract error")]
    Contract,
    // ... other variants
}
```

#### Context Attachments
- **TransactionContext**: Transaction details, gas info, addresses
- **RpcContext**: RPC URL, method, chain ID
- **ContractContext**: Contract address, function, ABI
- **ConfigContext**: Configuration parameters

#### Conversion to Diagnostics
```rust
impl From<Report<EvmError>> for Diagnostic {
    fn from(report: Report<EvmError>) -> Self {
        // Preserves full error chain
        // Extracts context for detailed messages
        // Provides actionable error information
    }
}
```

### 2. Action System

Actions are the primary interface for blockchain operations:

```rust
pub struct SendEth;

impl Action for SendEth {
    fn run(&self, context: Context) -> Result<Value, Diagnostic> {
        // Validate inputs
        // Build transaction
        // Sign and send
        // Return result with full context
    }
}
```

#### Key Actions
- **Transaction Actions**: `send_eth`, `send_transaction`, `sign_transaction`
- **Contract Actions**: `deploy_contract`, `call_contract`, `eth_call`
- **Encoding Actions**: `encode_abi`, `decode_abi`
- **Utility Actions**: `check_confirmations`, `get_balance`

### 3. Contract Framework

#### Compilation Support
- **Foundry**: Primary framework with full Solidity support
- **Hardhat**: Alternative framework (planned)
- **Solc**: Direct compiler integration

#### Contract Management
```rust
pub struct CompiledContract {
    pub bytecode: Bytes,
    pub abi: JsonAbi,
    pub metadata: ContractMetadata,
}
```

### 4. RPC Layer

#### Connection Management
```rust
pub struct EvmRpc {
    provider: Provider,
    chain_id: u64,
    url: String,
}
```

#### Features
- Automatic retry logic
- Connection pooling
- Error context preservation
- Gas estimation

### 5. Transaction Building

The transaction builder provides a layered approach:

1. **Input Validation**: Type checking, address validation
2. **Gas Estimation**: Smart gas calculation with safety margins
3. **Signing**: Multiple signer support (keys, hardware, etc.)
4. **Submission**: Broadcasting with confirmation tracking

## Testing Infrastructure

### FixtureBuilder System

The fixture-based testing system provides:

```rust
let fixture = FixtureBuilder::new("test_name")
    .with_runbook("main", runbook_content)
    .with_parameter("key", "value")
    .build()
    .await?;

fixture.execute_runbook("main").await?;
```

### Components

1. **AnvilManager**: Singleton Anvil instance with snapshot/revert
2. **RunbookParser**: HCL parsing and output injection
3. **Executor**: Builds and runs txtx from source
4. **NamedAccounts**: 26 deterministic test accounts (alice-zed)

### Test Isolation

Each test gets its own Anvil snapshot:
```rust
// Test 1 starts from clean state
let handle1 = manager.get_handle("test1").await?;
// Makes changes...

// Test 2 starts from clean state
let handle2 = manager.get_handle("test2").await?;
// Isolated from Test 1's changes
```

## Design Patterns

### 1. Builder Pattern
Used extensively for configuration and test setup:
- `FixtureBuilder`
- `TransactionBuilder`
- `ContractBuilder`

### 2. Singleton Pattern
For shared resources:
- `AnvilManager` - One Anvil instance per test run
- `CompilerCache` - Cached compilation results

### 3. RAII Pattern
Resource management with Drop:
- `TestFixture` - Cleans up temp directories
- `AnvilInstance` - Kills process on drop

### 4. Error Context Pattern
Every error includes rich context:
```rust
.change_context(EvmError::Transaction)
.attach(TransactionContext { ... })
.attach_printable(format!("Failed to send {} wei", amount))
```

## Performance Considerations

### Optimization Strategies
1. **Compilation Caching**: Contracts compiled once per session
2. **Connection Pooling**: Reuse RPC connections
3. **Parallel Testing**: Tests run concurrently with isolation
4. **Lazy Initialization**: Resources created on-demand

### Resource Management
- Single Anvil instance for all tests
- Snapshot/revert instead of restart
- Temp directory cleanup on test completion
- PID tracking for process cleanup

## Security Considerations

### Input Validation
- Address checksum validation
- Amount overflow checking
- Gas limit boundaries
- ABI type validation

### Secret Management
- No secrets in error messages
- Secure key derivation
- Memory zeroization for sensitive data

## Future Enhancements

### Planned Features
1. Hardware wallet support
2. Advanced proxy patterns
3. L2 chain optimizations
4. Enhanced gas optimization

### API Stability
The public API (actions, signers, functions) is stable. Internal implementations may change between minor versions.