# EVM Addon: Error Handling Enhancement & Testing Infrastructure

## Summary

This PR introduces comprehensive improvements to the txtx EVM addon, focusing on two major areas:
1. **Error handling enhancement** using error-stack for rich context preservation
2. **Fixture-based testing system** for robust, isolated test environments

## Key Changes

### 🎯 Error Handling with error-stack

- Migrated from basic error types to error-stack with full context preservation
- Added context attachments (TransactionContext, RpcContext, ContractContext)
- Improved error messages with actionable information
- Maintained backward compatibility while enhancing debugging capabilities

### 🧪 Fixture-Based Testing System

- **FixtureBuilder**: Fluent API for creating isolated test environments
- **AnvilManager**: Singleton Anvil instance with snapshot/revert for test isolation
- **RunbookParser**: Leverages txtx-core's HCL parser for consistency
- **Named Accounts**: 26 deterministic test accounts (alice through zed)
- **Auto-generated Outputs**: Automatic test output blocks for all actions
- **Source-based Testing**: Always builds txtx from current source

### 📚 Documentation

- Comprehensive testing guide (`TESTING_GUIDE.md`)
- Fixture builder architecture documentation
- Implementation summary with migration guides
- Extensive inline documentation

### 🔧 Code Quality

- Fixed duplicate struct definitions
- Resolved import path issues
- Corrected Ethereum address case sensitivity (EIP-55)
- Fixed Anvil startup issues
- Improved binary discovery logic

## Testing

The PR includes:
- 15+ new fixture builder tests
- Integration tests for contract deployment
- Error handling showcase tests
- Performance benchmarks
- All existing tests continue to pass

### Running Tests

```bash
# Run all EVM tests
cargo test --package txtx-addon-network-evm

# Run fixture builder tests specifically
cargo test --package txtx-addon-network-evm fixture_builder

# Run with output
cargo test --package txtx-addon-network-evm -- --nocapture
```

## Breaking Changes

None. All changes are backward compatible.

## Migration Guide

For teams wanting to adopt the new testing infrastructure:

```rust
// Old approach
#[test]
fn test_transfer() {
    // Manual setup...
}

// New approach
#[tokio::test]
async fn test_transfer() {
    let fixture = FixtureBuilder::new("test")
        .build()
        .await
        .unwrap();
    
    fixture.execute_runbook("transfer").await.unwrap();
    assert_action_success(&fixture.get_outputs("transfer").unwrap(), "transfer");
}
```

## Performance Impact

- Test execution is faster due to Anvil singleton with snapshots
- No runtime performance impact on production code
- Improved error handling has minimal overhead

## Future Work

- [ ] Gas usage tracking and assertions
- [ ] Event log verification helpers
- [ ] Multi-chain testing support
- [ ] Automated test generation

## Checklist

- [x] Tests pass locally
- [x] Documentation updated
- [x] No breaking changes
- [x] Code follows project conventions
- [x] Comprehensive test coverage
- [x] Migration guide included

## Related Issues

Addresses the need for:
- Better error context in blockchain operations
- Isolated test environments for runbooks
- Consistent HCL parsing in tests
- Reliable test execution

## Screenshots/Examples

### Error Handling Example
```rust
transaction_builder()
    .change_context(EvmError::Transaction)
    .attach(TransactionContext {
        hash: "0x123...",
        from: "0xabc...",
        to: "0xdef...",
    })
    .attach_printable("Failed to build transaction")?;
```

### Fixture Builder Example
```rust
let fixture = FixtureBuilder::new("my_test")
    .with_environment("testing")
    .with_confirmations(1)
    .build()
    .await?;
```

## Review Notes

- The error-stack integration follows the established patterns from the error-stack documentation
- The fixture builder is designed to be extensible for future addon testing needs
- All tests are isolated using Anvil snapshots to prevent state pollution
- Documentation is comprehensive and includes examples for common use cases