# Test Writing Guidelines for EVM Addon

## Core Principle: ACT Pattern (Arrange-Act-Assert)

Every test MUST follow the ACT pattern:
1. **Arrange** - Set up test fixtures and inputs
2. **Act** - Execute the code under test
3. **Assert** - Verify the expected behavior with explicit assertions

## Test Structure Requirements

### 1. Test Documentation
Every test MUST have:
```rust
/// Test: [Brief description of what is being tested]
/// 
/// Expected Behavior:
/// - [Specific expected outcome 1]
/// - [Specific expected outcome 2]
/// 
/// Validates:
/// - [Business rule or requirement being validated]
#[test]
fn test_specific_behavior() {
    // Test implementation
}
```

### 2. Mandatory Assertions
Every test MUST contain at least one `assert!` statement. Tests that print success without assertions are **invalid**.

#### ❌ BAD - No Assertions
```rust
#[test]
fn test_something() {
    let result = execute_action();
    
    if result.is_ok() {
        println!("✅ Test passed");
    } else {
        println!("✅ Error handled");
    }
    // FAIL: This test always passes!
}
```

#### ✅ GOOD - Explicit Assertions
```rust
#[test]
fn test_something() {
    let result = execute_action();
    
    // Assert the expected outcome
    assert!(result.is_ok(), "Action should succeed, but failed with: {:?}", result);
    
    // Assert specific values
    let value = result.unwrap().outputs.get("key")
        .expect("Should have output 'key'");
    assert_eq!(value, "expected_value", "Output should match expected value");
}
```

### 3. Error Testing Pattern
When testing error conditions, be specific about the expected error:

#### ❌ BAD - Vague Error Check
```rust
#[test]
fn test_error_condition() {
    let result = execute_invalid_action();
    
    if result.is_err() {
        println!("✅ Error caught");
    }
    // FAIL: What error? Why should it fail?
}
```

#### ✅ GOOD - Specific Error Validation
```rust
/// Test: Invalid nonce causes transaction rejection
/// 
/// Expected Behavior:
/// - Transaction with nonce gap should be rejected
/// - Error message should mention "nonce too high" or "gap"
#[test]
fn test_nonce_gap_rejection() {
    // Arrange
    let current_nonce = 5;
    let gap_nonce = 100; // Large gap
    
    // Act
    let result = send_transaction_with_nonce(gap_nonce);
    
    // Assert - Specific error expected
    assert!(result.is_err(), "Transaction with nonce gap should fail");
    
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("nonce too high") || error_msg.contains("gap"),
        "Error should mention nonce issue, got: {}", 
        error_msg
    );
}
```

### 4. Multiple Scenario Testing
When a behavior could have multiple valid outcomes, test each explicitly:

#### ❌ BAD - Accepting Any Outcome
```rust
#[test]
fn test_deployment() {
    let result = deploy_large_contract();
    
    if result.is_ok() {
        println!("✅ Deployment succeeded");
    } else {
        println!("✅ Size limit enforced");
    }
    // FAIL: Which behavior is correct?
}
```

#### ✅ GOOD - Test Each Scenario Separately
```rust
/// Test: Large contract deployment succeeds with sufficient gas
/// 
/// Expected Behavior:
/// - Contract within size limit deploys successfully
/// - Returns valid contract address
#[test]
fn test_large_contract_deployment_success() {
    // Arrange - Contract just under 24KB limit
    let contract_bytecode = generate_contract_bytecode(24_000);
    
    // Act
    let result = deploy_contract(contract_bytecode);
    
    // Assert
    assert!(result.is_ok(), "Contract under size limit should deploy");
    
    let address = result.unwrap().contract_address;
    assert!(address.starts_with("0x"), "Should return valid address");
    assert_eq!(address.len(), 42, "Address should be 42 characters");
}

/// Test: Oversized contract deployment fails
/// 
/// Expected Behavior:
/// - Contract over 24KB limit should be rejected
/// - Error should mention size limit
#[test]
fn test_oversized_contract_rejection() {
    // Arrange - Contract over 24KB limit
    let contract_bytecode = generate_contract_bytecode(25_000);
    
    // Act
    let result = deploy_contract(contract_bytecode);
    
    // Assert
    assert!(result.is_err(), "Oversized contract should fail");
    
    let error = result.unwrap_err().to_string();
    assert!(
        error.contains("size") || error.contains("too large"),
        "Error should mention size issue: {}",
        error
    );
}
```

### 5. Output Validation Pattern
Always validate the structure and content of outputs:

```rust
/// Test: Transaction receipt contains required fields
/// 
/// Expected Behavior:
/// - Receipt includes transaction hash
/// - Receipt includes gas used (greater than 0)
/// - Receipt includes status (success = 1)
#[test]
fn test_transaction_receipt_fields() {
    // Arrange
    let tx_hash = send_transaction();
    
    // Act
    let receipt = get_transaction_receipt(tx_hash);
    
    // Assert structure
    assert!(receipt.is_ok(), "Should get receipt for mined transaction");
    
    let receipt = receipt.unwrap();
    
    // Assert required fields
    assert_eq!(receipt.transaction_hash, tx_hash, "Hash should match");
    assert!(receipt.gas_used > 0, "Should have used some gas");
    assert_eq!(receipt.status, 1, "Transaction should succeed");
    assert!(receipt.block_number > 0, "Should be in a block");
}
```

## Common Anti-Patterns to Avoid

### 1. ❌ Tests Without Purpose
Every test must validate a specific requirement or behavior.

### 2. ❌ Conditional Success
Tests should not have multiple "success" paths. Each test should verify ONE specific behavior.

### 3. ❌ Silent Failures
Tests must explicitly fail when expectations aren't met, not silently pass.

### 4. ❌ Overly Broad Tests
Break down complex scenarios into multiple focused tests.

### 5. ❌ Missing Edge Cases
Test boundary conditions, not just happy paths.

## Test Naming Convention

Test names should clearly indicate:
1. What is being tested
2. The condition or scenario
3. The expected outcome

```rust
// ✅ GOOD
fn test_nonce_gap_causes_rejection()
fn test_insufficient_funds_transaction_fails()
fn test_create2_address_matches_prediction()

// ❌ BAD
fn test_transaction()
fn test_error()
fn test_deployment()
```

## Assertion Messages

Always include descriptive messages in assertions:

```rust
// ✅ GOOD
assert!(result.is_ok(), "Expected successful deployment, but got: {:?}", result);
assert_eq!(actual, expected, "Contract address should match prediction. Expected: {}, Got: {}", expected, actual);

// ❌ BAD
assert!(result.is_ok());
assert_eq!(actual, expected);
```

## Summary Checklist

Before committing a test, verify:
- [ ] Has clear documentation of what it tests
- [ ] Documents expected behavior
- [ ] Contains at least one `assert!` statement
- [ ] Follows ACT pattern (Arrange-Act-Assert)
- [ ] Has descriptive test name
- [ ] Includes assertion messages
- [ ] Tests ONE specific behavior
- [ ] Cannot pass when behavior is wrong