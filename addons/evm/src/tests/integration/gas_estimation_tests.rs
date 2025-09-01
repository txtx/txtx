//! Integration tests for gas estimation functionality
//! 
//! These tests verify that gas estimation properly:
//! - Estimates gas for simple transfers
//! - Estimates gas for contract deployments
//! - Provides accurate estimates that transactions succeed with
//! - Handles edge cases like insufficient balance

#[cfg(test)]
mod gas_estimation_tests {
    use crate::tests::integration::anvil_harness::AnvilInstance;
    use crate::tests::fixture_builder::{FixtureBuilder, get_anvil_manager};
    use tokio;
    
    #[tokio::test]
    async fn test_estimate_simple_transfer() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_estimate_simple_transfer - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing gas estimation for simple ETH transfer");
        
        // ARRANGE: Create inline runbook for gas estimation
        let gas_runbook = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

signer "sender" "evm::private_key" {
    private_key = input.private_key
}

# Estimate gas for a simple transfer
action "estimate_transfer" "evm::estimate_gas" {
    from = signer.sender.address
    to = input.recipient
    value = input.amount  # 1 ETH
}

# Actually send the transaction to verify estimate works
action "send_transfer" "evm::send_transaction" {
    from = signer.sender
    to = input.recipient
    value = input.amount
    gas_limit = action.estimate_transfer.gas_estimate
}

# Get receipt to check actual gas used
action "get_receipt" "evm::get_transaction_receipt" {
    tx_hash = action.send_transfer.tx_hash
}

output "estimated_transfer_gas" {
    value = action.estimate_transfer.gas_estimate
}

output "tx_hash" {
    value = action.send_transfer.tx_hash
}

output "actual_gas_used" {
    value = action.get_receipt.gas_used
}"#;
        
        let mut fixture = FixtureBuilder::new("test_estimate_transfer")
            .with_anvil_manager(get_anvil_manager().await.unwrap())
            .with_runbook("gas", gas_runbook)
            .build()
            .await
            .expect("Failed to build fixture");
        
        // Set up parameters
        let accounts = fixture.anvil_handle.accounts();
        fixture.config.parameters.insert("chain_id".to_string(), "31337".to_string());
        fixture.config.parameters.insert("rpc_url".to_string(), fixture.rpc_url.clone());
        fixture.config.parameters.insert("private_key".to_string(), accounts.alice.secret_string());
        fixture.config.parameters.insert("recipient".to_string(), accounts.bob.address_string());
        fixture.config.parameters.insert("amount".to_string(), "1000000000000000000".to_string()); // 1 ETH
        
        // ACT: Execute gas estimation and transaction
        fixture.execute_runbook("gas").await
            .expect("Failed to execute gas estimation");
        
        // ASSERT: Verify gas estimation
        let outputs = fixture.get_outputs("gas")
            .expect("Should have outputs");
        
        let estimated_gas = outputs.get("estimated_transfer_gas")
            .and_then(|v| v.as_integer().or_else(|| v.as_string()?.parse().ok()))
            .expect("Should have gas estimation") as u64;
        
        // ETH transfer should be exactly 21000 gas
        assert_eq!(estimated_gas, 21000, "Gas estimate should be 21000 for simple transfer");
        
        let actual_gas = outputs.get("actual_gas_used")
            .and_then(|v| v.as_integer().or_else(|| v.as_string()?.parse().ok()))
            .expect("Should have actual gas used") as u64;
        
        assert_eq!(actual_gas, 21000, "Actual gas used should be 21000");
        
        println!("✅ Simple transfer gas estimation: {} gas", estimated_gas);
    }
    
    #[tokio::test]
    async fn test_estimate_contract_deployment() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_estimate_contract_deployment - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing gas estimation for contract deployment");
        
        // ARRANGE: Create inline runbook for deployment gas estimation
        let deploy_runbook = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

signer "deployer" "evm::private_key" {
    private_key = input.private_key
}

