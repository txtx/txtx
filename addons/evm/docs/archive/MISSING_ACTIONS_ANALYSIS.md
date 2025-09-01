# Missing EVM Actions Analysis

## Currently Implemented Actions

Based on the codebase, these actions are currently implemented:
1. `evm::send_eth` - Send ETH from one address to another
2. `evm::check_confirmations` - Wait for transaction confirmations
3. `evm::sign_transaction` - Sign a transaction
4. `evm::eth_call` - Make a read-only call to a contract
5. `evm::deploy_contract` - Deploy a smart contract
6. `evm::call_contract` - Call a contract function (state-changing)

## Missing Actions That Tests Expect

### 1. `evm::decode_abi`
**Purpose**: Decode ABI-encoded data back into readable values
**Used in**: `abi_decode_test.tx`
**Expected inputs**:
- `data`: Hex-encoded ABI data to decode
- `types`: Array of Solidity types to decode as (e.g., ["address", "uint256"])
**Expected outputs**:
- Decoded values in their respective types

### 2. `evm::encode_abi`
**Purpose**: Encode values into ABI format for contract calls
**Used in**: `abi_encode_basic.tx`, `abi_encode_complex.tx`
**Expected inputs**:
- `types`: Array of Solidity types (e.g., ["address", "uint256", "bool"])
- `values`: Array of values to encode
**Expected outputs**:
- Hex-encoded ABI data

### 3. `evm::call_contract_function`
**Purpose**: Call a specific contract function by signature
**Used in**: `unicode_edge_cases.tx`, `unicode_storage.tx`
**Expected inputs**:
- `contract_address`: Address of the contract
- `function_signature`: Function signature like "transfer(address,uint256)"
- `function_args`: Array of arguments matching the signature
- `signer`: (optional) Signer for the transaction
**Expected outputs**:
- `tx_hash`: Transaction hash
- `result`: Return value from the function (if any)

**Note**: This appears to be similar to `evm::call_contract` but with a more user-friendly interface using function signatures instead of encoded data.

### 4. `evm::get_logs`
**Purpose**: Retrieve event logs from the blockchain
**Used in**: `event_logs.tx`
**Expected inputs**:
- `address`: Contract address to get logs from
- `from_block`: Starting block number or "latest"
- `to_block`: Ending block number or "latest"
- `topics`: (optional) Array of topic filters
**Expected outputs**:
- Array of log entries with decoded event data

### 5. `evm::simulate_transaction`
**Purpose**: Simulate a transaction without actually sending it (dry run)
**Used in**: `transaction_simulation.tx`
**Expected inputs**:
- `from`: Sender address
- `to`: Recipient address
- `value`: (optional) ETH amount to send
- `data`: (optional) Contract call data
- `gas`: (optional) Gas limit
**Expected outputs**:
- `success`: Whether the simulation succeeded
- `gas_used`: Estimated gas usage
- `return_data`: Any return data from the call
- `revert_reason`: (optional) Reason if the transaction would revert

## Implementation Status Analysis

### Why These Are Missing

Looking at the codebase structure, it appears that:

1. **ABI encoding/decoding** might be intended as **functions** rather than **actions**:
   - Functions are pure computations (no blockchain interaction)
   - Actions are for blockchain state changes
   - ABI encode/decode are pure data transformations

2. **`call_contract_function`** seems to be a higher-level wrapper around `call_contract`:
   - `call_contract` requires pre-encoded data
   - `call_contract_function` would handle the encoding internally

3. **`get_logs`** is a read operation that should probably exist
   - Essential for testing events
   - Common use case in smart contract testing

4. **`simulate_transaction`** is like `eth_call` but for transactions:
   - `eth_call` is for view functions
   - `simulate_transaction` would be for simulating state-changing transactions

## Recommendations

### Should Be Functions (not Actions):
- `encode_abi` - Pure data transformation
- `decode_abi` - Pure data transformation

### Should Be Actions:
- `get_logs` - Blockchain read operation
- `simulate_transaction` - Blockchain simulation

### Already Exists (Maybe):
- `call_contract_function` - Might be what `call_contract` does, just needs different interface

### Current Workarounds:
- For ABI encoding: Use the codec functions directly
- For function calls: Use `call_contract` with pre-encoded data
- For logs: Not currently possible without implementation
- For simulation: Use `eth_call` for view functions only