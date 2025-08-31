# txtx EVM Addon

Comprehensive EVM (Ethereum Virtual Machine) support for txtx, enabling interaction with Ethereum and EVM-compatible blockchains.

## Features

### Core Functionality
- 🔄 **Transaction Management** - Send ETH, tokens, and interact with smart contracts
- 📦 **Contract Deployment** - Deploy contracts with constructor arguments
- 📞 **Contract Interactions** - Call functions, read state, handle events
- ✍️ **Multiple Signers** - Support for private keys, mnemonics, and hardware wallets
- 🔍 **View Functions** - Automatic detection and gas-free execution of read-only functions
- 🌍 **Unicode Support** - Full UTF-8 support for international applications

### Advanced Features
- **CREATE2 Deployments** - Deterministic contract addresses (see [CREATE2_DEPLOYMENT.md](./CREATE2_DEPLOYMENT.md))
- **Proxy Patterns** - Support for upgradeable contracts
- **Batch Operations** - Execute multiple transactions efficiently
- **Gas Optimization** - Smart gas estimation and management
- **Error Recovery** - Comprehensive error handling with actionable messages

## Installation

The EVM addon is included with txtx. No separate installation needed.

### Prerequisites
For testing and development:
```bash
# Install Foundry (includes Anvil for local testing)
curl -L https://foundry.paradigm.xyz | bash
foundryup
```

## Quick Start

### Basic ETH Transfer
```hcl
addon "evm" {
    chain_id = 1  # Ethereum mainnet
    rpc_api_url = "https://eth-mainnet.g.alchemy.com/v2/YOUR-API-KEY"
}

signer "alice" "evm::secret_key" {
    secret_key = env.PRIVATE_KEY
}

action "send" "evm::send_eth" {
    recipient_address = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb7"
    amount = 1000000000000000000  # 1 ETH in wei
    signer = signer.alice
}

output "tx_hash" {
    value = action.send.tx_hash
}
```

### Smart Contract Interaction
```hcl
action "call_contract" "evm::call_contract_function" {
    contract_address = "0x..."
    function_signature = "transfer(address,uint256)"
    function_args = ["0xrecipient...", 1000000]
    signer = signer.alice
}
```

## Documentation

### Architecture & Development
- [TEST_INFRASTRUCTURE.md](TEST_INFRASTRUCTURE.md) - Complete testing framework documentation
- [ERROR_STACK_SUMMARY.md](ERROR_STACK_SUMMARY.md) - Error handling implementation details
- [UNICODE_SUPPORT.md](UNICODE_SUPPORT.md) - International character support

### Test Organization
- [TEST_CREATION_GUIDE.md](TEST_CREATION_GUIDE.md) - How to write tests
- [TEST_QUICK_REFERENCE.md](TEST_QUICK_REFERENCE.md) - Common patterns and snippets
- [TEST_MIGRATION_TRACKER.md](TEST_MIGRATION_TRACKER.md) - Test migration progress
- [FIXTURE_CONSOLIDATION_PLAN.md](FIXTURE_CONSOLIDATION_PLAN.md) - Test fixture strategy

### Implementation Notes
- [ERROR_FIXTURES.md](ERROR_FIXTURES.md) - Error scenario test fixtures
- [EMOJI_CLEANUP.md](EMOJI_CLEANUP.md) - Unicode handling in tests
- [VIEW_FUNCTION_OPTIMIZATION.md](VIEW_FUNCTION_OPTIMIZATION.md) - Gas-free read optimization

## Project Structure

```
addons/evm/
├── src/
│   ├── lib.rs                 # Main addon entry point
│   ├── codec/                 # ABI encoding/decoding
│   │   ├── abi.rs            # ABI type system
│   │   └── encoder.rs        # Encoding implementation
│   ├── commands/              # CLI commands
│   ├── rpc.rs                # RPC client implementation
│   ├── signers/              # Transaction signing
│   ├── errors.rs             # Error types with error-stack
│   └── tests/                # Comprehensive test suite
├── fixtures/                  # Test fixtures
│   ├── integration/          # Integration test runbooks
│   └── parsing/              # Parser test runbooks
└── contracts/                # Test contracts
```

## Actions

### Transaction Actions
- `evm::send_eth` - Send ETH to an address
- `evm::send_erc20` - Transfer ERC20 tokens
- `evm::call_contract_function` - Call any contract function
- `evm::deploy_contract` - Deploy a new contract

### Query Actions
- `evm::get_balance` - Get ETH balance
- `evm::get_erc20_balance` - Get token balance
- `evm::get_transaction_receipt` - Fetch transaction details
- `evm::call_contract_read_function` - Read contract state

