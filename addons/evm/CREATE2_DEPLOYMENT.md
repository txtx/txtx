# CREATE2 Deployment Support

The EVM addon provides full support for CREATE2 deterministic contract deployment.

## Usage

CREATE2 deployment is available through the `evm::deploy_contract` action:

```hcl
action "deploy" "evm::deploy_contract" {
    contract = variable.my_contract
    constructor_args = [42]
    create2 = {
        salt = "0x0000000000000000000000000000000000000000000000000000000000000042"
        # Optional: custom factory address (defaults to standard CREATE2 factory)
        # factory_address = "0x..."
    }
    signer = signer.deployer
    confirmations = 1
}
```

## Address Calculation

You can pre-calculate the deployment address using the `evm::create2` function:

```hcl
variable "expected_address" {
    value = evm::create2(variable.salt, variable.init_code)
}
```

Where `init_code` is the contract bytecode concatenated with constructor arguments:

```hcl
variable "init_code" {
    value = std::concat(
        variable.contract.bytecode,
        evm::encode_constructor_args(variable.contract.abi, [42])
    )
}
```

## Test Coverage

CREATE2 functionality is thoroughly tested:

- **Address Calculation**: `src/tests/integration/deployment_tests.rs::test_create2_address_calculation`
- **Full Deployment**: `src/tests/integration/foundry_deploy_tests.rs::test_deploy_with_create2_from_foundry`
- **Factory Support**: Custom CREATE2 factory addresses are supported via the `factory_address` field

## Implementation Details

- Default CREATE2 factory: `0x4e59b44847b379578588920cA78FbF26c0B4956C`
- Salt must be 32 bytes (64 hex characters)
- Deployment is idempotent - deploying to the same address twice will succeed if the contract already exists
- Proxied deployments also support CREATE2 for deterministic proxy addresses

## Example: Deterministic Multi-Chain Deployment

```hcl
# Deploy the same contract to the same address across multiple chains
variable "universal_salt" {
    value = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
}

action "deploy_mainnet" "evm::deploy_contract" {
    contract = variable.my_contract
    create2 = { salt = variable.universal_salt }
    signer = signer.mainnet_deployer
}

action "deploy_polygon" "evm::deploy_contract" {
    contract = variable.my_contract
    create2 = { salt = variable.universal_salt }
    signer = signer.polygon_deployer
}

# Both deployments will result in the same contract address
```