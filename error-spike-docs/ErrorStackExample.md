# Error-Stack Example: EVM Contract Deployment

This example demonstrates how error-stack would improve error reporting for a typical EVM contract deployment flow in txtx.

## Current Error Flow

Let's trace an error through the current system when deploying an EVM contract fails:

```rust
// In evm/src/commands/actions/deploy_contract.rs
let result = provider.send_transaction(tx_request).await.map_err(|e| {
    diagnosed_error!("unable to send deploy_contract transaction request: {e}")
})?;

// Propagates to txtx-core/src/eval/mod.rs
match result {
    Err(diag) => {
        evaluations.push_diagnostic(
            diag,
            &construct.construct_did(),
            construct_type,
            &construct.name(),
            &namespace,
        );
    }
}

// Finally displayed in CLI
println!("{} {}", red!("x"), diag);
// Output: x unable to send deploy_contract transaction request: insufficient funds
```

### Problems with Current Approach:
- Lost context about which contract deployment failed
- No information about the account that lacks funds
- No guidance on how to fix the issue
- No stack trace to understand the call chain

## Error-Stack Improved Flow

### 1. Define Structured Error Types

```rust
use error_stack::{Report, ResultExt, Context};

#[derive(Debug)]
pub enum EvmDeployError {
    InvalidBytecode,
    InsufficientFunds,
    NetworkError,
    GasEstimationFailed,
}

impl fmt::Display for EvmDeployError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBytecode => write!(f, "Contract bytecode is invalid"),
            Self::InsufficientFunds => write!(f, "Insufficient funds for deployment"),
            Self::NetworkError => write!(f, "Network communication failed"),
            Self::GasEstimationFailed => write!(f, "Failed to estimate gas"),
        }
    }
}

impl Context for EvmDeployError {}
```

### 2. Create Rich Attachments

```rust
#[derive(Debug)]
pub struct DeploymentContext {
    pub contract_name: String,
    pub deployer_address: String,
    pub network: String,
    pub estimated_cost: Option<String>,
}

#[derive(Debug)]
pub struct AccountBalance {
    pub address: String,
    pub balance: String,
    pub required: String,
}

#[derive(Debug)]
pub struct RecoverySuggestion {
    pub steps: Vec<String>,
}
```

### 3. Implement Error-Rich Contract Deployment

```rust
pub async fn deploy_contract(
    &self,
    inputs: &CommandInputs,
) -> Result<CommandExecutionResult, Report<EvmDeployError>> {
    let bytecode = self.get_bytecode(&inputs.contract_path)
        .change_context(EvmDeployError::InvalidBytecode)
        .attach_printable(format!("contract path: {}", inputs.contract_path))
        .attach(ErrorDocumentation {
            help: "Ensure the contract has been compiled with 'forge build'".into(),
            example: Some("forge build --contracts contracts/MyToken.sol".into()),
            link: Some("https://docs.txtx.io/evm/compilation".into()),
        })?;

    let deployer = self.get_deployer_address(&inputs.signer)
        .change_context(EvmDeployError::NetworkError)?;

    let deployment_context = DeploymentContext {
        contract_name: inputs.contract_name.clone(),
        deployer_address: deployer.to_string(),
        network: self.network.clone(),
        estimated_cost: None,
    };

    // Estimate gas and check balance
    let gas_estimate = self.estimate_gas(&bytecode, &deployer).await
        .change_context(EvmDeployError::GasEstimationFailed)
        .attach(deployment_context.clone())?;
    
    let balance = self.get_balance(&deployer).await
        .change_context(EvmDeployError::NetworkError)?;
    
    let required_balance = gas_estimate.gas_cost + inputs.value.unwrap_or_default();
    
    if balance < required_balance {
        return Err(Report::new(EvmDeployError::InsufficientFunds)
            .attach(deployment_context)
            .attach(AccountBalance {
                address: deployer.to_string(),
                balance: format_ether(balance),
                required: format_ether(required_balance),
            })
            .attach(RecoverySuggestion {
                steps: vec![
                    format!("Send at least {} ETH to {}", format_ether(required_balance - balance), deployer),
                    format!("Or reduce deployment gas by optimizing contract size"),
                    format!("Current balance: {}", format_ether(balance)),
                ],
            })
            .attach_printable(format!(
                "Deployer {} has {} ETH but needs {} ETH",
                deployer,
                format_ether(balance),
                format_ether(required_balance)
            )));
    }

    // Proceed with deployment
    let tx_result = self.send_deployment_transaction(bytecode, gas_estimate)
        .await
        .change_context(EvmDeployError::NetworkError)
        .attach(deployment_context)
        .attach_printable("Transaction submission failed")?;

    Ok(CommandExecutionResult {
        outputs: HashMap::from([
            ("contract_address".to_string(), Value::String(tx_result.contract_address)),
            ("tx_hash".to_string(), Value::String(tx_result.hash)),
        ]),
    })
}
```

