# Error Handling Test Fixtures

## Overview
Created specialized fixtures for testing various error scenarios in the EVM addon.

## New Error Fixtures

### 1. insufficient_funds_transfer.tx
**Location**: `fixtures/integration/errors/insufficient_funds_transfer.tx`
**Purpose**: Tests transaction failures due to insufficient ETH balance
**Scenario**: Attempts to send 1 ETH from an account with no funds

### 2. insufficient_gas.tx  
**Location**: `fixtures/integration/errors/insufficient_gas.tx`
**Purpose**: Tests failures due to insufficient gas funds
**Scenario**: Deploys a contract from an account without enough ETH to pay for gas

### 3. invalid_hex_address.tx
**Location**: `fixtures/integration/errors/invalid_hex_address.tx`
**Purpose**: Tests invalid hex encoding in addresses
**Scenario**: Attempts to get balance of malformed address "0xINVALIDHEXADDRESS"

### 4. missing_signer.tx
**Location**: `fixtures/integration/errors/missing_signer.tx`
**Purpose**: Tests references to non-existent signers
**Scenario**: References `signer.nonexistent_signer` which is not defined

### 5. invalid_function_call.tx
**Location**: `fixtures/integration/errors/invalid_function_call.tx`
**Purpose**: Tests calling non-existent contract functions
**Scenario**: Deploys contract then calls `nonExistentFunction()` which doesn't exist

## Usage Pattern
These fixtures are designed to be used with the ProjectTestHarness:

```rust
let fixture = PathBuf::from("fixtures/integration/errors/insufficient_funds_transfer.tx");
let mut harness = ProjectTestHarness::from_fixture(&fixture);
harness
    .with_input("chain_id", Value::integer(chain_id))
    .with_input("rpc_url", Value::string(rpc_url));

let result = harness.run(vec![], vec![]);
assert!(result.is_err());
```

## Benefits
1. **Reusable**: Each fixture can be used by multiple tests
2. **Maintainable**: Error scenarios are defined in `.tx` files, not hardcoded in tests
3. **Realistic**: Tests actual txtx runbook execution, not mocked errors
4. **Comprehensive**: Covers major error categories (funds, encoding, references, functions)