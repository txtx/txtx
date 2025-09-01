// Simple test to verify Anvil manager works

#[cfg(test)]
mod tests {
    use super::super::anvil_singleton::AnvilGuard;
    use std::time::Duration;

    #[test]
    fn test_anvil_available() {
        // Check if anvil command is available
        let output = std::process::Command::new("anvil")
            .arg("--version")
            .output();
        assert!(output.is_ok(), "Anvil is not installed");
    }

    #[test]
    fn test_anvil_spawn_sync() {
        // Test using the singleton AnvilGuard
        let guard = AnvilGuard::new();
        assert!(guard.is_ok(), "Failed to get Anvil guard: {:?}", guard.err());

        let guard = guard.unwrap();
        assert!(guard.rpc_url().contains("127.0.0.1"));

        // Check accounts were created
        let accounts = guard.accounts();
        assert_eq!(accounts.names().len(), 26);

        // Guard should maintain singleton on drop
        drop(guard);

        // Give it a moment
        std::thread::sleep(Duration::from_millis(100));
    }

    #[tokio::test]
    async fn test_anvil_manager_basic() {
        use super::super::anvil_manager::AnvilManager;
        let manager = AnvilManager::new().await;
        assert!(manager.is_ok(), "Failed to create AnvilManager: {:?}", manager.err());

        let mut manager = manager.unwrap();

        // Test snapshot
        let snapshot = manager.snapshot("test").await;
        assert!(snapshot.is_ok(), "Failed to take snapshot: {:?}", snapshot.err());

        let snapshot_id = snapshot.unwrap();
        assert!(snapshot_id.starts_with("0x"));

        // Test mine blocks
        let mine_result = manager.mine_blocks(5).await;
        assert!(mine_result.is_ok(), "Failed to mine blocks: {:?}", mine_result.err());
    }
}
