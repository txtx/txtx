// Anvil manager with snapshot/revert for efficient test isolation

use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use tokio::sync::Mutex;
use serde_json::json;
use reqwest::Client;
use std::time::Duration;

use super::accounts::{NamedAccounts, TestAccount};

/// Manages a single shared Anvil instance with snapshot/revert capabilities
pub struct AnvilManager {
    instance: AnvilInstance,
    snapshots: HashMap<String, String>,  // test_name -> snapshot_id
    client: Client,
}

impl AnvilManager {
    /// Create a new Anvil manager with a single instance
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        eprintln!("📍 AnvilManager::new() called - spawning NEW Anvil process");
        let instance = AnvilInstance::spawn()?;
        let client = Client::new();
        
        Ok(Self {
            instance,
            snapshots: HashMap::new(),
            client,
        })
    }
    
    /// Take a snapshot for a test
    pub async fn snapshot(&mut self, test_name: &str) -> Result<String, Box<dyn std::error::Error>> {
        let response = self.client
            .post(&self.instance.url)
            .json(&json!({
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
            .ok_or("Failed to get snapshot ID")?
            .to_string();
        
        self.snapshots.insert(test_name.to_string(), snapshot_id.clone());
        eprintln!("📸 Snapshot taken for test '{}': {}", test_name, snapshot_id);
        
        Ok(snapshot_id)
    }
    
    /// Revert to a snapshot
    pub async fn revert(&mut self, snapshot_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        eprintln!("🔄 Reverting to snapshot: {}", snapshot_id);
        
        let response = self.client
            .post(&self.instance.url)
            .json(&json!({
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
        
        // Clean up snapshots that were deleted (>= reverted snapshot)
        let reverted_id = snapshot_id.trim_start_matches("0x");
        if let Ok(reverted_num) = u64::from_str_radix(reverted_id, 16) {
            self.snapshots.retain(|_, id| {
                let id_num = id.trim_start_matches("0x");
                u64::from_str_radix(id_num, 16).unwrap_or(u64::MAX) < reverted_num
            });
        }
        
        eprintln!("✅ Successfully reverted to snapshot {}", snapshot_id);
        Ok(())
    }
    
    /// Mine blocks (fast-forward)
    pub async fn mine_blocks(&self, blocks: u32) -> Result<(), Box<dyn std::error::Error>> {
        eprintln!("⛏️  Mining {} blocks...", blocks);
        
        let response = self.client
            .post(&self.instance.url)
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "hardhat_mine",
                "params": [format!("0x{:x}", blocks)],
                "id": 1
            }))
            .send()
            .await?;
        
        let result: serde_json::Value = response.json().await?;
        
        if result.get("error").is_some() {
            return Err(format!("Failed to mine {} blocks: {:?}", blocks, result["error"]).into());
        }
        
        eprintln!("✅ Successfully mined {} blocks", blocks);
        Ok(())
    }
    
    /// Get the RPC URL
    pub fn url(&self) -> &str {
        &self.instance.url
    }
    
    /// Get the accounts
    pub fn accounts(&self) -> &NamedAccounts {
        &self.instance.accounts
    }
    
    /// Get a handle for a specific test
    pub async fn get_handle(&mut self, test_name: &str) -> Result<AnvilHandle, Box<dyn std::error::Error>> {
        // Take initial snapshot for this test
        let snapshot_id = self.snapshot(test_name).await?;
        
        Ok(AnvilHandle {
            test_name: test_name.to_string(),
            snapshot_id,
            url: self.instance.url.clone(),
            accounts: self.instance.accounts.clone(),
        })
    }
    
    /// Cleanup a test (revert to its snapshot)
    pub async fn cleanup_test(&mut self, test_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(snapshot_id) = self.snapshots.get(test_name).cloned() {
            self.revert(&snapshot_id).await?;
        }
        Ok(())
    }
    
    /// Check if a snapshot exists (for testing)
    #[cfg(test)]
    pub fn has_snapshot(&self, test_name: &str) -> bool {
        self.snapshots.contains_key(test_name)
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
    /// Get the RPC URL
    pub fn url(&self) -> &str {
        &self.url
    }
    
    /// Get the test accounts
    pub fn accounts(&self) -> &NamedAccounts {
        &self.accounts
    }
}

/// Anvil instance
pub struct AnvilInstance {
    process: Option<Child>,
    pub process_id: Option<u32>,
    pub url: String,
    pub chain_id: u64,
    pub accounts: NamedAccounts,
}

impl AnvilInstance {
    /// Check if Anvil is available
    pub fn is_available() -> bool {
        Command::new("anvil")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    
    /// Spawn a new Anvil instance
    pub fn spawn() -> Result<Self, Box<dyn std::error::Error>> {
        if !Self::is_available() {
            return Err("Anvil not found. Please install Foundry: curl -L https://foundry.paradigm.xyz | bash".into());
        }
        
        // Find an available port
        let port = find_available_port()?;
        
        eprintln!("🚀 Starting Anvil on port {}...", port);
        
        // Start anvil with deterministic accounts
        let mut child = Command::new("anvil")
            .arg("--port").arg(port.to_string())
            .arg("--accounts").arg("26")  // Generate all 26 accounts
            .arg("--balance").arg("10000")  // 10000 ETH per account
            .arg("--mnemonic").arg(super::accounts::TEST_MNEMONIC)
            .arg("--chain-id").arg("31337")
            // Note: --block-time 0 is not valid, omit for instant mining (default)
            .arg("--silent")  // Reduce output noise
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn anvil: {}", e))?;
        
        let process_id = child.id();
        eprintln!("  📝 Anvil PID: {}", process_id);
        
        // Wait for Anvil to start with retries
        let url = format!("http://127.0.0.1:{}", port);
        let mut ready = false;
        for i in 0..20 {  // More retries with shorter sleep
            std::thread::sleep(Duration::from_millis(100));
            if check_anvil_ready(port) {
                ready = true;
                eprintln!("  ✓ Anvil ready after {} ms", (i + 1) * 100);
                break;
            }
        }
        
        if !ready {
            // Try to get stderr for debugging
            if let Some(stderr) = child.stderr.take() {
                use std::io::Read;
                let mut err_output = String::new();
                let _ = std::io::BufReader::new(stderr).read_to_string(&mut err_output);
                eprintln!("Anvil stderr: {}", err_output);
            }
            let _ = child.kill();
            return Err(format!("Anvil failed to start on port {} after 2 seconds", port).into());
        }
        
        eprintln!("✅ Anvil started successfully on {}", url);
        
        // Create named accounts
        let accounts = NamedAccounts::from_anvil()?;
        
        Ok(Self {
            process: Some(child),
            process_id: Some(process_id),
            url,
            chain_id: 31337,
            accounts,
        })
    }
}

impl Drop for AnvilInstance {
    fn drop(&mut self) {
        // Only kill the process we started
        if let Some(mut process) = self.process.take() {
            if let Some(pid) = self.process_id {
                eprintln!("🛑 Stopping our Anvil instance at {} (PID: {})...", self.url, pid);
            } else {
                eprintln!("🛑 Stopping our Anvil instance at {}...", self.url);
            }
            
            // Try graceful shutdown first
            let _ = process.kill();
            // Wait for it to actually exit
            match process.wait() {
                Ok(status) => eprintln!("   Anvil exited with status: {:?}", status),
                Err(e) => eprintln!("   Error waiting for Anvil to exit: {}", e),
            }
        }
        // Note: We do NOT kill by PID separately as that could kill a user's process
        // The Child handle should be sufficient
    }
}

/// Find an available port for Anvil
fn find_available_port() -> Result<u16, Box<dyn std::error::Error>> {
    // Start with test-specific ports to avoid conflicting with user's Anvil (usually on 8545)
    for port in [9545, 9546, 9547, 9548, 9549] {
        if port_is_available(port) {
            eprintln!("   Using port {} for test Anvil", port);
            return Ok(port);
        }
    }
    
    // Find a random available port if test ports are busy
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    eprintln!("   Using random port {} for test Anvil", port);
    Ok(port)
}

/// Check if a port is available
fn port_is_available(port: u16) -> bool {
    use std::net::TcpListener;
    TcpListener::bind(format!("127.0.0.1:{}", port)).is_ok()
}

/// Check if Anvil is ready by checking if port is listening (like nc -z)
fn check_anvil_ready(port: u16) -> bool {
    use std::net::TcpStream;
    use std::time::Duration;
    
    // Just check if we can connect to the port (equivalent to nc -z)
    TcpStream::connect_timeout(
        &format!("127.0.0.1:{}", port).parse().unwrap(),
        Duration::from_millis(100)
    ).is_ok()
}

use std::sync::OnceLock;

/// Global singleton Anvil manager using OnceLock for true singleton behavior
static ANVIL_MANAGER: OnceLock<Arc<Mutex<AnvilManager>>> = OnceLock::new();

/// Get or create the global Anvil manager
pub async fn get_anvil_manager() -> Result<Arc<Mutex<AnvilManager>>, Box<dyn std::error::Error>> {
    // Ensure cleanup is registered
    super::cleanup::ensure_cleanup_on_exit();
    
    // Check if already initialized
    if let Some(manager) = ANVIL_MANAGER.get() {
        eprintln!("♻️  Reusing existing Anvil manager");
        return Ok(manager.clone());
    }
    
    // Need to create - this is thread-safe due to OnceLock
    eprintln!("🔧 Creating singleton Anvil manager (one instance for all tests)...");
    eprintln!("   ⚠️  Note: Anvil will be automatically cleaned up on exit");
    
    let new_manager = Arc::new(Mutex::new(AnvilManager::new().await?));
    
    // get_or_init ensures only one initialization even with concurrent access
    Ok(ANVIL_MANAGER.get_or_init(|| new_manager).clone())
}

/// Clean up the global Anvil manager (kills ONLY our Anvil process)
pub async fn cleanup_anvil_manager() {
    eprintln!("🧹 Cleaning up test Anvil instance...");
    
    // With OnceLock, we can't take ownership, but we can access it
    if let Some(manager_arc) = ANVIL_MANAGER.get() {
        let mut manager = manager_arc.lock().await;
        
        if manager.instance.process_id.is_some() {
            eprintln!("   Stopping Anvil PID: {:?}", manager.instance.process_id);
            // Clone accounts before the replace
            let accounts_clone = manager.instance.accounts.clone();
            // Take the process to trigger Drop
            let instance = std::mem::replace(&mut manager.instance, AnvilInstance {
                process: None,
                process_id: None,
                url: String::new(),
                chain_id: 0,
                accounts: accounts_clone,
            });
            drop(instance); // This will kill the process
            eprintln!("✅ Test Anvil instance terminated");
        } else {
            eprintln!("ℹ️  No test Anvil process to clean up");
        }
    } else {
        eprintln!("ℹ️  No Anvil manager initialized");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_anvil_manager_snapshot_revert() {
        let mut manager = AnvilManager::new().await.unwrap();
        
        // Take a snapshot
        let snapshot1 = manager.snapshot("test1").await.unwrap();
        assert!(snapshot1.starts_with("0x"));
        
        // Take another snapshot
        let snapshot2 = manager.snapshot("test2").await.unwrap();
        assert!(snapshot2 != snapshot1);
        
        // Revert to first snapshot
        manager.revert(&snapshot1).await.unwrap();
        
        // Second snapshot should be gone
        assert!(!manager.snapshots.contains_key("test2"));
    }
    
    #[tokio::test]
    async fn test_mine_blocks() {
        let manager = AnvilManager::new().await.unwrap();
        
        // Mine 10 blocks
        manager.mine_blocks(10).await.unwrap();
        
        // Could verify block number increased by making RPC call
    }
}