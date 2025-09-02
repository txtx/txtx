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
pub mod comprehensive_error_tests;
pub mod basic_execution_test;
// pub mod panic_aware_tests;  // Has compilation issues - using simple_panic_tests instead
pub mod simple_panic_tests;
pub mod validated_tests;
pub mod validate_all_runbooks;
pub mod minimal_test;
pub mod integer_vs_string_test;
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