### 4. Error Display with Full Context

When an error occurs, error-stack provides rich output:

```
Error: Insufficient funds for deployment

Caused by:
  0: Deployer 0x742d35Cc6634C0532925a3b844Bc9e7595f89590 has 0.5 ETH but needs 1.2 ETH
  
Attachments:
  - DeploymentContext {
      contract_name: "MyToken",
      deployer_address: "0x742d35Cc6634C0532925a3b844Bc9e7595f89590",
      network: "mainnet",
      estimated_cost: Some("1.2 ETH")
    }
  - AccountBalance {
      address: "0x742d35Cc6634C0532925a3b844Bc9e7595f89590",
      balance: "0.5 ETH",
      required: "1.2 ETH"
    }
  - RecoverySuggestion {
      steps: [
        "Send at least 0.7 ETH to 0x742d35Cc6634C0532925a3b844Bc9e7595f89590",
        "Or reduce deployment gas by optimizing contract size",
        "Current balance: 0.5 ETH"
      ]
    }

Stack backtrace:
   0: evm::commands::deploy_contract::execute
      at addons/evm/src/commands/actions/deploy_contract.rs:145
   1: txtx_core::eval::run_action_evaluation
      at crates/txtx-core/src/eval/mod.rs:234
   2: txtx_core::supervised_runloop
      at crates/txtx-core/src/lib.rs:567
```

### 5. Programmatic Error Handling

The CLI can now provide smart error handling:

```rust
fn handle_deployment_error(error: &Report<EvmDeployError>) {
    // Check if it's an insufficient funds error
    if let Some(balance_info) = error.request_ref::<AccountBalance>() {
        println!("{} Insufficient funds detected", red!("x"));
        println!("  Current balance: {}", balance_info.balance);
        println!("  Required: {}", balance_info.required);
        
        if let Some(suggestion) = error.request_ref::<RecoverySuggestion>() {
            println!("\n{} To fix this issue:", yellow!("→"));
            for step in &suggestion.steps {
                println!("  • {}", step);
            }
        }
    }
    
    // Show deployment context for any deployment error
    if let Some(ctx) = error.request_ref::<DeploymentContext>() {
        println!("\n{} Deployment Details:", yellow!("i"));
        println!("  Contract: {}", ctx.contract_name);
        println!("  Network: {}", ctx.network);
        println!("  Deployer: {}", ctx.deployer_address);
    }
}
```

## Benefits Demonstrated

1. **Preserved Context**: Every level adds relevant information without losing previous context
2. **Actionable Errors**: Users get specific steps to resolve issues
3. **Debugging Support**: Full stack traces and attached data for developers
4. **Type Safety**: Compile-time guarantees about error handling
5. **Extensibility**: Easy to add new attachment types for different error scenarios

## Migration Impact

This example shows how a single action's error handling would improve. The same patterns would apply across:
- All blockchain addons (Bitcoin, Stacks, Solana, etc.)
- Core execution engine
- Parser and validation
- CLI user interactions

The structured approach ensures consistent, helpful error messages throughout the entire txtx system.