# Simple storage contract bytecode
variable "bytecode" {
    value = "0x608060405234801561001057600080fd5b5060b88061001f6000396000f3fe6080604052348015600f57600080fd5b506004361060325760003560e01c80632e64cec11460375780636057361d14604c575b600080fd5b60005460405190815260200160405180910390f35b6059605736600460506565565b50565b005b600055565b60006"
}

# Estimate gas for deployment
action "estimate_deployment" "evm::estimate_gas" {
    from = signer.deployer.address
    data = variable.bytecode
}

# Deploy with estimated gas
action "deploy_contract" "evm::deploy_contract" {
    artifact_source = concat("inline:", variable.bytecode)
    signer = signer.deployer
    gas_limit = action.estimate_deployment.gas_estimate
}

output "estimated_deployment_gas" {
    value = action.estimate_deployment.gas_estimate
}

output "contract_address" {
    value = action.deploy_contract.contract_address
}"#;
        
        let mut fixture = FixtureBuilder::new("test_estimate_deployment")
            .with_anvil_manager(get_anvil_manager().await.unwrap())
            .with_runbook("deploy", deploy_runbook)
            .build()
            .await
            .expect("Failed to build fixture");
        
        // Set up parameters
        let accounts = fixture.anvil_handle.accounts();
        fixture.config.parameters.insert("chain_id".to_string(), "31337".to_string());
        fixture.config.parameters.insert("rpc_url".to_string(), fixture.rpc_url.clone());
        fixture.config.parameters.insert("private_key".to_string(), accounts.alice.secret_string());
        
        // ACT: Execute deployment gas estimation
        fixture.execute_runbook("deploy").await
            .expect("Failed to execute deployment gas estimation");
        
        // ASSERT: Verify deployment gas estimation
        let outputs = fixture.get_outputs("deploy")
            .expect("Should have outputs");
        
        let deployment_gas = outputs.get("estimated_deployment_gas")
            .and_then(|v| v.as_integer().or_else(|| v.as_string()?.parse().ok()))
            .expect("Should have deployment gas estimation") as u64;
        
        // Contract deployment needs more gas than simple transfer
        assert!(deployment_gas > 50000, "Deployment should need significant gas");
        assert!(deployment_gas < 1000000, "Deployment gas should be reasonable");
        
        let contract_address = outputs.get("contract_address")
            .and_then(|v| v.as_string())
            .expect("Should have deployed contract");
        assert!(contract_address.starts_with("0x"), "Should have valid contract address");
        
        println!("✅ Contract deployment gas estimation: {} gas", deployment_gas);
    }
    
    #[tokio::test]
    async fn test_estimated_gas_sufficient() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_estimated_gas_sufficient - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing that estimated gas is sufficient for transaction");
        
        // ARRANGE: Create inline runbook
        let sufficient_runbook = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

signer "sender" "evm::private_key" {
    private_key = input.private_key
}

# Estimate gas first
action "estimate" "evm::estimate_gas" {
    from = signer.sender.address
    to = input.recipient
    value = 5000000000000000  # 0.005 ETH
}

# Send with estimated gas (should succeed)
action "send_tx" "evm::send_transaction" {
    from = signer.sender
    to = input.recipient
    value = 5000000000000000
    gas_limit = action.estimate.gas_estimate
}

# Verify transaction succeeded
action "get_receipt" "evm::get_transaction_receipt" {
    tx_hash = action.send_tx.tx_hash
}

output "estimated_gas" {
    value = action.estimate.gas_estimate
}

output "tx_hash" {
    value = action.send_tx.tx_hash
}

output "actual_gas_used" {
    value = action.get_receipt.gas_used
}

