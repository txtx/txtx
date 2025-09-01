//! Integration testing module with Anvil
//! 
//! This module provides integration tests against a real Ethereum node (Anvil)
//! to validate codec, RPC, and error handling functionality.

// TEMPORARILY DISABLED: Converting to new FixtureBuilder system
// The tests below are being migrated to use the new fixture builder
// system instead of ProjectTestHarness. 
//
// See src/tests/fixture_builder/ for the new testing approach.

// pub mod abi_decoding_tests;
// pub mod abi_encoding_tests;
// pub mod advanced_transaction_tests;
pub mod anvil_harness;
// pub mod comprehensive_deployment_tests;
// pub mod comprehensive_error_tests;
// pub mod event_log_tests;
// pub mod function_selector_tests;
// pub mod gas_estimation_tests;
// pub mod transaction_cost_tests;
// pub mod transaction_signing_tests;
// pub mod transaction_simulation_tests;
// pub mod transaction_types_tests;
// pub mod check_confirmations_tests;
// pub mod codec_integration_tests;
// pub mod contract_interaction_tests;
// pub mod create2_deployment_tests;
// pub mod deployment_tests;
// pub mod error_handling_tests;
// pub mod foundry_deploy_tests;
// pub mod insufficient_funds_tests;
// pub mod migrated_abi_tests;
// pub mod migrated_deployment_tests;
// pub mod migrated_transaction_tests;
// pub mod transaction_management_tests;
pub mod transaction_tests;  // This one doesn't use ProjectTestHarness
// pub mod txtx_commands_tests;
// pub mod txtx_eth_transfer_tests;
// pub mod txtx_execution_integration_tests;
// pub mod debug_unsupervised_test;
// pub mod test_confirmations_issue;
// pub mod basic_execution_test;
// pub mod test_state_reading;
// pub mod test_structured_logs;
// pub mod unicode_storage_tests;
// pub mod view_function_tests;


/// Conditionally run integration tests based on Anvil availability
/// 
/// Tests using this will:
/// - Run normally if Anvil is installed
/// - Skip with a warning message if Anvil is not available
/// - Never be marked as #[ignore]
#[cfg(test)]
#[macro_export]
macro_rules! anvil_test {
    ($name:ident, $body:expr) => {
        #[test]
        fn $name() {
            if !$crate::tests::integration::anvil_harness::AnvilInstance::is_available() {
                eprintln!("⚠️  Skipping {} - Anvil not installed", stringify!($name));
                eprintln!("    Install with: curl -L https://foundry.paradigm.xyz | bash");
                return;
            }
            
            $body()
        }
    };
}