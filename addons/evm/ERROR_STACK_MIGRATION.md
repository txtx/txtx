# Error-Stack Migration Guide for EVM Addon

## Overview

This guide documents the migration from mixed error handling (String/Diagnostic) to the unified error-stack approach in the EVM addon.

## Benefits Demonstrated

### 1. Rich Error Context
- Automatic stack traces with context at each error propagation point
- Structured error types instead of string concatenation
- Machine-readable error chains for better debugging

### 2. Performance Improvements
- Zero-cost abstractions in release builds
- Stack traces only in debug mode
- More efficient than 236 `.map_err(|e|` string conversions

### 3. Better Developer Experience
```rust
// Before: Opaque string errors
rpc.get_nonce(&from).await.map_err(|e| e.to_string())?

// After: Rich context with error-stack
rpc.get_nonce(&from)
    .await
    .map_err(|e| Report::new(EvmError::Rpc(RpcError::NodeError(e.to_string()))))
    .attach(RpcContext {
        endpoint: rpc.get_endpoint(),
        method: "eth_getTransactionCount".to_string(),
        params: Some(format!("[\"{:?}\", \"pending\"]", from)),
    })
    .attach_printable(format!("Fetching nonce for address {}", from))?
```

## Migration Pattern

### Step 1: Replace Result Types
```rust
// Before
fn process() -> Result<Value, String>
fn process() -> Result<Value, Diagnostic>

// After
use crate::errors::EvmResult;
fn process() -> EvmResult<Value>
```

### Step 2: Convert Error Creation
```rust
// Before
Err(diagnosed_error!("invalid address: {}", addr))

// After
Err(Report::new(EvmError::Codec(CodecError::InvalidAddress(addr))))
    .attach_printable("Validating transaction address")
```

### Step 3: Add Context at Boundaries
```rust
// At RPC calls
.attach(RpcContext { endpoint, method, params })

// At transaction operations
.attach(TransactionContext { tx_hash, from, to, value, gas_limit, chain_id })

// At contract calls
.attach(ContractContext { address, function, args })
```

## Compatibility Layer

The `From<Report<EvmError>> for Diagnostic` implementation ensures backward compatibility:

```rust
// Automatic conversion at API boundaries
let diagnostic: Diagnostic = error_stack_result.unwrap_err().into();
```

## Migration Priority

### High Priority (196 instances)
- All `diagnosed_error!` macro usages
- Transaction building functions
- RPC communication layer

### Medium Priority (71 instances)
- Functions returning `Result<_, String>`
- Codec/encoding functions
- Signer implementations

### Low Priority
- Test utilities
- Example code
- Documentation

## Testing Strategy

1. **Unit Tests**: Verify error context propagation
2. **Integration Tests**: Ensure Diagnostic conversion works
3. **Performance Tests**: Confirm no regression in release builds

## Rollout Plan

### Phase 1: Core Infrastructure ✅
- [x] Add error-stack dependency
- [x] Create errors module
- [x] Implement conversion layer

### Phase 2: Critical Path (Next)
- [ ] Migrate transaction building
- [ ] Migrate RPC module
- [ ] Migrate signers

### Phase 3: Full Migration
- [ ] Convert all error sites
- [ ] Remove string error returns
- [ ] Update documentation

## Success Metrics

1. **Error Quality**: Detailed error reports with full context
2. **Developer Velocity**: Faster debugging with stack traces
3. **Performance**: No regression in release builds
4. **Compatibility**: Zero breaking changes for consumers

## Decision Points

After Phase 2 completion, evaluate:
- Developer feedback on error quality
- Performance impact measurements
- Integration complexity

Based on results, either:
1. **Proceed**: Roll out to entire txtx project
2. **Adjust**: Modify approach based on learnings
3. **Abandon**: Revert if benefits don't justify complexity

## Example Error Output

### Before (String Error)
```
Error: failed to parse 'from' address: invalid address format
```

### After (Error-Stack)
```
Error: Transaction error: Invalid recipient address: 0xinvalid

Caused by:
  0: Parsing 'from' address for transaction
  1: Invalid address format: missing 0x prefix
  
Context:
  - RPC Endpoint: http://localhost:8545
  - Method: eth_sendTransaction
  - Chain ID: 1
  - From: 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb
  - Value: 1000000000000000000 wei
```

## Resources

- [error-stack documentation](https://docs.rs/error-stack/0.5.0)
- `addons/evm/src/errors.rs` - Error type definitions
- `addons/evm/src/codec/transaction_builder_refactored.rs` - Example implementation
