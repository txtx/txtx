// Anvil manager v2 - uses singleton pattern
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use super::anvil_singleton::AnvilGuard;
use super::accounts::NamedAccounts;

/// Wrapper around the singleton that provides snapshot/revert functionality
pub struct AnvilManager {
    guard: AnvilGuard,
    snapshots: HashMap<String, String>,
    client: reqwest::Client,
}

impl AnvilManager {
    /// Create a new manager (connects to singleton Anvil)
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let guard = AnvilGuard::new()?;
        
        Ok(Self {
            guard,
            snapshots: HashMap::new(),
            client: reqwest::Client::new(),
        })
    }
    
    /// Take a snapshot
    pub async fn snapshot(&mut self, name: &str) -> Result<String, Box<dyn std::error::Error>> {
        eprintln!("📸 Taking snapshot: {}", name);
        
        let response = self.client
            .post(&self.guard.rpc_url())
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "evm_snapshot",
                "params": [],
                "id": 1
            }))
            .send()
            .await?;
        
        let result: serde_json::Value = response.json().await?;
        let snapshot_id = result["result"]
            .as_str()
            .ok_or("Invalid snapshot response")?
            .to_string();
        
        self.snapshots.insert(name.to_string(), snapshot_id.clone());
        eprintln!("   Snapshot ID: {}", snapshot_id);
        Ok(snapshot_id)
    }
    
    /// Revert to a snapshot
    pub async fn revert(&mut self, snapshot_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        eprintln!("🔄 Reverting to snapshot: {}", snapshot_id);
        
        let response = self.client
            .post(&self.guard.rpc_url())
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "evm_revert",
                "params": [snapshot_id],
                "id": 1
            }))
            .send()
            .await?;
        
        let result: serde_json::Value = response.json().await?;
        
        if !result["result"].as_bool().unwrap_or(false) {
            return Err(format!("Failed to revert to snapshot {}", snapshot_id).into());
        }
        
        eprintln!("✅ Successfully reverted");
        Ok(())
    }
    
    /// Get handle for a test
    pub async fn get_handle(&mut self, test_name: &str) -> Result<AnvilHandle, Box<dyn std::error::Error>> {
        // Take a snapshot for this test
        let snapshot_id = if !self.snapshots.contains_key(test_name) {
            self.snapshot(test_name).await?
        } else {
            // Revert to existing snapshot
            let id = self.snapshots[test_name].clone();
            self.revert(&id).await?;
            id
        };
        
        Ok(AnvilHandle {
            test_name: test_name.to_string(),
            snapshot_id,
            url: self.guard.rpc_url(),
            accounts: self.guard.accounts(),
        })
    }
    
    /// Mine blocks
    pub async fn mine_blocks(&self, blocks: u32) -> Result<(), Box<dyn std::error::Error>> {
        eprintln!("⛏️  Mining {} blocks...", blocks);
        
        for _ in 0..blocks {
            self.client
                .post(&self.guard.rpc_url())
                .json(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "evm_mine",
                    "params": [],
                    "id": 1
                }))
                .send()
                .await?;
        }
        
        eprintln!("✅ Mined {} blocks", blocks);
        Ok(())
    }
    
    /// Check if we have a snapshot
    pub fn has_snapshot(&self, name: &str) -> bool {
        self.snapshots.contains_key(name)
    }
}

/// Handle to Anvil for a specific test
pub struct AnvilHandle {
    pub test_name: String,
    pub snapshot_id: String,
    pub url: String,
    pub accounts: NamedAccounts,
}

impl AnvilHandle {
    pub fn url(&self) -> &str {
        &self.url
    }
    
    pub fn accounts(&self) -> &NamedAccounts {
        &self.accounts
    }
}

/// Global manager instance using the singleton
static MANAGER: std::sync::OnceLock<Arc<Mutex<AnvilManager>>> = std::sync::OnceLock::new();

/// Get the global Anvil manager (singleton-backed)
pub async fn get_anvil_manager() -> Result<Arc<Mutex<AnvilManager>>, Box<dyn std::error::Error>> {
    // Try to get existing
    if let Some(manager) = MANAGER.get() {
        return Ok(manager.clone());
    }
    
    // Create new manager
    eprintln!("🔧 Creating Anvil manager (singleton-backed)...");
    let manager = Arc::new(Mutex::new(AnvilManager::new().await?));
    
    // Store it (race-safe with get_or_init)
    Ok(MANAGER.get_or_init(|| manager).clone())
}

/// Clean up (for compatibility)
pub async fn cleanup_anvil_manager() {
    eprintln!("🧹 Cleanup requested - singleton will handle it");
    // The singleton handles its own cleanup via Drop
}