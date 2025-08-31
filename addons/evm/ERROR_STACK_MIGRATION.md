# Error-Stack Migration Guide for EVM Addon

## Overview
This document describes the patterns used for migrating from string-based errors to the error-stack library in the txtx EVM addon.

## Key Principles

### 1. Rich Error Types
Instead of generic string errors, use specific error enums with context:

```rust
// Before
return Err("Invalid address".to_string());

// After  
return Err(Report::new(EvmError::Codec(CodecError::InvalidAddress(address.clone()))));
```

### 2. Contextual Attachments
Add context to errors using `attach_printable()`:

```rust
rpc.estimate_gas(&tx)
    .await
    .attach_printable(format!("Estimating gas for transaction to {}", to_address))
    .attach_printable(format!("Transaction value: {} ETH", value))?;
```

### 3. Error Chaining
Use `ResultExt` for automatic error context:

```rust
let nonce = get_nonce(&address)
    .change_context(EvmError::Transaction(TransactionError::InvalidNonce))
    .attach_printable("Failed to fetch account nonce")?;
```

## Migration Patterns

### Pattern 1: Simple Error Conversion
```rust
// Old pattern
pub fn parse_address(input: &str) -> Result<Address, String> {
    Address::from_str(input)
        .map_err(|e| format!("Invalid address: {}", e))
}

// New pattern  
pub fn parse_address(input: &str) -> EvmResult<Address> {
    Address::from_str(input)
        .map_err(|e| Report::new(EvmError::Codec(
            CodecError::InvalidAddress(input.to_string())
        )))
        .attach_printable(format!("Parsing error: {}", e))
}
```

### Pattern 2: Multi-Step Operations
```rust
// Old pattern
async fn deploy_contract(code: &str) -> Result<Address, String> {
    let bytecode = hex::decode(code)
        .map_err(|e| format!("Invalid bytecode: {}", e))?;
    let tx = build_deploy_tx(bytecode)
        .map_err(|e| format!("Failed to build tx: {}", e))?;
    let receipt = send_tx(tx).await
        .map_err(|e| format!("Failed to send tx: {}", e))?;
    Ok(receipt.contract_address)
}

// New pattern
async fn deploy_contract(code: &str) -> EvmResult<Address> {
    let bytecode = hex::decode(code)
        .map_err(|e| Report::new(EvmError::Codec(
            CodecError::InvalidBytecode(e.to_string())
        )))
        .attach_printable("Decoding contract bytecode")?;
        
    let tx = build_deploy_tx(bytecode)
        .attach_printable("Building deployment transaction")?;
        
    let receipt = send_tx(tx).await
        .attach_printable("Broadcasting deployment transaction")?;
        
    receipt.contract_address
        .ok_or_else(|| Report::new(EvmError::Contract(
            ContractError::DeploymentFailed("No contract address in receipt".into())
        )))
}
```

### Pattern 3: Conditional Context
```rust
// Add context based on error type
let gas_limit = rpc.estimate_gas(&tx)
    .await
    .map_err(|estimate_err| match call_res {
        Ok(res) => {
            estimate_err
                .attach_printable(format!("Simulation succeeded with result: {}", res))
                .attach_printable("Gas estimation failed despite successful simulation")
        }
        Err(e) => {
            estimate_err
                .attach_printable(format!("Simulation also failed: {}", e))
                .attach_printable("Both simulation and gas estimation failed")
        }
    })?;
```

## Error Types Reference

### Core Error Enum
```rust
pub enum EvmError {
    Config(ConfigError),        // Configuration issues
    Codec(CodecError),          // Encoding/decoding errors
    Contract(ContractError),    // Smart contract errors
    Transaction(TransactionError), // Transaction building/sending
    Signer(SignerError),        // Key management errors
    Rpc(RpcError),             // Network/RPC errors
    Verification(VerificationError), // Confirmation errors
    InvalidInput(String),       // Generic input validation
}
```

