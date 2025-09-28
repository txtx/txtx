use std::path::PathBuf;
use txtx_test_utils::builders::{
    create_test_manifest_with_env, RunbookBuilder, ValidationMode,
};

/// Example implementation showcasing the enhanced RunbookBuilder pattern
///
/// This demonstrates:
/// 1. Basic runbook construction with fluent API
/// 2. Multi-mode validation (HCL-only vs Linter)
/// 3. Environment and manifest integration
/// 4. Complex runbook scenarios
/// 5. Validation error handling

fn main() {
    println!("Enhanced RunbookBuilder Examples\n");

    // Example 1: Basic runbook construction
    basic_runbook_example();

    // Example 2: Environment-aware runbook
    environment_aware_runbook_example();

    // Example 3: Multi-action workflow
    multi_action_workflow_example();

    // Example 4: Cross-chain deployment
    cross_chain_deployment_example();

    // Example 5: Validation modes comparison
    validation_modes_example();

    // Example 6: Complex DeFi workflow
    complex_defi_workflow_example();
}

/// Example 1: Basic runbook construction with fluent API
fn basic_runbook_example() {
    println!("=== Example 1: Basic Runbook Construction ===");

    let mut builder = RunbookBuilder::new()
        // Add EVM addon configuration
        .addon("evm", vec![("chain_id", "1"), ("rpc_url", "env.ETH_RPC_URL")])
        // Define a signer
        .signer("deployer", "evm::secp256k1", vec![("private_key", "env.DEPLOYER_KEY")])
        // Add a variable
        .variable("token_supply", "1000000")
        // Deploy contract action
        .action("deploy", "evm::deploy_contract")
        .input("contract", "\"./contracts/Token.sol\"")
        .input("constructor_args", "[variable.token_supply]")
        .input("signer", "signer.deployer")
        // Output the result
        .output("contract_address", "action.deploy.contract_address");

    let result = builder.validate();

    if result.success {
        println!("✓ Basic runbook validated successfully");
    } else {
        println!("✗ Validation failed:");
        for error in &result.errors {
            println!("  - {}", error.message);
        }
    }
    println!();
}

/// Example 2: Environment-aware runbook with manifest
fn environment_aware_runbook_example() {
    println!("=== Example 2: Environment-Aware Runbook ===");

    // Create a manifest with multiple environments
    let manifest = create_test_manifest_with_env(vec![
        (
            "development",
            vec![
                ("ETH_RPC_URL", "http://localhost:8545"),
                (
                    "DEPLOYER_KEY",
                    "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
                ),
                ("TOKEN_NAME", "DevToken"),
            ],
        ),
        (
            "production",
            vec![
                ("ETH_RPC_URL", "https://eth-mainnet.infura.io/v3/YOUR_KEY"),
                ("DEPLOYER_KEY", "env.PROD_DEPLOYER_KEY"),
                ("TOKEN_NAME", "ProdToken"),
            ],
        ),
    ]);

    let mut builder = RunbookBuilder::new()
        .addon("evm", vec![("rpc_url", "env.ETH_RPC_URL")])
        .variable("token_name", "env.TOKEN_NAME")
        .action("deploy", "evm::deploy_contract")
        .input("contract", "\"Token.sol\"")
        .input("constructor_args", "[variable.token_name, \"TKN\", 18]")
        .input("signer", "signer.deployer")
        .signer("deployer", "evm::secp256k1", vec![("private_key", "env.DEPLOYER_KEY")]);

    // Validate with linter mode for full validation
    let result = builder.validate_with_linter(Some(manifest), Some("development".to_string()));

    println!(
        "Validation result for development environment: {}",
        if result.success { "✓ Success" } else { "✗ Failed" }
    );
    println!();
}

