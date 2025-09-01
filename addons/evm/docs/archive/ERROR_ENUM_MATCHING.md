# Error Enum Matching in EVM Addon Tests

## Overview

The EVM addon test suite has been fully updated to use proper error enum variant matching instead of string-based error checking. This leverages the error-stack library's `Report<EvmError>` type system for more robust and maintainable error assertions.

## Update Status

✅ **Completed**: All test files now use error enum matching
- Integration tests: 6 files updated
- Unit tests: 3 files updated  
- Codec tests: 1 file updated
- Total: ~200 string contains() assertions replaced with type-safe matching

## Changes Made

### 1. Test Harness Update

The `ProjectTestHarness` now returns `Report<EvmError>` instead of `String` errors:

```rust
pub fn execute_runbook(&self) -> Result<TestResult, Report<EvmError>>
```

This preserves the rich error type information from the error-stack library.

### 2. Error Assertion Pattern

#### Before (String Matching):
```rust
if let Err(e) = result {
    assert!(
        e.contains("insufficient") || 
        e.contains("balance"),
        "Error should mention insufficient funds"
    );
}
```

#### After (Enum Variant Matching):
```rust
if let Err(report) = result {
    let is_insufficient_funds = matches!(
        report.current_context(),
        EvmError::Transaction(TransactionError::InsufficientFunds { .. })
    );
    assert!(
        is_insufficient_funds,
        "Expected TransactionError::InsufficientFunds, got: {:?}",
        report.current_context()
    );
}
```

## Error Types Available for Matching

### Transaction Errors
- `TransactionError::InsufficientFunds { required, available }`
- `TransactionError::InvalidNonce { expected, provided }`
- `TransactionError::GasEstimationFailed`
- `TransactionError::InvalidRecipient(String)`
- `TransactionError::SigningFailed`
- `TransactionError::BroadcastFailed`

### Codec Errors
- `CodecError::InvalidAddress(String)`
- `CodecError::InvalidHex(String)`
- `CodecError::AbiEncodingFailed(String)`
- `CodecError::AbiDecodingFailed(String)`
- `CodecError::FunctionNotFound { name }`
- `CodecError::ArgumentCountMismatch { expected, got }`

### Signer Errors
- `SignerError::KeyNotFound`
- `SignerError::InvalidPrivateKey`
- `SignerError::InvalidMnemonic`
- `SignerError::SignatureFailed`

### Contract Errors
- `ContractError::NotDeployed(Address)`
- `ContractError::FunctionNotFound(String)`
- `ContractError::ExecutionReverted(String)`
- `ContractError::DeploymentFailed(String)`

### RPC Errors
- `RpcError::ConnectionFailed(String)`
- `RpcError::RequestTimeout`
- `RpcError::InvalidResponse(String)`
- `RpcError::NodeError(String)`

## Benefits

1. **Type Safety**: Tests verify exact error types, preventing false positives
2. **Maintainability**: Error message changes don't break tests
3. **Clarity**: Expected errors are explicitly documented in test code
4. **Debugging**: Full error context available via `Report<EvmError>`
5. **Refactoring Safety**: Compiler ensures all error handling is updated

## Example Usage

### Testing Multiple Error Types
```rust
let is_gas_or_funds_error = matches!(
    report.current_context(),
    EvmError::Transaction(TransactionError::InsufficientFunds { .. }) |
    EvmError::Transaction(TransactionError::GasEstimationFailed)
);
```

### Extracting Error Details
```rust
if let EvmError::Transaction(TransactionError::InsufficientFunds { required, available }) = report.current_context() {
    println!("Required: {}, Available: {}", required, available);
}
```

## Files Updated

### Integration Tests
- `src/tests/integration/comprehensive_error_tests.rs` - Comprehensive error matching examples
- `src/tests/integration/error_handling_tests.rs` - Basic error handling patterns
- `src/tests/integration/insufficient_funds_tests.rs` - Fund-related error matching
- `src/tests/integration/transaction_tests.rs` - Transaction error matching
- `src/tests/integration/abi_decoding_tests.rs` - Codec error matching

### Unit Tests
- `src/tests/error_handling_tests.rs` - Error creation and formatting tests
- `src/tests/verification_error_tests.rs` - Verification error chain tests
- `src/codec/tests/abi_error_stack_tests.rs` - ABI encoding error tests

### Test Infrastructure
- `src/tests/test_harness/mod.rs` - Core test harness with `Report<EvmError>` support

## Testing Pattern

The recommended pattern for error testing is:

```rust
// 1. First verify the error type
let is_expected_error = matches!(
    report.current_context(),
    EvmError::Transaction(TransactionError::InsufficientFunds { required, available })
    if *required == 1000 && *available == 500
);
assert!(is_expected_error, "Expected InsufficientFunds, got: {:?}", report.current_context());

// 2. Then optionally verify message quality for UX
let error_str = report.to_string();
assert!(error_str.contains("helpful context"), "Error message should guide users");
```

## Migration Guide

When writing new tests or updating existing ones:

1. Import the error types:
```rust
use crate::errors::{EvmError, TransactionError, CodecError, SignerError};
```

2. Use `matches!` macro for assertions:
```rust
let is_expected_error = matches!(
    report.current_context(),
    EvmError::Transaction(TransactionError::SpecificError { .. })
);
assert!(is_expected_error, "Expected SpecificError, got: {:?}", report.current_context());
```

3. For debugging, the full error chain is available:
```rust
println!("Full error context: {:?}", report);
```

## Remaining String Checks

Some legitimate uses of `contains()` remain in the codebase:
- **Error message quality tests**: After verifying error type, checking that messages are helpful
- **Non-error assertions**: Checking output values, debug strings, etc.
- **Documentation**: Example code showing error handling patterns

## Integration with txtx Core

While the txtx-addon-kit still uses `Diagnostic` for errors, the EVM addon now:
1. Uses `Report<EvmError>` internally for rich error information
2. Implements `From<Report<EvmError>>` for `Diagnostic` for compatibility
3. Preserves error context in `Diagnostic::documentation` field

This allows the EVM addon to benefit from error-stack's features while maintaining compatibility with the broader txtx ecosystem.