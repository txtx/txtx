# EVM Addon Error Handling Guide

This document describes the error handling patterns used in the EVM addon after the migration to `error-stack` v0.5.0.

## Overview

The EVM addon uses `error-stack` for comprehensive error handling, providing:
- Rich error context with full error chains
- Structured error types for different failure categories
- Actionable error messages for users
- Detailed debugging information for developers

## Error Type Hierarchy

```rust
pub enum EvmError {
    Transaction(TransactionError),    // Transaction building/sending failures
    Rpc(RpcError),                   // RPC communication errors
    Contract(ContractError),         // Smart contract interaction errors
    Verification(VerificationError), // Contract verification errors
    Codec(CodecError),              // Encoding/decoding errors
    Signer(SignerError),            // Key management errors
    Config(ConfigError),            // Configuration errors
}
```

## Usage Patterns

### 1. Creating Error-Stack Functions

When creating new functions that can fail, use `EvmResult<T>` as the return type:

```rust
use crate::errors::{EvmError, EvmResult};
use error_stack::{Report, ResultExt};

pub async fn my_function() -> EvmResult<String> {
    // Function implementation
    something_that_can_fail()
        .map_err(|e| Report::new(EvmError::Rpc(RpcError::NodeError(e.to_string()))))
        .attach_printable("Additional context about what was being attempted")?;
    
    Ok(result)
}
```

### 2. Handling Insufficient Funds

The addon specifically detects and reports insufficient funds errors with concrete amounts:

```rust
// When gas estimation fails due to insufficient funds, we calculate required amounts:
// required = (gas_price * estimated_gas_units) + transaction_value
//
// This provides users with actionable information:
// - Current balance (fetched from the network)
// - Estimated required amount (calculated based on gas price and estimated usage)
// - Clear suggestions for fixing the issue
```

### 3. Adding Context to Errors

Use `attach_printable` to add human-readable context:

```rust
operation()
    .attach_printable(format!("Building contract call to {} function {}", address, function))
    .attach_printable(format!("Function arguments: {:?}", args))?;
```

### 4. Converting from Old Error Handling

When migrating from string-based errors:

**Before:**
```rust
pub fn old_function() -> Result<Value, String> {
    something()
        .map_err(|e| format!("failed: {}", e))?;
    Ok(value)
}
```

**After:**
```rust
pub fn new_function() -> EvmResult<Value> {
    something()
        .map_err(|e| Report::new(EvmError::Rpc(RpcError::NodeError(e.to_string()))))
        .attach_printable("What was being attempted")?;
    Ok(value)
}
```

### 5. Preserving Error Context Through Layers

When errors pass through multiple layers, preserve the original error:

```rust
build_transaction()
    .await
    .map_err(|e| {
        // Check if it's a specific error we want to preserve
        let error_str = e.to_string();
        if error_str.contains("Insufficient funds") {
            error_str  // Preserve the original message
        } else {
            format!("Failed to build transaction: {}", error_str)
        }
    })?;
```

## Best Practices

### DO:
- ✅ Use specific error variants that match the failure type
- ✅ Add contextual information with `attach_printable`
- ✅ Provide actionable suggestions in error messages
- ✅ Calculate and show concrete values (e.g., required funds)
- ✅ Preserve error context through the call chain

### DON'T:
- ❌ Use generic error messages like "operation failed"
- ❌ Lose context by using `change_context` unnecessarily
- ❌ Convert to strings too early in the error chain
- ❌ Hide technical details that could help debugging

## Examples

### Example 1: RPC Operation with Retry

```rust
pub async fn get_nonce(&self, address: &Address) -> EvmResult<u64> {
    EvmRpc::retry_async(|| async {
        self.provider.get_transaction_count(address.clone())
            .await
            .map_err(|e| Report::new(EvmError::Rpc(RpcError::NodeError(e.to_string()))))
            .attach(RpcContext {
                endpoint: self.url.to_string(),
                method: "eth_getTransactionCount".to_string(),
                params: Some(format!("[\"{:?}\", \"pending\"]", address)),
            })
            .attach_printable(format!("Getting nonce for address {}", address))
    })
    .await
}
```

### Example 2: Contract Call with Full Context

```rust
pub async fn call_contract() -> EvmResult<TransactionRequest> {
    let (tx, cost, _) = build_unsigned_transaction_v2(rpc, values, common)
        .await
        .attach_printable(format!("Building contract call to {} function {}", 
            contract_address, function_name))
        .attach_printable(format!("Function arguments: {:?}", function_args))?;
    
    Ok(tx)
}
```

### Example 3: Handling Missing Configuration

```rust
let rpc_url = values
    .get_expected_string(RPC_API_URL)
    .map_err(|e| Report::new(EvmError::Config(ConfigError::MissingField(
        format!("rpc_api_url: {}", e)
    ))))?;
```

## Migration Status

### Completed ✅
- Core RPC module
- Transaction building (codec)
- Contract deployment (CREATE, CREATE2, Proxy)
- call_contract action
- send_eth action

### Pending 🔄
- sign_transaction action
- eth_call action (read-only calls)
- Additional context attachments

### Future Improvements 📋
- Add retry logic with exponential backoff
- Implement error recovery strategies
- Enhanced error categorization

## Testing Error Paths

When testing error handling:

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_insufficient_funds_detection() {
        // Test that insufficient funds errors are properly detected
        // and include required/available amounts
    }
    
    #[test]
    fn test_error_context_preservation() {
        // Test that error context flows through the call chain
    }
}
```

## Backward Compatibility

During migration, compatibility functions exist:

```rust
// Old interface (will be removed)
pub fn new_compat(url: &str) -> Result<Self, String> {
    Self::new(url).map_err(|e| e.to_string())
}

// New interface
pub fn new(url: &str) -> EvmResult<Self> {
    // Implementation with error-stack
}
```

These compatibility layers will be removed once the migration is complete.