/// Example 3: Multi-action workflow with dependencies
fn multi_action_workflow_example() {
    println!("=== Example 3: Multi-Action Workflow ===");

    let mut builder = RunbookBuilder::new()
        .addon("evm", vec![("chain_id", "1")])
        // Deploy token contract
        .action("deploy_token", "evm::deploy_contract")
        .input("contract", "\"Token.sol\"")
        .input("constructor_args", "[\"MyToken\", \"MTK\", 1000000]")
        // Deploy DEX contract
        .action("deploy_dex", "evm::deploy_contract")
        .input("contract", "\"DEX.sol\"")
        .input("depends_on", "[action.deploy_token]")
        // Add liquidity
        .action("add_liquidity", "evm::call")
        .input("contract", "action.deploy_dex.contract_address")
        .input("method", "\"addLiquidity\"")
        .input("args", "[action.deploy_token.contract_address, 100000]")
        .input("depends_on", "[action.deploy_dex]")
        // Output results
        .output("token_address", "action.deploy_token.contract_address")
        .output("dex_address", "action.deploy_dex.contract_address")
        .output("liquidity_tx", "action.add_liquidity.tx_hash");

    let result = builder.validate();
    println!(
        "Multi-action workflow validation: {}",
        if result.success { "✓ Success" } else { "✗ Failed" }
    );
    println!();
}

/// Example 4: Cross-chain deployment scenario
fn cross_chain_deployment_example() {
    println!("=== Example 4: Cross-Chain Deployment ===");

    let mut builder = RunbookBuilder::new()
        // Configure multiple chains
        .addon("mainnet", vec![("type", "evm"), ("chain_id", "1"), ("rpc_url", "env.MAINNET_RPC")])
        .addon(
            "optimism",
            vec![("type", "evm"), ("chain_id", "10"), ("rpc_url", "env.OPTIMISM_RPC")],
        )
        .addon(
            "arbitrum",
            vec![("type", "evm"), ("chain_id", "42161"), ("rpc_url", "env.ARBITRUM_RPC")],
        )
        // Deploy on mainnet
        .action("deploy_mainnet", "mainnet::deploy_contract")
        .input("contract", "\"MultiChainToken.sol\"")
        .input("constructor_args", "[\"MCT\", 1000000000]")
        // Deploy on Optimism
        .action("deploy_optimism", "optimism::deploy_contract")
        .input("contract", "\"MultiChainToken.sol\"")
        .input("constructor_args", "[\"MCT\", 1000000000]")
        .input("depends_on", "[action.deploy_mainnet]")
        // Deploy on Arbitrum
        .action("deploy_arbitrum", "arbitrum::deploy_contract")
        .input("contract", "\"MultiChainToken.sol\"")
        .input("constructor_args", "[\"MCT\", 1000000000]")
        .input("depends_on", "[action.deploy_mainnet]")
        // Bridge setup
        .action("setup_bridge", "mainnet::call")
        .input("contract", "action.deploy_mainnet.contract_address")
        .input("method", "\"setRemoteTokens\"")
        .input(
            "args",
            "[action.deploy_optimism.contract_address, action.deploy_arbitrum.contract_address]",
        )
        .input("depends_on", "[action.deploy_optimism, action.deploy_arbitrum]");

    let result = builder.validate();
    println!(
        "Cross-chain deployment validation: {}",
        if result.success { "✓ Success" } else { "✗ Failed" }
    );
    println!();
}

/// Example 5: Comparing validation modes
fn validation_modes_example() {
    println!("=== Example 5: Validation Modes Comparison ===");

    // Create a runbook with intentional issues
    let runbook = || {
        RunbookBuilder::new()
            .addon("evm", vec![])
            .action("test", "evm::send_eth")
            .input("to", "\"0x123\"")
            .input("value", "\"1000\"")
            .input("signer", "signer.undefined_signer") // Undefined signer
            .output("result", "action.test.invalid_field")
    }; // Invalid field

    // Test 1: HCL-only validation
    let mut builder1 = runbook();
    let hcl_result = builder1.validate();
    println!("HCL-only validation: {}", if hcl_result.success { "✓ Passed" } else { "✗ Failed" });
    if !hcl_result.errors.is_empty() {
        println!("  Errors detected: {}", hcl_result.errors.len());
    }

    // Test 2: Linter validation (would catch more issues)
    let mut builder2 = runbook();
    let lint_result = builder2.validate_with_mode(ValidationMode::Linter {
        manifest: None,
        environment: None,
        file_path: Some(PathBuf::from("test.tx")),
    });
    println!("Linter validation: {}", if lint_result.success { "✓ Passed" } else { "✗ Failed" });
    if !lint_result.errors.is_empty() {
        println!("  Errors detected: {}", lint_result.errors.len());
        for error in &lint_result.errors {
            println!("    - {}", error.message);
        }
    }

    println!();
}

