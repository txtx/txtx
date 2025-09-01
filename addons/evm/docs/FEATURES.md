# EVM Addon Features

## Core Features

### Transaction Management

#### Send ETH
Transfer native ETH between addresses:

```hcl
action "transfer" "evm::send_eth" {
    from = "0x..."
    to = "0x..."
    amount = "1000000000000000000"  # 1 ETH in wei
    signer = signer.alice
}
```

#### Send Transaction
Send raw transactions with full control:

```hcl
action "tx" "evm::send_transaction" {
    to = "0x..."
    data = "0x..."
    value = "0"
    gas_limit = 100000
    signer = signer.alice
}
```

### Smart Contract Support

#### Deploy Contracts
Deploy contracts with constructor arguments:

```hcl
action "deploy" "evm::deploy_contract" {
    contract = "Token"
    constructor_args = ["MyToken", "MTK", 1000000]
    signer = signer.deployer
}
```

#### Call Contract Functions
Interact with deployed contracts:

```hcl
action "call" "evm::call_contract" {
    contract_address = action.deploy.contract_address
    function = "transfer(address,uint256)"
    args = ["0x...", 100]
    signer = signer.alice
}
```

#### View Functions
Read contract state without gas:

```hcl
action "read" "evm::eth_call" {
    contract_address = "0x..."
    function = "balanceOf(address)"
    args = ["0x..."]
}
```

### ABI Encoding/Decoding

#### Encode ABI
Encode function calls and parameters:

```hcl
action "encode" "evm::encode_abi" {
    types = ["address", "uint256", "bool"]
    values = ["0x...", 123, true]
}
```

#### Decode ABI
Decode transaction data and logs:

```hcl
action "decode" "evm::decode_abi" {
    types = ["address", "uint256"]
    data = "0x..."
}
```

## Advanced Features

### CREATE2 Deployment

Deploy contracts to deterministic addresses:

```hcl
action "deploy_create2" "evm::deploy_contract_create2" {
    contract = "Token"
    salt = "0x1234..."
    constructor_args = [...]
    signer = signer.deployer
}

output "predicted_address" {
    value = action.deploy_create2.predicted_address
}
```

Benefits:
- Predictable contract addresses
- Cross-chain same addresses
- Counterfactual deployments

### Unicode Support

Full UTF-8 support for international applications:

```hcl
action "store_unicode" "evm::call_contract" {
    function = "setMessage(string)"
    args = ["Hello 世界 🌍"]
}
```

Supports:
- Emoji: 🚀 💰 ⚡
- Chinese: 你好世界
- Japanese: こんにちは
- Korean: 안녕하세요
- Arabic: مرحبا بالعالم
- All UTF-8 characters

### View Function Detection

Automatic detection of read-only functions:

```hcl
# Automatically uses eth_call (no gas) for view/pure functions
action "get_balance" "evm::call_contract" {
    contract_address = "0x..."
    function = "balanceOf(address)"  # Detected as view function
    args = [input.user_address]
}
```

### Gas Optimization

#### Smart Gas Estimation
Automatic gas estimation with safety margins:

```hcl
action "transfer" "evm::send_eth" {
    # Gas automatically estimated
    # 20% safety margin added
    # Capped at reasonable limits
}
```

#### Custom Gas Settings
Override with specific values:

```hcl
action "complex_call" "evm::call_contract" {
    gas_limit = 500000
    gas_price = "20000000000"  # 20 gwei
    # Or use EIP-1559:
    max_fee_per_gas = "100000000000"
    max_priority_fee_per_gas = "2000000000"
}
```

### Transaction Confirmation

Wait for confirmations:

```hcl
action "send" "evm::send_eth" {
    confirmations = 6  # Wait for 6 blocks
    # ...
}

action "check" "evm::check_confirmations" {
    tx_hash = action.send.tx_hash
    confirmations = 12  # Wait for more confirmations
}
```

### Event Log Handling

Extract and decode contract events:

```hcl
action "get_receipt" "evm::check_confirmations" {
    tx_hash = "0x..."
}

output "events" {
    value = action.get_receipt.logs
}
```

## Signer Types

### Secret Key
Direct private key:

```hcl
signer "alice" "evm::secret_key" {
    secret_key = env.PRIVATE_KEY
}
```

### Mnemonic
HD wallet from seed phrase:

```hcl
signer "wallet" "evm::mnemonic" {
    mnemonic = env.SEED_PHRASE
    derivation_path = "m/44'/60'/0'/0/0"
}
```

### Hardware Wallet
(Planned) Ledger/Trezor support:

```hcl
signer "ledger" "evm::hardware_wallet" {
    type = "ledger"
    derivation_path = "m/44'/60'/0'/0/0"
}
```

## Chain Support

### Mainnet Chains
- Ethereum
- Polygon
- Binance Smart Chain
- Avalanche
- Arbitrum
- Optimism

### Testnets
- Sepolia
- Goerli
- Mumbai
- BSC Testnet

### Local Development
- Anvil (Foundry)
- Hardhat Network
- Ganache

Configuration example:

```hcl
addon "evm" {
    # Ethereum Mainnet
    chain_id = 1
    rpc_api_url = "https://eth-mainnet.g.alchemy.com/v2/KEY"
    
    # Polygon
    # chain_id = 137
    # rpc_api_url = "https://polygon-rpc.com"
    
    # Local Anvil
    # chain_id = 31337
    # rpc_api_url = "http://localhost:8545"
}
```

## Error Handling

### Rich Error Context

Errors include detailed context:

```
Error: Transaction failed

Caused by:
  0: RPC error
  1: Insufficient funds for gas * price + value

Context:
  Transaction:
    From: 0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266
    To: 0x70997970c51812dc3a010c7d01b50e0d17dc79c8
    Value: 1000000000000000000 wei (1 ETH)
    Gas: 21000
    Gas Price: 20 gwei
    
  Required: 1.00042 ETH
  Available: 0.5 ETH
  
Suggestion: Ensure account has sufficient balance for transaction + gas
```

### Recovery Suggestions

Errors include actionable suggestions:

- **Insufficient funds**: Check balance, reduce amount
- **Nonce mismatch**: Wait for pending transactions
- **Gas too low**: Increase gas limit or gas price
- **Contract error**: Check function signature and arguments

## Performance Features

### Connection Pooling
Reuse RPC connections across actions

### Compilation Caching
Contracts compiled once per session

### Parallel Execution
Multiple independent actions run concurrently

### Batch Operations
Group multiple calls for efficiency:

```hcl
# Coming soon: Batch multiple calls
action "batch" "evm::batch_call" {
    calls = [
        { target = "0x...", data = "0x..." },
        { target = "0x...", data = "0x..." },
    ]
}
```

## Security Features

### Input Validation
- Address checksum verification
- Amount overflow protection
- Gas limit boundaries
- ABI type checking

### Secret Protection
- No secrets in error messages
- Secure memory handling
- Environment variable support

### Transaction Safety
- Nonce management
- Gas price protection
- Reentrancy awareness

## Debugging Features

### Transaction Simulation
Preview transactions before sending:

```hcl
action "simulate" "evm::simulate_transaction" {
    to = "0x..."
    data = "0x..."
    value = "1000000000000000000"
}

output "would_succeed" {
    value = action.simulate.success
}
```

### State Overrides
Test with modified state:

```hcl
# Coming soon: State overrides for testing
action "test_call" "evm::eth_call" {
    state_overrides = {
        "0x...": {
            balance = "1000000000000000000"
        }
    }
}
```