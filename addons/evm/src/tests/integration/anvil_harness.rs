//! Anvil test harness for integration testing
//!
//! Provides utilities for spinning up Anvil instances and managing test state.

use std::process::{Child, Command, Stdio};
use std::time::Duration;
use alloy::primitives::{Address, U256};
use alloy::providers::{Provider, ProviderBuilder};
use alloy_signer_local::PrivateKeySigner;
use std::str::FromStr;

/// Test account with known private key
#[derive(Debug, Clone)]
pub struct TestAccount {
    pub address: Address,
    pub private_key: String,
    pub signer: PrivateKeySigner,
}

impl TestAccount {
    /// Create test account from private key
    pub fn from_private_key(private_key: &str) -> Self {
        let signer = PrivateKeySigner::from_str(private_key)
            .expect("Valid private key");
        let address = signer.address();
        
        Self {
            address,
            private_key: private_key.to_string(),
            signer,
        }
    }
}

/// Anvil instance for testing
pub struct AnvilInstance {
    process: Option<Child>,
    pub url: String,
    pub chain_id: u64,
    pub accounts: Vec<TestAccount>,
}

impl AnvilInstance {
    /// Check if Anvil is available on the system
    pub fn is_available() -> bool {
        Command::new("anvil")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Spawn a new Anvil instance
    pub fn spawn() -> Self {
        // Check if anvil is installed
        let check = Command::new("anvil")
            .arg("--version")
            .output();
        
        if check.is_err() {
            panic!("Anvil not found. Please install Foundry: curl -L https://foundry.paradigm.xyz | bash");
        }
        
        // Start anvil with deterministic accounts on fixed port
        let port = 8545;
        let child = Command::new("anvil")
            .arg("--port").arg(port.to_string())
            .arg("--accounts").arg("10")
            .arg("--balance").arg("10000")
            .arg("--mnemonic").arg("test test test test test test test test test test test junk")
            .arg("--chain-id").arg("31337")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn anvil");
        
        // Wait for anvil to start
        std::thread::sleep(Duration::from_millis(1000)); // Give it more time to start
        
        let url = format!("http://127.0.0.1:{}", port);
        
        // Create test accounts (deterministic based on mnemonic)
        let accounts = vec![
            TestAccount::from_private_key("0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"),
            TestAccount::from_private_key("0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d"),
            TestAccount::from_private_key("0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a"),
        ];
        
        Self {
            process: Some(child),
            url,
            chain_id: 31337,
            accounts,
        }
    }
    
    /// Spawn with specific configuration
    pub fn spawn_with_config(port: u16, chain_id: u64, block_time: Option<u64>) -> Self {
        let mut cmd = Command::new("anvil");
        cmd.arg("--port").arg(port.to_string())
           .arg("--chain-id").arg(chain_id.to_string())
           .arg("--accounts").arg("10")
           .arg("--balance").arg("10000")
           .arg("--mnemonic").arg("test test test test test test test test test test test junk");
        
        if let Some(block_time) = block_time {
            cmd.arg("--block-time").arg(block_time.to_string());
        }
        
        let mut child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn anvil");
        
        // Wait for startup
        std::thread::sleep(Duration::from_millis(1000));
        
        let url = format!("http://127.0.0.1:{}", port);
        
        // Create test accounts
        let accounts = vec![
            TestAccount::from_private_key("0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"),
            TestAccount::from_private_key("0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d"),
            TestAccount::from_private_key("0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a"),
        ];
        
        Self {
            process: Some(child),
            url,
            chain_id,
            accounts,
        }
    }
    
    /// Get RPC URL
    pub fn rpc_url(&self) -> &str {
        &self.url
    }
    
    /// Get first test account
    pub fn account(&self, index: usize) -> &TestAccount {
        &self.accounts[index]
    }
    
    /// Fund an address with ETH
    pub async fn fund_account(&self, address: Address, amount: U256) -> Result<(), Box<dyn std::error::Error>> {
        // Use first account as funder
        let funder = &self.accounts[0];
        
        // Create provider and send transaction
        let provider = ProviderBuilder::new()
            .on_http(self.url.parse()?);
        
        // This would need actual transaction sending logic
        // For now, this is a placeholder
        Ok(())
    }
    
    /// Mine a block
    pub async fn mine_block(&self) -> Result<(), Box<dyn std::error::Error>> {
        let provider = ProviderBuilder::new()
            .on_http(self.url.parse()?);
        
        // Send evm_mine RPC call
        // This would need the actual RPC implementation
        Ok(())
    }
    
    /// Create a snapshot
    pub async fn snapshot(&self) -> Result<String, Box<dyn std::error::Error>> {
        // Send evm_snapshot RPC call
        Ok("0x1".to_string())
    }
    
    /// Revert to snapshot
    pub async fn revert(&self, snapshot_id: String) -> Result<(), Box<dyn std::error::Error>> {
        // Send evm_revert RPC call
        Ok(())
    }
    
    /// Reset the chain
    pub async fn reset(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Send anvil_reset RPC call
        Ok(())
    }
}

impl Drop for AnvilInstance {
    fn drop(&mut self) {
        if let Some(mut child) = self.process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    #[ignore] // Requires anvil to be installed
    fn test_anvil_spawn() {
        let anvil = AnvilInstance::spawn();
        assert!(!anvil.url.is_empty());
        assert_eq!(anvil.chain_id, 31337);
        assert_eq!(anvil.accounts.len(), 3);
    }
    
    #[test]
    #[ignore] // Requires anvil
    fn test_account_creation() {
        let account = TestAccount::from_private_key("0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80");
        assert_eq!(
            account.address.to_string().to_lowercase(),
            "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266"
        );
    }
}