/// Example 6: Complex DeFi workflow
fn complex_defi_workflow_example() {
    println!("=== Example 6: Complex DeFi Workflow ===");

    let mut builder = RunbookBuilder::new()
        // Environment setup
        .with_environment(
            "production",
            vec![
                ("ETH_RPC_URL", "https://eth-mainnet.infura.io/v3/KEY"),
                ("TREASURY_KEY", "0x..."),
                ("INITIAL_LIQUIDITY", "1000000"),
            ],
        )
        // CLI inputs for dynamic configuration
        .with_cli_input("token_name", "DeFiToken")
        .with_cli_input("token_symbol", "DFT")
        // Addons
        .addon("evm", vec![("rpc_url", "env.ETH_RPC_URL")])
        // Signers
        .signer("treasury", "evm::secp256k1", vec![("private_key", "env.TREASURY_KEY")])
        // Variables
        .variable("token_name", "input.token_name")
        .variable("token_symbol", "input.token_symbol")
        .variable("initial_supply", "100000000")
        .variable("initial_liquidity", "env.INITIAL_LIQUIDITY")
        // Deploy governance token
        .action("deploy_token", "evm::deploy_contract")
        .input("contract", "\"GovernanceToken.sol\"")
        .input(
            "constructor_args",
            "[variable.token_name, variable.token_symbol, variable.initial_supply]",
        )
        .input("signer", "signer.treasury")
        // Deploy timelock controller
        .action("deploy_timelock", "evm::deploy_contract")
        .input("contract", "\"TimelockController.sol\"")
        .input("constructor_args", "[86400, [], []]") // 24h delay
        .input("signer", "signer.treasury")
        // Deploy governor
        .action("deploy_governor", "evm::deploy_contract")
        .input("contract", "\"Governor.sol\"")
        .input(
            "constructor_args",
            "[action.deploy_token.contract_address, action.deploy_timelock.contract_address]",
        )
        .input("signer", "signer.treasury")
        .input("depends_on", "[action.deploy_token, action.deploy_timelock]")
        // Deploy treasury
        .action("deploy_treasury", "evm::deploy_contract")
        .input("contract", "\"Treasury.sol\"")
        .input("constructor_args", "[action.deploy_timelock.contract_address]")
        .input("signer", "signer.treasury")
        .input("depends_on", "[action.deploy_timelock]")
        // Deploy AMM pool
        .action("deploy_pool", "evm::deploy_contract")
        .input("contract", "\"AMMPool.sol\"")
        .input("constructor_args", "[action.deploy_token.contract_address]")
        .input("signer", "signer.treasury")
        .input("depends_on", "[action.deploy_token]")
        // Add initial liquidity
        .action("add_liquidity", "evm::call")
        .input("contract", "action.deploy_pool.contract_address")
        .input("method", "\"addLiquidity\"")
        .input("args", "[variable.initial_liquidity]")
        .input("value", "variable.initial_liquidity")
        .input("signer", "signer.treasury")
        .input("depends_on", "[action.deploy_pool]")
        // Transfer ownership to governance
        .action("transfer_ownership", "evm::call")
        .input("contract", "action.deploy_token.contract_address")
        .input("method", "\"transferOwnership\"")
        .input("args", "[action.deploy_timelock.contract_address]")
        .input("signer", "signer.treasury")
        .input("depends_on", "[action.deploy_governor, action.add_liquidity]")
        // Outputs
        .output("token_address", "action.deploy_token.contract_address")
        .output("governor_address", "action.deploy_governor.contract_address")
        .output("timelock_address", "action.deploy_timelock.contract_address")
        .output("treasury_address", "action.deploy_treasury.contract_address")
        .output("pool_address", "action.deploy_pool.contract_address")
        .output("liquidity_added", "action.add_liquidity.tx_hash");

    // Build manifest from the builder
    let manifest = builder.build_manifest();

    // Validate with linter mode
    let result = builder.validate_with_linter(Some(manifest), Some("production".to_string()));

    println!(
        "Complex DeFi workflow validation: {}",
        if result.success { "✓ Success" } else { "✗ Failed" }
    );

    if !result.errors.is_empty() {
        println!("\nErrors found:");
        for error in &result.errors {
            println!("  - {}", error.message);
        }
    }

    if !result.warnings.is_empty() {
        println!("\nWarnings:");
        for warning in &result.warnings {
            println!("  - {}", warning.message);
        }
    }

    println!();
}

