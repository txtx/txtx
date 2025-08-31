# Error-Stack Architecture for txtx Addons

## Overview

This document describes the error-stack integration pattern for txtx addons, pioneered by the EVM addon. This pattern allows addons to use strongly-typed error-stack errors while maintaining compatibility with txtx-core's generic `Diagnostic` interface.

## Problem Statement

txtx-core uses a generic `Diagnostic` type for error reporting across all addons. However, modern error handling practices benefit from:
- Strongly-typed errors with rich context
- Error chains showing causality
- Structured error data for programmatic handling
- Better debugging through error-stack traces

The challenge is bridging these two approaches without breaking existing code.

## Solution: Error Preservation Through Any Trait

### Core Concept

The `Diagnostic` struct in txtx-addon-kit now includes an optional field that can store the original error:

```rust
pub struct Diagnostic {
    // ... existing fields ...
    /// Original error preserved for addons using error-stack
    pub source_error: Option<Box<dyn std::any::Any + Send + Sync>>,
}
```

This allows addons to:
1. Create rich, strongly-typed errors using error-stack
2. Convert them to `Diagnostic` for txtx-core compatibility
3. Preserve the original error for later extraction

### Error Flow

```
┌─────────────────┐
│   EVM Action    │
│  (e.g., send)   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   RPC Module    │──── Creates Report<EvmError>
│                 │     with full context
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Error Conversion│──── Converts to Diagnostic
│                 │     preserving Report in
│                 │     source_error field
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   txtx-core     │──── Works with Diagnostic
│                 │     (generic interface)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│     Tests       │──── Extract Report<EvmError>
│                 │     from Diagnostic for
│                 │     type-safe assertions
└─────────────────┘
```

## Implementation Pattern

### 1. Define Addon-Specific Error Types

```rust
#[derive(Debug, Clone)]
pub enum EvmError {
    Transaction(TransactionError),
    Rpc(RpcError),
    Contract(ContractError),
    // ... other variants
}

#[derive(Debug, Clone)]
pub enum TransactionError {
    InsufficientFunds { required: u128, available: u128 },
    InvalidNonce { expected: u64, actual: u64 },
    // ... other variants
}
```

### 2. Create Errors with Context

```rust
use error_stack::{Report, ResultExt};

// In RPC module
Err(Report::new(EvmError::Transaction(TransactionError::InsufficientFunds {
    required: calculated_cost,
    available: account_balance,
}))
.attach_printable(format!("Account {} has insufficient funds", address))
.attach(RpcContext {
    endpoint: self.url.to_string(),
    method: "eth_sendTransaction".to_string(),
    params: Some(format!("{:?}", tx)),
}))
```

### 3. Convert to Diagnostic While Preserving Original

```rust
impl From<EvmErrorReport> for Diagnostic {
    fn from(wrapper: EvmErrorReport) -> Self {
        let report = wrapper.0.clone();
        
        // Create diagnostic with human-readable message
        let mut diagnostic = Diagnostic::error_from_string(report.to_string());
        
        // Add detailed context for debugging
        diagnostic.documentation = Some(format!("{:?}", report));
        
        // Preserve the original Report<EvmError>
        diagnostic.source_error = Some(Box::new(report));
        
        diagnostic
    }
}
```

### 4. Extract Original Error in Tests

```rust
// In test code
let result = harness.execute_runbook();

if let Err(diagnostics) = result {
    // Try to extract the original error
    for diagnostic in &diagnostics {
        if let Some(report) = diagnostic.downcast_source::<Report<EvmError>>() {
            // Now we can make type-safe assertions
            match report.current_context() {
                EvmError::Transaction(TransactionError::InsufficientFunds { required, available }) => {
                    assert!(required > available);
                }
                _ => panic!("Expected insufficient funds error"),
            }
        }
    }
}
```

## Benefits

1. **Type Safety**: Strongly-typed errors within each addon
2. **Rich Context**: Full error-stack traces with attachments
3. **Backward Compatibility**: Existing code continues to work
4. **Gradual Migration**: Other addons can adopt this pattern incrementally
5. **Better Debugging**: Original errors available for inspection
6. **Test Precision**: Tests can assert on specific error types and values

## Migration Guide for Other Addons

To adopt this pattern in your addon:

1. **Define Error Types**: Create an enum hierarchy for your addon's errors
2. **Use error-stack**: Add `error-stack` as a dependency
3. **Create Wrapper Type**: Define a wrapper like `EvmErrorReport` for conversion
4. **Implement Conversion**: Convert your `Report<YourError>` to `Diagnostic`
5. **Preserve Original**: Store the report in `diagnostic.source_error`
6. **Update Tests**: Use `downcast_source()` to extract typed errors

## Example for a Hypothetical Stacks Addon

```rust
// Define errors
#[derive(Debug, Clone)]
pub enum StacksError {
    Clarity(ClarityError),
    Network(NetworkError),
}

// Wrapper for conversion
pub struct StacksErrorReport(pub Report<StacksError>);

// Conversion implementation
impl From<StacksErrorReport> for Diagnostic {
    fn from(wrapper: StacksErrorReport) -> Self {
        let report = wrapper.0.clone();
        let mut diagnostic = Diagnostic::error_from_string(report.to_string());
        diagnostic.source_error = Some(Box::new(report));
        diagnostic
    }
}

// Helper function
pub fn report_to_diagnostic(report: Report<StacksError>) -> Diagnostic {
    StacksErrorReport(report).into()
}
```

## Testing Considerations

### Unit Tests
- Test error creation with proper context
- Verify error conversion preserves information
- Check that downcasting works correctly

### Integration Tests
- Use real blockchain (e.g., Anvil for EVM)
- Verify errors are detected and reported correctly
- Assert on specific error types and values

### Example Test

```rust
#[test]
fn test_insufficient_funds_error() {
    let harness = TestHarness::new()
        .with_anvil()  // Start local blockchain
        .with_fixture("insufficient_funds.tx");
    
    let result = harness.execute_runbook();
    
    assert!(result.is_err());
    
    let diagnostic = result.unwrap_err();
    let report = diagnostic.downcast_source::<Report<EvmError>>()
        .expect("Should have EvmError");
    
    match report.current_context() {
        EvmError::Transaction(TransactionError::InsufficientFunds { .. }) => {
            // Test passes
        }
        _ => panic!("Wrong error type"),
    }
}
```

## Future Enhancements

1. **Type Registry**: Register error types for automatic deserialization
2. **Error Middleware**: Chain error transformers for common patterns
3. **Diagnostic Rendering**: Rich terminal output using error context
4. **Error Recovery**: Suggest fixes based on error types
5. **Cross-Addon Errors**: Standard error types for common failures

## Conclusion

This architecture provides a bridge between txtx-core's generic error handling and addon-specific strongly-typed errors. It maintains backward compatibility while enabling modern error handling practices, making debugging easier and tests more precise.