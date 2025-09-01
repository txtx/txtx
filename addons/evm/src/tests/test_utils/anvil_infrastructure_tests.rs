// Infrastructure tests for Anvil management
// These verify Anvil test infrastructure works correctly

#[cfg(test)]
mod anvil_infrastructure_tests {
    use crate::tests::fixture_builder::anvil_singleton::AnvilGuard;
    use crate::tests::fixture_builder::anvil_manager::AnvilManager;
    use std::time::Duration;
    
    #[test]
    fn test_anvil_command_available() {
        // ARRANGE: Prepare to check for anvil
        
        // ACT: Check if anvil command exists
        let output = std::process::Command::new("anvil")
            .arg("--version")
            .output();
        
        // ASSERT: Verify anvil is installed
        assert!(output.is_ok(), "Anvil should be installed for tests to work");
        if let Ok(output) = output {
            assert!(output.status.success(), "Anvil --version should succeed");
        }
    }
    
    #[test]
    fn test_anvil_guard_singleton() {
        // ARRANGE: Create first guard
        let guard1 = AnvilGuard::new();
        assert!(guard1.is_ok(), "First guard should succeed");
        let guard1 = guard1.unwrap();
        let url1 = guard1.rpc_url();
        
        // ACT: Create second guard (should get same instance)
        let guard2 = AnvilGuard::new();
        assert!(guard2.is_ok(), "Second guard should succeed");
        let guard2 = guard2.unwrap();
        let url2 = guard2.rpc_url();
        
        // ASSERT: Both guards point to same Anvil instance
        assert_eq!(url1, url2, "Should reuse singleton Anvil instance");
        
        // Cleanup
        drop(guard1);
        drop(guard2);
        std::thread::sleep(Duration::from_millis(100));
    }
    
    #[tokio::test]
    async fn test_anvil_manager_snapshot_revert() {
        // ARRANGE: Create manager and take initial snapshot
        let mut manager = AnvilManager::new().await
            .expect("Failed to create AnvilManager");
        
        // ACT: Take snapshot and get ID
        let snapshot_id = manager.snapshot("test_snapshot").await
            .expect("Failed to take snapshot");
        
        // ASSERT: Snapshot ID should be valid
        assert!(snapshot_id.starts_with("0x"), "Snapshot ID should be hex");
        assert!(manager.has_snapshot("test_snapshot"), "Should track snapshot");
        
        // ACT: Revert to snapshot
        let revert_result = manager.revert(&snapshot_id).await;
        
        // ASSERT: Revert should succeed
        assert!(revert_result.is_ok(), "Should be able to revert to snapshot");
    }
    
    #[tokio::test]
    async fn test_anvil_manager_mine_blocks() {
        // ARRANGE: Create manager
        let manager = AnvilManager::new().await
            .expect("Failed to create AnvilManager");
        
        // ACT: Mine some blocks
        let mine_result = manager.mine_blocks(5).await;
        
        // ASSERT: Mining should succeed
        assert!(mine_result.is_ok(), "Should be able to mine blocks");
    }
}