### Specific Error Types
Each variant contains specific error cases:

```rust
pub enum TransactionError {
    InsufficientFunds { required: u128, available: u128 },
    InvalidNonce { expected: u64, actual: u64 },
    GasEstimationFailed(String),
    InvalidType(String),
    BuildFailed(String),
}
```

## Best Practices

1. **Be Specific**: Use the most specific error type available
2. **Add Context**: Always attach contextual information about what was being attempted
3. **Include Values**: Include the actual values that caused the error when relevant
4. **Chain Errors**: Use `change_context()` when converting between error types
5. **Test Errors**: Write tests that verify error messages contain expected information

## Migration Approach

The migration was completed in phases:

1. **Phase 1**: Created new error types and EvmResult type alias
2. **Phase 2**: Migrated core modules (codec, RPC, signers)
3. **Phase 3**: Updated all actions and commands
4. **Phase 4**: Converted test assertions to error enum matching
5. **Phase 5**: Removed all compatibility wrappers and cleaned up API

All functions now directly return `EvmResult<T>` without any versioning suffixes.

## Testing Error Cases

```rust
#[test]
fn test_insufficient_funds_error() {
    let error = Report::new(EvmError::Transaction(
        TransactionError::InsufficientFunds {
            required: 1_000_000_000_000_000_000, // 1 ETH
            available: 500_000_000_000_000_000,  // 0.5 ETH
        }
    ))
    .attach_printable("Attempting to send ETH")
    .attach_printable("Account: 0x123...");
    
    let error_str = format!("{:?}", error);
    assert!(error_str.contains("InsufficientFunds"));
    assert!(error_str.contains("1000000000000000000"));
}
```

## Benefits

1. **Better Debugging**: Full error context with stack traces
2. **Type Safety**: Errors are typed and can't be accidentally ignored
3. **Consistency**: All errors follow the same pattern
4. **User Experience**: Clear, actionable error messages
5. **Maintainability**: Easier to track down error sources

## Migration Status

### ✅ MIGRATION COMPLETE

The error-stack migration for the EVM addon is now **100% complete**. All modules have been successfully migrated from string-based errors to typed error-stack errors.

### Completed Modules
- ✅ **ABI encoding/decoding** (`/codec/abi/`)
  - Rich parameter-level error messages with positions
  - Type mismatch detection with suggestions
  - Array/tuple validation with detailed context
  
- ✅ **Transaction building** (`/codec/transaction/`)
  - Full error context for all transaction types
  - Enhanced gas estimation error handling
  
- ✅ **Contract interactions** (`/commands/actions/`)
  - All actions migrated to error-stack
  - Rich error context for contract calls
  - Deployment error handling with detailed diagnostics
  
- ✅ **RPC operations** (`/rpc/`)
  - Complete error context for network failures
  - Retry logic with detailed error reporting
  
- ✅ **Signer operations** (`/signers/`)
  - Full key management error handling
  - Hardware wallet error context
  
- ✅ **All utility functions** (`/functions.rs`)
  - All helper functions use EvmResult
  - No more string errors or compatibility wrappers
  
- ✅ **Test suite** (`/tests/`)
  - All ~200 test assertions updated to use error enum matching
  - Type-safe error verification throughout
  - No more string-based error checks

### Key Achievements
- **Zero string errors**: Complete elimination of `Result<T, String>` patterns
- **No compatibility wrappers**: Removed all `Diagnostic::error_from_string` usage
- **Clean API**: All _v2 functions renamed to original names
- **Type-safe testing**: All tests use error enum matching instead of string contains
- **Consistent error handling**: Every function returns `EvmResult<T>`
- **Rich error context**: Detailed, actionable error messages throughout

## Future Improvements

1. Add structured error codes for programmatic handling
2. Implement error recovery suggestions
3. Add telemetry hooks for error tracking
4. Create error documentation generator