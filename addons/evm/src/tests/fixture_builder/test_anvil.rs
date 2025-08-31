// Simple test to verify Anvil manager works

#[cfg(test)]
mod tests {
    use super::super::anvil_manager::*;
    use std::time::Duration;
    
    #[test]
    fn test_anvil_available() {
        assert!(AnvilInstance::is_available(), "Anvil is not installed");
    }
    
    #[test]
    fn test_anvil_spawn_sync() {
        // Test without async to isolate issues
        let instance = AnvilInstance::spawn();
        assert!(instance.is_ok(), "Failed to spawn Anvil: {:?}", instance.err());
        
        let instance = instance.unwrap();
        assert_eq!(instance.chain_id, 31337);
        assert!(instance.url.contains("127.0.0.1"));
        
        // Check accounts were created
        assert_eq!(instance.accounts.names().len(), 26);
        
        // Instance should clean up on drop
        drop(instance);
        
        // Give it a moment to clean up
        std::thread::sleep(Duration::from_millis(100));
    }
    
    #[tokio::test]
    async fn test_anvil_manager_basic() {
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