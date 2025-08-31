// Cleanup utilities for test infrastructure

use super::anvil_manager::cleanup_anvil_manager;
use std::sync::Once;

static INIT: Once = Once::new();

/// Ensure cleanup happens at process exit
pub fn ensure_cleanup_on_exit() {
    INIT.call_once(|| {
        // Register a panic hook to cleanup on panic
        let original_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            eprintln!("⚠️  Panic detected - test Anvil will be cleaned up by Drop");
            // Don't try to create a runtime here as we might already be in one
            // The Drop implementation will handle cleanup
            original_hook(panic_info);
        }));
    });
}

/// Cleanup function to be called explicitly in tests if needed
pub async fn cleanup_test_infrastructure() {
    cleanup_anvil_manager().await;
    eprintln!("✅ Test infrastructure cleaned up");
}

/// Force cleanup our test Anvil (does NOT kill user's Anvil processes)
pub fn force_cleanup_test_anvil() {
    // We can't safely force kill without knowing which process is ours
    eprintln!("⚠️  Force cleanup requested - will be handled by Drop");
    // The Drop implementation will handle cleanup when the process exits
}