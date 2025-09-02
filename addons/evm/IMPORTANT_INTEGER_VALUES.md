# Important: Use Integers for Numeric Values in Runbooks

## Issue
When writing txtx runbooks, numeric values MUST be written as integers (without quotes), not as strings (with quotes).

## ❌ WRONG (causes panic)
```hcl
action "send_eth" "evm::send_eth" {
    recipient_address = "0x123..."
    amount = "1000000000000000000"  # STRING - will cause panic!
    gas_limit = "21000"              # STRING - will cause panic!
    confirmations = "1"              # STRING - will cause panic!
    signer = signer.alice
}
```

## ✅ CORRECT
```hcl
action "send_eth" "evm::send_eth" {
    recipient_address = "0x123..."    # Addresses are strings - OK
    amount = 1000000000000000000      # INTEGER - no quotes!
    gas_limit = 21000                 # INTEGER - no quotes!
    confirmations = 1                 # INTEGER - no quotes!
    signer = signer.alice
}
```

## Fields That Must Be Integers

### Common Fields
- `amount` - Wei amount for transactions
- `confirmations` - Number of block confirmations to wait
- `gas_limit` - Gas limit for transaction
- `gas_price` - Gas price in wei
- `max_fee_per_gas` - EIP-1559 max fee
- `max_priority_fee_per_gas` - EIP-1559 priority fee
- `nonce` - Transaction nonce
- `chain_id` - Chain ID number

### Examples

#### send_eth
```hcl
action "transfer" "evm::send_eth" {
    recipient_address = input.recipient  # String
    amount = 1000000000000000000         # Integer (1 ETH in wei)
    confirmations = 0                    # Integer
    signer = signer.alice
}
```

#### call_contract
```hcl
action "call" "evm::call_contract" {
    contract_address = "0x123..."        # String
    contract_abi = "[...]"                # String
    function_name = "transfer"           # String
    function_args = ["0x456...", 100]    # Mixed: string address, integer amount
    amount = 0                            # Integer (no ETH sent)
    gas_limit = 100000                   # Integer
    signer = signer.alice
}
```

## Why This Happens

The txtx parser treats quoted values as strings and unquoted numeric values as integers. The EVM addon expects integers for numeric fields and will panic with "internal error: entered unreachable code" when it receives a string instead.

## How to Debug

If you see this error:
```
thread 'main' panicked at crates/txtx-addon-kit/src/types/types.rs:349:18:
internal error: entered unreachable code
```

Check your runbook for quoted numeric values and remove the quotes!

## Wei Values Reference

Remember that Ethereum uses wei as the smallest unit:
- 1 wei = 1
- 1 gwei = 1000000000 (10^9)
- 1 ether = 1000000000000000000 (10^18)

Always use the full wei value as an integer:
```hcl
# Send 0.1 ETH
amount = 100000000000000000   # 0.1 * 10^18

# Send 1 ETH  
amount = 1000000000000000000   # 1 * 10^18

# Send 10 gwei (for gas price)
gas_price = 10000000000        # 10 * 10^9
```