/// Advanced example: Testing validation edge cases
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_state_management() {
        // Test that builder properly manages state between actions
        let mut builder = RunbookBuilder::new()
            .action("first", "evm::deploy_contract")
            .input("contract", "\"First.sol\"")
            .action("second", "evm::deploy_contract") // Should close first action
            .input("contract", "\"Second.sol\"");

        let content = builder.build_content();
        assert!(content.contains("action \"first\""));
        assert!(content.contains("action \"second\""));
        assert_eq!(content.matches('}').count(), 2); // Both actions closed
    }

    #[test]
    fn test_value_formatting() {
        // Test that builder properly formats different value types
        let mut builder = RunbookBuilder::new()
            .variable("string_var", "hello") // Should be quoted
            .variable("ref_var", "env.TEST") // Should not be quoted
            .variable("action_ref", "action.test.output") // Should not be quoted
            .action("test", "evm::call")
            .input("number", "42") // Should not be quoted
            .input("signer_ref", "signer.test") // Should not be quoted
            .input("string", "test value"); // Should be quoted

        let content = builder.build_content();
        assert!(content.contains("value = \"hello\""));
        assert!(content.contains("value = env.TEST"));
        assert!(content.contains("value = action.test.output"));
        assert!(content.contains("number = 42"));
        assert!(content.contains("signer_ref = signer.test"));
        assert!(content.contains("string = \"test value\""));
    }

    #[test]
    fn test_multi_file_support() {
        // Test multi-file runbook construction
        let builder = RunbookBuilder::new()
            .with_file("contracts/Token.sol", "contract Token { ... }")
            .with_file("scripts/deploy.js", "const deploy = async () => { ... }")
            .with_content(
                r#"
                addon "evm" {}
                action "deploy" "evm::deploy_contract" {
                    contract = "./contracts/Token.sol"
                }
            "#,
            );

        assert_eq!(builder.file_count(), 2);
        assert!(builder.has_file("contracts/Token.sol"));
    }

    #[test]
    fn test_manifest_generation() {
        // Test that builder correctly generates manifests
        let builder = RunbookBuilder::new()
            .with_environment(
                "dev",
                vec![("API_KEY", "dev-key"), ("RPC_URL", "http://localhost:8545")],
            )
            .with_environment(
                "prod",
                vec![("API_KEY", "prod-key"), ("RPC_URL", "https://mainnet.infura.io")],
            );

        let manifest = builder.build_manifest();
        assert_eq!(manifest.environments.len(), 2);
        assert_eq!(manifest.environments["dev"]["API_KEY"], "dev-key");
        assert_eq!(manifest.environments["prod"]["RPC_URL"], "https://mainnet.infura.io");
    }
}

// Note: assert_validation_error and assert_success macros are already imported from txtx_test_utils
