# Error-Stack Demonstration Guide

## How to See the New Errors

Run the demonstration tests to see the rich error output from error-stack:

```bash
cd addons/evm

# Run all demos
cargo test errors_demo::demo_tests -- --nocapture

# Or run individual demos:
cargo test demo_transaction_insufficient_funds_error -- --nocapture
cargo test demo_contract_deployment_failure -- --nocapture
cargo test demo_rpc_connection_error -- --nocapture
cargo test demo_verification_error_with_context -- --nocapture
cargo test demo_error_comparison -- --nocapture
```

## Key Features Demonstrated

### 1. **Rich Error Context** 
The error-stack implementation shows:
- Full error chain with causality
- Contextual attachments at each level
- Human-readable suggestions
- Debug information with file locations

### 2. **Error Types**

#### Transaction Error Example:
```
Transaction error: Insufficient funds: required 1000000000000000000, available 500000000000000000
├╴Transaction requires 1 ETH but wallet only has 0.5 ETH
├╴Transaction: Send 1 ETH from 0x7474...7474 to 0x5f5f...5f5f
├╴Suggested action: Add more funds or reduce transaction amount
```

#### Contract Deployment Error Example:
```
Contract error: Contract deployment failed: Contract size exceeds maximum allowed (24KB > 24KB limit)
├╴Optimization suggestion: Enable optimizer in Solidity compiler
├╴Alternative: Split contract into multiple smaller contracts
```

#### RPC Connection Error Example:
```
RPC error: Failed to connect to RPC endpoint: http://localhost:8545
├╴Attempt 1/3: Connection refused
├╴Attempt 2/3: Connection refused
├╴Attempt 3/3: Connection refused
├╴Possible causes:
├╴  - Local node not running (try: geth --http)
├╴  - Incorrect port (default is 8545)
├╴  - Firewall blocking connection
```

### 3. **Comparison with Old Errors**

**Old String Error:**
```
Error: failed to send transaction: insufficient funds
```

**New Error-Stack Error:**
- Full error chain visible
- Contextual information attached
- Suggested actions included
- Type-safe error handling
- Zero-cost in release builds

## Benefits for Developers

1. **Faster Debugging**: Complete context at point of failure
2. **Better UX**: Actionable error messages with suggestions
3. **Type Safety**: Strongly typed errors vs string errors
4. **Performance**: Zero overhead in release builds
5. **Compatibility**: Seamless conversion to existing Diagnostic system

## Files

- `src/errors.rs` - Core error type definitions
- `src/errors_demo.rs` - Demonstration tests
- `src/codec/transaction_builder_refactored.rs` - Example refactored module
- `ERROR_STACK_MIGRATION.md` - Full migration guide