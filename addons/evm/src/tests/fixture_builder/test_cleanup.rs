//! Test module that ensures Anvil cleanup after all tests complete

#[cfg(test)]
mod cleanup_tests {
    use crate::tests::fixture_builder::{cleanup_test_infrastructure, force_cleanup_test_anvil, cleanup_singleton};
    
    // This test runs last alphabetically, ensuring cleanup
    #[tokio::test]
    async fn zzz_cleanup_anvil() {
        eprintln!("🧹 Running final test cleanup...");
        
        // Wrap cleanup in catch_unwind to prevent test from failing
        let _ = std::panic::catch_unwind(|| {
            // Cleanup the singleton Anvil instance
            cleanup_singleton();
        });
        
        // Call async cleanup for old manager (if any)
        let _ = cleanup_test_infrastructure().await;
        
        // Note: We do NOT force kill all anvil processes as that would
        // interfere with user's own Anvil instances
        force_cleanup_test_anvil();
        
        eprintln!("✅ Test cleanup completed (user's Anvil instances were not affected)");
    }
}