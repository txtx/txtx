// Test utilities and infrastructure tests
// These tests verify test helpers work correctly, not EVM behavior

pub mod fixture_infrastructure_tests;
pub mod anvil_infrastructure_tests;

// Re-export commonly used test utilities
pub use crate::tests::fixture_builder::{
    FixtureBuilder,
    FixtureConfig,
    TestFixture,
    NamedAccounts,
    AnvilManager,
    get_anvil_manager,
};

pub use crate::tests::integration::anvil_harness::{
    AnvilInstance,
    TestAccount,
};