### Utility Actions
- `evm::encode_function_call` - Encode function calls
- `evm::decode_function_result` - Decode return values
- `evm::compute_contract_address` - Calculate deployment addresses

## Functions

### Encoding Functions
- `evm::encode_address` - Encode addresses
- `evm::encode_uint256` - Encode numbers
- `evm::encode_bytes` - Encode byte arrays
- `evm::encode_string` - Encode strings (with Unicode support)

### Utility Functions
- `evm::get_chain_id` - Get current chain ID
- `evm::wei_to_eth` - Convert wei to ETH
- `evm::eth_to_wei` - Convert ETH to wei
- `evm::keccak256` - Compute Keccak256 hash

## Testing

### Run All Tests
```bash
cargo test --package txtx-addon-network-evm
```

### Run Specific Test Categories
```bash
# Unit tests only
cargo test --package txtx-addon-network-evm --lib

# Integration tests (requires Anvil)
cargo test --package txtx-addon-network-evm integration

# Unicode support tests
cargo test --package txtx-addon-network-evm unicode_storage

# Error handling tests
cargo test --package txtx-addon-network-evm error_handling
```

### Test Coverage Areas
- ✅ Basic transactions and transfers
- ✅ Contract deployment and interaction
- ✅ ABI encoding/decoding
- ✅ Error scenarios and edge cases
- ✅ Unicode/international character support
- ✅ Gas estimation and optimization
- ✅ View function detection
- ✅ CREATE2 deployments

## Configuration

### Network Configuration
```hcl
addon "evm" {
    chain_id = 1              # Required: Network chain ID
    rpc_api_url = "..."       # Required: RPC endpoint URL
    
    # Optional configurations
    gas_price = 20000000000   # Gas price in wei
    gas_limit = 3000000       # Default gas limit
    confirmations = 1         # Block confirmations to wait
}
```

### Signer Types

#### Private Key
```hcl
signer "user" "evm::secret_key" {
    secret_key = "0x..."  # 64-character hex string
}
```

#### Mnemonic
```hcl
signer "user" "evm::mnemonic" {
    mnemonic = "word1 word2 ... word12"
    derivation_path = "m/44'/60'/0'/0/0"  # Optional
}
```

## Common Patterns

### Deploying and Verifying Contracts
```hcl
# Deploy contract
action "deploy" "evm::deploy_contract" {
    contract_name = "MyContract"
    artifact_source = "foundry"  # or "hardhat", "inline:0x..."
    constructor_args = [42, "Hello"]
    signer = signer.deployer
}

# Verify deployment
action "verify_code" "evm::get_code" {
    address = action.deploy.contract_address
}

output "deployed_address" {
    value = action.deploy.contract_address
}
```

### Working with Unicode Data
```hcl
# Store international text
action "set_message" "evm::call_contract_function" {
    contract_address = "0x..."
    function_signature = "setMessage(string)"
    function_args = ["Hello 世界 🌍"]  # Full Unicode support
    signer = signer.user
}
```

### Error Handling
```hcl
# Actions automatically handle common errors:
# - Insufficient funds
# - Invalid addresses
# - Failed transactions
# - Network issues
# - Contract reverts

# Errors provide actionable feedback:
# "Insufficient funds: need 1.5 ETH, have 0.5 ETH"
# "Contract reverted: ERC20: transfer amount exceeds balance"
```

## Troubleshooting

### Common Issues

1. **"Anvil not found"**
   - Install Foundry: `curl -L https://foundry.paradigm.xyz | bash`

2. **"Insufficient funds" errors**
   - Ensure account has enough ETH for gas
   - Check if amount + gas exceeds balance

3. **"Function not found" errors**
   - Verify function signature matches contract ABI
   - Check that contract is deployed at the address

4. **Unicode characters not displaying**
   - Ensure terminal supports UTF-8
   - Check that source files are UTF-8 encoded

## Contributing

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for general contribution guidelines.

### EVM-Specific Guidelines
1. Add tests for new features in `src/tests/`
2. Create fixtures for integration tests
3. Update error types in `errors.rs` using error-stack
4. Document new actions/functions
5. Ensure Unicode compatibility

## License

Same as txtx project - see [LICENSE](../../LICENSE)

## Support

For issues specific to the EVM addon:
- Open an issue with `[EVM]` prefix
- Include runbook examples
- Provide RPC endpoint (use public endpoints for reproduction)
- Include full error messages with stack traces