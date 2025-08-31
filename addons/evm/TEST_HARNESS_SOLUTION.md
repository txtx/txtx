# Test Harness Solution: JSON Output Approach

## Summary
The test harness should use txtx's JSON output capability to verify runbook execution results, rather than trying to intercept internal execution state. This approach keeps the temp filesystem around long enough to run txtx CLI and parse its JSON output.

## Implementation Status

### ✅ Completed
1. Modified test harness to keep temp directory alive during test execution
2. Added ability to execute txtx CLI binary directly with `--output-json` flag
3. Created basic test structure that sets up projects correctly
4. Identified that `--unsupervised` mode is needed for non-interactive execution

### 🚧 In Progress
1. JSON output parsing - The structure needs to execute txtx and parse the resulting JSON
2. Output verification - Tests need to check the JSON outputs match expectations

### ❌ Issues Found
1. **txtx execution hangs** - Even with `--unsupervised` and `confirmations=0`, txtx may hang waiting for something
2. **Many old processes** - Found many stuck txtx processes from previous test runs
3. **Output evaluation** - Outputs that reference action results (`action.send_eth.tx_hash`) may not be evaluated in unsupervised mode

## Recommended Approach

### 1. Use Simple Output-Only Runbooks for Basic Tests
```hcl
# Test runbook that doesn't require action execution
output "test_passed" {
    value = true
}

output "computed_value" {
    value = input.some_value * 2
}
```

### 2. Add Test-Specific Outputs
For action tests, add outputs that can be verified without complex expression evaluation:
```hcl
action "send_eth" "evm::send_eth" {
    # ... action config ...
}

# Add simple test outputs
output "action_executed" {
    value = true  # Simple flag that action was reached
}
```

### 3. Use Supervisor Mode for Complex Tests
For tests that need full action execution and output evaluation, consider:
- Using supervisor mode with automated responses
- Creating a test-specific supervisor that auto-approves transactions
- Running with a mock blockchain that doesn't require real transactions

## Alternative Solutions

### Option 1: Direct State File Access
After txtx execution, read the state file directly:
```rust
let state_file = project_path.join(".txtx/state.json");
let state: serde_json::Value = serde_json::from_reader(File::open(state_file)?)?;
// Extract outputs from state
```

### Option 2: Log Parsing
Parse txtx logs for execution results:
```rust
cmd.arg("--log-level").arg("debug");
// Parse stdout/stderr for action results
```

### Option 3: Test-Specific Execution Mode
Add a `--test` flag to txtx that:
- Auto-approves all transactions
- Evaluates all outputs regardless of action success
- Returns structured JSON with all intermediate values

## Next Steps

1. **Fix the hanging issue** - Investigate why txtx hangs even in unsupervised mode
2. **Create working examples** - Build tests that successfully execute and verify outputs
3. **Document patterns** - Create templates for common test scenarios
4. **Clean up processes** - Add automatic cleanup of stuck txtx processes

## Test Categories

### Level 1: Structure Tests (✅ Working)
- Verify runbook parsing
- Check project setup
- Validate configuration

### Level 2: Evaluation Tests (🚧 Partial)
- Test input evaluation
- Verify output expressions
- Check variable substitution

### Level 3: Action Tests (❌ Blocked)
- Execute blockchain actions
- Verify transaction results
- Check state changes

### Level 4: Integration Tests (❌ Blocked)
- Multi-action workflows
- Cross-addon interactions
- Complex state management