# EVM Addon Test Infrastructure

## Overview

The EVM addon test suite uses a sophisticated infrastructure for testing against a real Ethereum node (Anvil). This document explains the architecture and requirements for writing and running tests.

## Sequential Execution Requirement

**IMPORTANT**: All integration tests that use Anvil MUST run sequentially, not in parallel.

### Why Sequential Execution?

1. **Singleton Anvil Instance**: We use a single Anvil instance across all tests for performance
2. **Snapshot/Revert Mechanism**: Tests use snapshots to isolate state, which would conflict if run in parallel
3. **Port Management**: Anvil runs on specific ports that can't be shared
4. **Resource Efficiency**: Running multiple Anvil instances is resource-intensive

### How to Ensure Sequential Execution

All tests using Anvil must be marked with the `#[serial(anvil)]` attribute:

```rust
use serial_test::serial;

#[tokio::test]
#[serial(anvil)]  // This ensures sequential execution
async fn my_test() {
    // Test implementation
}
```

## Test Infrastructure Components

### 1. Anvil Singleton (`anvil_singleton.rs`)

- Manages a single Anvil process across all tests
- Uses `OnceLock` for true singleton pattern
- Tracks PID in `/tmp/txtx_test_anvil.pid` for cleanup
- Automatically cleans up on exit

### 2. Anvil Manager (`anvil_manager.rs`)

- Wraps the singleton with snapshot/revert functionality
- Manages test isolation through Anvil snapshots
- Provides RPC client for blockchain operations

### 3. Fixture Builder (`fixture_builder/mod.rs`)

- Creates test project structures
- Generates txtx.yml configuration
- Manages runbook execution
- Provides comprehensive instrumentation for debugging

### 4. Test Accounts

The test suite uses 26 deterministic accounts (alice through zed) with:
- Known addresses and private keys
- 10,000 ETH balance each
- Derived from test mnemonic: "test test test test test test test test test test test junk"

## Writing Tests

### Basic Test Structure

```rust
#[cfg(test)]
mod my_tests {
    use crate::tests::integration::anvil_harness::AnvilInstance;
    use crate::tests::fixture_builder::{FixtureBuilder, get_anvil_manager};
    use serial_test::serial;
    use tokio;
    
    #[tokio::test]
    #[serial(anvil)]  // REQUIRED for Anvil tests
    async fn test_example() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test - Anvil not installed");
            return;
        }
        
        // Build test fixture
        let mut fixture = FixtureBuilder::new("test_name")
            .with_anvil_manager(get_anvil_manager().await.unwrap())
            .with_runbook("my_runbook", &runbook_content)
            .build()
            .await
            .expect("Failed to build fixture");
        
        // Add parameters
        fixture.config.parameters.insert("key".to_string(), "value".to_string());
        
        // Execute runbook
        fixture.execute_runbook("my_runbook").await
            .expect("Failed to execute runbook");
        
        // Check outputs
        let outputs = fixture.get_outputs("my_runbook")
            .expect("Should have outputs");
        
        // Assertions...
    }
}
```

### Running Tests

```bash
# Run all tests (sequentially)
cargo test --package txtx-addon-network-evm

# Run specific test
cargo test --package txtx-addon-network-evm test_name

# Run with output
cargo test --package txtx-addon-network-evm -- --nocapture

# Run tests in a specific file
cargo test --package txtx-addon-network-evm --lib tests::integration::module_name
```

## Common Issues and Solutions

### Issue: Tests Fail with Port Conflicts
**Solution**: Ensure all tests use `#[serial(anvil)]` attribute

### Issue: Snapshot/Revert Errors
**Solution**: Tests must run sequentially - check for missing `#[serial(anvil)]` attributes

### Issue: Multiple Anvil Instances
**Solution**: Kill stray instances with `pkill -f "anvil.*test test test"`

### Issue: Tests Timeout
**Solution**: 
1. Check if Anvil is installed (`anvil --version`)
2. Ensure no other process is using the test ports
3. Check system resources

## Debugging Tests

The test infrastructure provides comprehensive instrumentation:

```
🔧 Creating Anvil manager (singleton-backed)...
📸 Taking snapshot: test_name
📁 Creating project structure in: /tmp/.tmpXXXXXX
📝 Registering 1 runbook(s) in txtx.yml
🚀 Executing runbook: my_runbook
✅ Execution successful, 2 outputs captured
```

Enable verbose output with `-- --nocapture` flag when running tests.

### Preserving Test Directories

Test directories are automatically preserved when:
1. A test panics or fails
2. The `PRESERVE_TEST_DIRS` environment variable is set
3. You explicitly call `fixture.preserve_directory()` in your test

To always preserve test directories for inspection:

```bash
# Preserve all test directories
PRESERVE_TEST_DIRS=1 cargo test --package txtx-addon-network-evm

# Preserve and show output
PRESERVE_TEST_DIRS=1 cargo test --package txtx-addon-network-evm -- --nocapture
```

When preserved, you'll see:
```
📁 Preserving test directory: /tmp/.tmpXXXXXX
   ⚠️  Test panicked - directory preserved for debugging
```

You can then inspect the directory contents:
```bash
ls -la /tmp/.tmpXXXXXX/
cat /tmp/.tmpXXXXXX/txtx.yml
cat /tmp/.tmpXXXXXX/runbooks/*/main.tx
```

## Best Practices

1. **Always use `#[serial(anvil)]`** for tests using Anvil
2. **Check Anvil availability** before running tests
3. **Use descriptive test names** for better debugging
4. **Clean up resources** - the framework handles this automatically
5. **Use snapshots** for test isolation rather than deploying new contracts
6. **Keep fixtures simple** - complex fixtures are harder to debug
7. **Document expected outputs** in test assertions

## Architecture Decisions

### Why Singleton Anvil?
- **Performance**: Starting Anvil is slow (~100-500ms per instance)
- **Resource Usage**: Each Anvil uses significant memory
- **Consistency**: All tests use the same blockchain state baseline

### Why Snapshot/Revert?
- **Isolation**: Each test gets a clean state
- **Speed**: Reverting is much faster than redeploying
- **Predictability**: Tests always start from known state

### Why Sequential Execution?
- **Simplicity**: No complex locking or state management
- **Reliability**: Eliminates race conditions
- **Debugging**: Easier to troubleshoot failures

## Future Improvements

- [ ] Parallel test execution with multiple Anvil instances
- [ ] Better error reporting with error-stack integration
- [ ] Automatic retry for flaky tests
- [ ] Performance profiling for slow tests
- [ ] Integration with CI/CD pipelines