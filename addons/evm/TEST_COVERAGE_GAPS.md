# EVM Test Coverage Gaps

## Overview
This document tracks test scenarios mentioned in `runbook_execution_tests.rs` and other places that are not yet implemented with actual integration tests.

## Coverage Status

### ✅ Already Tested

1. **send_eth / ETH transfers**
   - ✅ Basic transfer: `simple_eth_transfer.tx`
   - ✅ Custom gas: `custom_gas_transfer.tx`
   - ✅ Legacy transaction: `legacy_transaction.tx`
   - ✅ Batch transfers: `batch_transactions.tx`
   - ✅ Insufficient funds: `insufficient_funds_transfer.tx`

2. **deploy_contract**
   - ✅ Simple deployment: `minimal_contract.tx`
   - ✅ Constructor args: `constructor_args.tx`, `complex_constructor.tx`
   - ✅ Deploy and interact: `deploy_and_interact.tx`
   - ✅ Factory pattern: `factory_pattern.tx`
   - ✅ Proxy pattern: `upgradeable_proxy.tx`
   - ✅ CREATE2: `create2/` directory has fixtures

3. **call_contract**
   - ✅ Basic calls: Multiple fixtures use this
   - ✅ Complex ABI types: `complex_types.tx`
   - ✅ View functions: `view_functions/` directory
   - ✅ Function not found: `invalid_function_call.tx`

4. **Error Scenarios**
   - ✅ Insufficient funds: `insufficient_funds_transfer.tx`
   - ✅ Insufficient gas: `insufficient_gas.tx`
   - ✅ Invalid hex address: `invalid_hex_address.tx`
   - ✅ Function not found: `invalid_function_call.tx`
   - ✅ Missing signer: `missing_signer.tx`

### ❌ Missing Test Coverage

1. **Transaction Management**
   - ❌ Wrong nonce scenario
   - ❌ Nonce too high/too low errors
   - ❌ Transaction replacement by fee
   - ❌ Transaction cancellation
   - ❌ Pending transaction status

2. **Signing Operations**
   - ❌ `sign_transaction` action tests
   - ❌ Different signer types (mnemonic vs private key)
   - ❌ Hardware wallet simulation

3. **Confirmation Tracking**
   - ⚠️ `check_confirmations` has parsing fixture but needs integration test
   - ❌ Timeout waiting for confirmations
   - ❌ Reorg handling

4. **Advanced Error Scenarios**
   - ❌ Chain ID mismatch (partially tested)
   - ❌ RPC timeout/connection errors
   - ❌ Transaction underpriced
   - ❌ Contract size limit exceeded
   - ❌ Stack too deep error
   - ❌ Invalid opcode

5. **Gas Management**
   - ❌ EIP-1559 transaction tests
   - ❌ Access list transactions (EIP-2930)
   - ❌ Gas estimation failures
   - ❌ Max fee per gas scenarios

6. **Event Handling**
   - ❌ Event filtering tests
   - ❌ Event decoding tests
   - ❌ Log parsing tests

## Priority Implementation Plan

### High Priority (Core Functionality)
1. **Wrong nonce handling** - Critical for transaction management
2. **sign_transaction action** - Core signing functionality
3. **check_confirmations integration** - Transaction verification

### Medium Priority (Error Handling)
1. **RPC errors** - Connection, timeout, invalid responses
2. **Gas-related errors** - Estimation, limits, pricing
3. **Chain ID mismatch** - Network safety

### Low Priority (Advanced Features)
1. **Transaction replacement/cancellation**
2. **Event filtering and decoding**
3. **Hardware wallet support**

## Test Implementation Guidelines

### Creating New Test Fixtures

For each missing scenario, create a fixture in the appropriate directory:

```
fixtures/integration/
├── transactions/     # For nonce, replacement, cancellation tests
├── errors/          # For new error scenarios
├── gas/            # For gas management tests (new directory)
├── events/         # For event handling tests (new directory)
└── signing/        # For signing operation tests (new directory)
```

### Fixture Template

```hcl
# fixtures/integration/transactions/wrong_nonce.tx
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

signer "sender" "evm::secret_key" {
    secret_key = input.sender_private_key
}

# Deliberately use wrong nonce
action "wrong_nonce_tx" "evm::send_eth" {
    recipient_address = "0x70997970C51812dc3A010C7d01b50e0d17dc79C8"
    amount = 100
    signer = signer.sender
    nonce = 999  # This will be wrong
}

output "should_fail" {
    value = action.wrong_nonce_tx.tx_hash
}
```

### Test Implementation Template

```rust
#[test]
fn test_wrong_nonce_error() {
    let fixture = PathBuf::from("fixtures/integration/transactions/wrong_nonce.tx");
    let mut harness = ProjectTestHarness::from_fixture(&fixture)
        .with_anvil();
    
    harness.setup().expect("Failed to setup");
    
    let result = harness.execute_runbook();
    assert!(result.is_err());
    
    let error = result.unwrap_err();
    assert!(error.contains("nonce") || error.contains("Nonce"));
    
    harness.cleanup();
}
```

## Notes from runbook_execution_tests.rs

The file `runbook_execution_tests.rs` serves more as documentation/outline than actual tests. It contains:

1. **Example runbook formats** - These are documentation, not tests
2. **Error scenarios** - Most are now covered by fixtures
3. **Action list** - Verification that addon provides all actions

### Actions to Verify

The test mentions these actions should be provided:
- ✅ `send_eth` - Implemented and tested
- ✅ `deploy_contract` - Implemented and tested
- ✅ `deploy_contract_create2` - Has fixtures in create2/
- ✅ `call_contract` - Implemented and tested
- ⚠️ `eth_call` - Might be redundant with view functions
- ❌ `sign_transaction` - Not tested
- ⚠️ `check_confirmations` - Has parsing fixture, needs integration

## Recommendation

1. **Delete runbook_execution_tests.rs** - It's an outline, not real tests
2. **Implement high-priority missing tests** - Focus on nonce, signing, confirmations
3. **Create new fixture directories** - Organize by feature area
4. **Update TEST_MIGRATION_TRACKER.md** - Track new test additions

## Tracking

When implementing new tests:
1. Move item from ❌ to ✅ in this document
2. Update TEST_MIGRATION_TRACKER.md
3. Add fixture path and test function name
4. Run `check_migration_status.sh` to verify progress