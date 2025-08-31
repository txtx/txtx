//! Fixture-based testing system for EVM addon
//! 
//! This module provides a comprehensive testing framework that:
//! - Uses a single Anvil instance with snapshot/revert for test isolation
//! - Automatically augments runbooks with test outputs
//! - Provides templates for common test scenarios
//! - Handles confirmations via block mining

pub mod anvil_pool;
pub mod augmenter;
pub mod builder;
pub mod runtime;
pub mod templates;

pub use anvil_pool::{AnvilPool, AnvilHandle};
pub use augmenter::{OutputAugmenter, ActionInfo};
pub use builder::{FixtureBuilder, FixtureConfig};
pub use runtime::{TestFixture, TestCheckpoint, RunbookResult};
pub use templates::{TemplateEngine, TemplateVariables};

/// Global test configuration
#[derive(Debug, Clone)]
pub struct TestConfig {
    /// Number of default confirmations for tests
    pub default_confirmations: u32,
    /// Whether to preserve test outputs on failure
    pub preserve_on_failure: bool,
    /// Default environment name
    pub environment: String,
    /// Anvil configuration
    pub anvil: AnvilConfig,
}

#[derive(Debug, Clone)]
pub struct AnvilConfig {
    /// Port for Anvil instance
    pub port: u16,
    /// Mnemonic for deterministic accounts
    pub mnemonic: String,
    /// Number of accounts to create
    pub accounts: usize,
    /// Initial balance for accounts
    pub balance: u64,
    /// Chain ID
    pub chain_id: u64,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            default_confirmations: 0,
            preserve_on_failure: true,
            environment: "testing".to_string(),
            anvil: AnvilConfig::default(),
        }
    }
}

impl Default for AnvilConfig {
    fn default() -> Self {
        Self {
            port: 8545,
            mnemonic: "test test test test test test test test test test test junk".to_string(),
            accounts: 10,
            balance: 10000,
            chain_id: 31337,
        }
    }
}