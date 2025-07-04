# Error Reporting Comparison: Before vs After

## Scenario: Deploying a contract with an invalid address

### 🔴 OLD: Using Diagnostic

```
x unable to parse address: invalid-address
```

That's it. The user has to guess:
- What address format is expected?
- Where in the code did this happen?
- What action was being performed?
- How to fix it?

### 🟢 NEW: Using error-stack

```
Execution failed
├╴at deploy.tx:15:10
├╴Failed to deploy contract 'MyContract'
├╴Action: deploy_contract (evm::construct_12345)
│
╰─▶ Validation failed
    ├╴Invalid Ethereum address: invalid-address
    ├╴Documentation: Ethereum addresses must start with '0x' and be 42 characters total
    ├╴Example: address = "0x742d35Cc6634C0532925a3b844Bc9e7595f89590"
    ╰╴Link: https://docs.txtx.io/ethereum/addresses
```

The new error provides:
- ✅ Exact location (file:line:column)
- ✅ Clear context (deploying contract 'MyContract')
- ✅ Action details (namespace and construct ID)
- ✅ Root cause (validation failed)
- ✅ Helpful documentation
- ✅ Working example
- ✅ Link to more information

## Scenario: Insufficient funds for transaction

### 🔴 OLD: Using Diagnostic

```
x unable to send transaction: insufficient funds
```

### 🟢 NEW: Using error-stack

```
Insufficient funds for operation
├╴at transaction.tx:25:8
├╴Account 0x123...789 has insufficient funds
├╴Transaction from 0x123...789 to 0x456...012 for 100.00 ETH
│
├╴Account Balance:
│  Address: 0x123...789
│  Current: 50.00 ETH
│  Required: 100.00 ETH
│
╰╴Help: Your account doesn't have enough funds for this transaction
   Suggestion: Send at least 50.00 ETH to 0x123...789 to proceed
```

## Key Improvements

1. **Context Preservation**: Errors maintain full context as they propagate
2. **Structured Attachments**: Type-safe additional information
3. **Actionable Guidance**: Clear steps to resolve issues
4. **Debugging Support**: File locations and stack traces
5. **Rich Formatting**: Hierarchical display of error chains

## How to See These Errors

1. **Run the demo**:
   ```bash
   cargo run --example error_stack_demo --package txtx-addon-kit
   ```

2. **Run the tests with output**:
   ```bash
   cargo test --package txtx-addon-kit errors::demo -- --nocapture
   ```

3. **In actual usage**, errors would appear in:
   - CLI output when commands fail
   - Log files with full context
   - Web UI with formatted display

## Developer Experience

### Creating errors with rich context:

```rust
// Old way
diagnosed_error!("unable to parse address: {}", addr)

// New way
Err(Report::new(EvmError::InvalidAddress))
    .attach_printable(format!("Invalid address: {}", addr))
    .with_location(file!(), line!(), column!())
    .with_documentation("Addresses must start with 0x and be 42 chars")
    .with_example("0x742d35Cc6634C0532925a3b844Bc9e7595f89590")
    .with_action_context("deploy_contract", "evm", construct_id)
```

### Propagating errors with additional context:

```rust
parse_address(input)
    .change_context(TxtxError::Execution)
    .attach_printable("Failed during contract deployment")
    .with_transaction_info(tx_details)?
```

The new system makes debugging significantly easier and provides users with actionable information to resolve issues quickly.