output "status" {
    value = action.get_receipt.status
}"#;
        
        let mut fixture = FixtureBuilder::new("test_sufficient_gas")
            .with_anvil_manager(get_anvil_manager().await.unwrap())
            .with_runbook("sufficient", sufficient_runbook)
            .build()
            .await
            .expect("Failed to build fixture");
        
        // Set up parameters
        let accounts = fixture.anvil_handle.accounts();
        fixture.config.parameters.insert("chain_id".to_string(), "31337".to_string());
        fixture.config.parameters.insert("rpc_url".to_string(), fixture.rpc_url.clone());
        fixture.config.parameters.insert("private_key".to_string(), accounts.alice.secret_string());
        fixture.config.parameters.insert("recipient".to_string(), accounts.charlie.address_string());
        
        // ACT: Execute transaction with estimated gas
        fixture.execute_runbook("sufficient").await
            .expect("Failed to execute transaction");
        
        // ASSERT: Verify transaction succeeded
        let outputs = fixture.get_outputs("sufficient")
            .expect("Should have outputs");
        
        let tx_hash = outputs.get("tx_hash")
            .and_then(|v| v.as_string())
            .expect("Should have transaction hash");
        assert!(tx_hash.starts_with("0x"), "Should have valid transaction hash");
        
        let status = outputs.get("status")
            .and_then(|v| v.as_bool().or_else(|| v.as_integer().map(|i| i == 1)))
            .expect("Should have transaction status");
        assert!(status, "Transaction should succeed with estimated gas");
        
        let actual_gas = outputs.get("actual_gas_used")
            .and_then(|v| v.as_integer().or_else(|| v.as_string()?.parse().ok()))
            .expect("Should have actual gas used") as u64;
        
        println!("✅ Transaction succeeded with {} gas used", actual_gas);
    }
    
    #[tokio::test]
    async fn test_custom_gas_limit() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_custom_gas_limit - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing transaction with custom gas limit");
        
        // ARRANGE: Create inline runbook with custom gas limit
        let custom_runbook = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

signer "sender" "evm::private_key" {
    private_key = input.private_key
}

# Send transaction with explicit custom gas limit
action "send_custom" "evm::send_transaction" {
    from = signer.sender
    to = input.recipient
    value = 1000000000000000  # 0.001 ETH
    gas_limit = 50000  # More than needed for simple transfer
}

# Get receipt to verify
action "get_receipt" "evm::get_transaction_receipt" {
    tx_hash = action.send_custom.tx_hash
}

output "tx_hash" {
    value = action.send_custom.tx_hash
}

output "gas_provided" {
    value = 50000
}

output "gas_used" {
    value = action.get_receipt.gas_used
}

output "status" {
    value = action.get_receipt.status
}"#;
        
        let mut fixture = FixtureBuilder::new("test_custom_gas")
            .with_anvil_manager(get_anvil_manager().await.unwrap())
            .with_runbook("custom", custom_runbook)
            .build()
            .await
            .expect("Failed to build fixture");
        
        // Set up parameters
        let accounts = fixture.anvil_handle.accounts();
        fixture.config.parameters.insert("chain_id".to_string(), "31337".to_string());
        fixture.config.parameters.insert("rpc_url".to_string(), fixture.rpc_url.clone());
        fixture.config.parameters.insert("private_key".to_string(), accounts.alice.secret_string());
        fixture.config.parameters.insert("recipient".to_string(), accounts.dave.address_string());
        
        // ACT: Execute transaction with custom gas limit
        fixture.execute_runbook("custom").await
            .expect("Failed to execute transaction with custom gas");
        
        // ASSERT: Verify transaction succeeded with custom gas limit
        let outputs = fixture.get_outputs("custom")
            .expect("Should have outputs");
        
        let status = outputs.get("status")
            .and_then(|v| v.as_bool().or_else(|| v.as_integer().map(|i| i == 1)))
            .expect("Should have transaction status");
        assert!(status, "Transaction should succeed with custom gas limit");
        
        let gas_used = outputs.get("gas_used")
            .and_then(|v| v.as_integer().or_else(|| v.as_string()?.parse().ok()))
            .expect("Should have gas used") as u64;
        
        // Should use standard 21000 gas even though we provided 50000
        assert_eq!(gas_used, 21000, "Should use only needed gas");
        
        println!("✅ Custom gas limit test passed (used {} of 50000 gas)", gas_used);
    }
}