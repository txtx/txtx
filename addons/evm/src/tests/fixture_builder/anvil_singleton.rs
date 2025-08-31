// Singleton Anvil manager using OnceLock for guaranteed single instance
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use super::accounts::{NamedAccounts, TEST_MNEMONIC};

/// Global singleton instance of the Anvil process manager
static ANVIL_INSTANCE: OnceLock<Arc<Mutex<AnvilManager>>> = OnceLock::new();

/// Track if we've registered the exit handler
static EXIT_HANDLER_REGISTERED: std::sync::Once = std::sync::Once::new();

pub struct AnvilManager {
    process: Option<Child>,
    pid: Option<u32>,
    port: u16,
    url: String,
    accounts: NamedAccounts,
}

impl AnvilManager {
    /// Get or create the singleton instance
    pub fn instance() -> Arc<Mutex<Self>> {
        // Register exit handler on first access
        EXIT_HANDLER_REGISTERED.call_once(|| {
            register_exit_handler();
        });
        
        ANVIL_INSTANCE
            .get_or_init(|| {
                eprintln!("🔧 Initializing singleton Anvil manager...");
                Arc::new(Mutex::new(AnvilManager {
                    process: None,
                    pid: None,
                    port: 0, // Will be set when started
                    url: String::new(),
                    accounts: NamedAccounts::from_mnemonic(TEST_MNEMONIC)
                        .expect("Failed to create accounts"),
                }))
            })
            .clone()
    }

    /// Start the Anvil process if not already running
    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.process.is_some() && self.is_running() {
            eprintln!("✅ Anvil already running at {}", self.url);
            return Ok(());
        }

        // Find available port (prefer test ports 9545-9549 to avoid user's Anvil)
        self.port = find_available_port()?;
        self.url = format!("http://127.0.0.1:{}", self.port);
        
        eprintln!("🚀 Starting Anvil on port {}...", self.port);
        
        let mut child = Command::new("anvil")
            .arg("--port").arg(self.port.to_string())
            .arg("--accounts").arg("26")  // All 26 accounts
            .arg("--balance").arg("10000")  // 10000 ETH each
            .arg("--mnemonic").arg(TEST_MNEMONIC)
            .arg("--chain-id").arg("31337")
            .arg("--silent")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        let pid = child.id();
        eprintln!("   PID: {}", pid);
        
        // Save the PID for later cleanup
        self.pid = Some(pid);
        
        // Also write PID to a file so other processes can find it
        write_pid_file(pid)?;
        
        // Wait for Anvil to be ready
        for i in 0..30 {
            std::thread::sleep(Duration::from_millis(100));
            if check_port_listening(self.port) {
                eprintln!("   ✓ Anvil ready after {} ms", (i + 1) * 100);
                self.process = Some(child);
                return Ok(());
            }
        }
        
        // Failed to start
        let _ = child.kill();
        Err("Anvil failed to start within 3 seconds".into())
    }

    /// Check if the process is still running
    pub fn is_running(&mut self) -> bool {
        if let Some(ref mut process) = self.process {
            match process.try_wait() {
                Ok(None) => true,  // Still running
                Ok(Some(status)) => {
                    eprintln!("⚠️  Anvil exited with status: {:?}", status);
                    self.process = None;
                    false
                }
                Err(e) => {
                    eprintln!("⚠️  Error checking Anvil status: {}", e);
                    false
                }
            }
        } else {
            false
        }
    }

    /// Stop the Anvil process
    pub fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(mut process) = self.process.take() {
            let pid = process.id();
            eprintln!("🛑 Stopping Anvil (PID: {})...", pid);
            
            // Try graceful shutdown first (SIGTERM)
            #[cfg(unix)]
            {
                use std::process::Command;
                let _ = Command::new("kill")
                    .args(&["-TERM", &pid.to_string()])
                    .output();
                
                // Give it a moment to exit gracefully
                std::thread::sleep(Duration::from_millis(100));
            }
            
            // Then force kill if needed
            match process.try_wait() {
                Ok(Some(_)) => eprintln!("   Anvil stopped gracefully"),
                _ => {
                    let _ = process.kill();
                    let _ = process.wait();
                    eprintln!("   Anvil force stopped");
                }
            }
            
            // Clean up PID file
            remove_pid_file();
        } else if let Some(pid) = self.pid {
            // No process handle but we have PID - kill it directly
            eprintln!("🛑 Stopping Anvil by PID: {}...", pid);
            use std::process::Command;
            
            // Try SIGTERM first
            let _ = Command::new("kill")
                .args(&["-TERM", &pid.to_string()])
                .output();
            
            std::thread::sleep(Duration::from_millis(100));
            
            // Then SIGKILL if needed
            let _ = Command::new("kill")
                .args(&["-9", &pid.to_string()])
                .output();
            
            // Clean up PID file
            remove_pid_file();
        }
        
        self.pid = None;
        Ok(())
    }

    /// Get the RPC URL for the running instance
    pub fn rpc_url(&self) -> String {
        self.url.clone()
    }
    
    /// Get the test accounts
    pub fn accounts(&self) -> &NamedAccounts {
        &self.accounts
    }
}

impl Drop for AnvilManager {
    fn drop(&mut self) {
        // Ensure cleanup when the manager is dropped
        if self.process.is_some() {
            eprintln!("🧹 AnvilManager Drop: cleaning up Anvil process");
            let _ = self.stop();
        }
    }
}

/// Register exit handler to cleanup Anvil on process exit
fn register_exit_handler() {
    // Try to register a panic hook (but be careful not to panic during panic)
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Don't try to cleanup during panic - it can cause SIGKILL
        // Just note that cleanup should happen
        eprintln!("⚠️  Panic detected - Anvil cleanup will be handled by PID file");
        original_hook(panic_info);
    }));
    
    // Note: We can't reliably cleanup on normal exit because Rust doesn't 
    // provide exit handlers for statics. The test cleanup module should handle this.
    eprintln!("📝 Registered Anvil cleanup handlers");
}

/// Force cleanup the singleton Anvil instance
pub fn cleanup_singleton() {
    // Wrap everything in catch_unwind to prevent cleanup from panicking
    let _ = std::panic::catch_unwind(|| {
        // First try to cleanup via the singleton
        if let Some(manager) = ANVIL_INSTANCE.get() {
            if let Ok(mut guard) = manager.try_lock() {
                if guard.process.is_some() || guard.pid.is_some() {
                    eprintln!("🧹 Cleaning up singleton Anvil instance...");
                    let _ = guard.stop();
                }
            }
        }
        
        // Also check PID file and kill if exists
        cleanup_by_pid_file();
        
        // Finally, kill any test Anvil processes on test ports as fallback
        cleanup_test_anvil_processes();
    });
}

/// Kill Anvil process using saved PID file
fn cleanup_by_pid_file() {
    if let Ok(pid) = read_pid_file() {
        eprintln!("🔪 Found test Anvil PID file: {}", pid);
        use std::process::Command;
        let _ = Command::new("kill")
            .args(&["-9", &pid.to_string()])
            .output();
        remove_pid_file();
    }
}

/// Kill any Anvil processes on test ports (9545-9549)
pub fn cleanup_test_anvil_processes() {
    use std::process::Command;
    
    for port in [9545, 9546, 9547, 9548, 9549] {
        // Use lsof to find process using the port
        let output = Command::new("lsof")
            .args(&["-ti", &format!(":{}", port)])
            .output();
        
        if let Ok(output) = output {
            let pid_str = String::from_utf8_lossy(&output.stdout);
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                eprintln!("🔪 Killing test Anvil on port {} (PID: {})", port, pid);
                let _ = Command::new("kill")
                    .args(&["-9", &pid.to_string()])
                    .output();
            }
        }
    }
}

/// Helper struct for RAII-style management in tests
pub struct AnvilGuard {
    manager: Arc<Mutex<AnvilManager>>,
}

impl AnvilGuard {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let manager = AnvilManager::instance();
        manager.lock().unwrap().start()?;
        Ok(AnvilGuard { manager })
    }

    pub fn rpc_url(&self) -> String {
        self.manager.lock().unwrap().rpc_url()
    }
    
    pub fn accounts(&self) -> NamedAccounts {
        self.manager.lock().unwrap().accounts().clone()
    }
}

// Helper functions
fn find_available_port() -> Result<u16, Box<dyn std::error::Error>> {
    // Prefer test ports to avoid user's Anvil
    for port in [9545, 9546, 9547, 9548, 9549] {
        if !check_port_listening(port) {
            return Ok(port);
        }
    }
    
    // Find random port
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

fn check_port_listening(port: u16) -> bool {
    use std::net::TcpStream;
    TcpStream::connect_timeout(
        &format!("127.0.0.1:{}", port).parse().unwrap(),
        Duration::from_millis(100)
    ).is_ok()
}

/// Test harness function - runs test with Anvil
pub fn with_anvil<F, R>(test_fn: F) -> R
where
    F: FnOnce(&str, &NamedAccounts) -> R + std::panic::UnwindSafe,
{
    let manager = AnvilManager::instance();
    let mut guard = manager.lock().unwrap();
    guard.start().expect("Failed to start Anvil");
    let url = guard.rpc_url();
    let accounts = guard.accounts().clone();
    drop(guard); // Release lock before running test
    
    // Run the test, catching panics to ensure we release the lock
    let result = std::panic::catch_unwind(|| test_fn(&url, &accounts));
    
    match result {
        Ok(r) => r,
        Err(e) => std::panic::resume_unwind(e),
    }
}

// PID file management
fn pid_file_path() -> std::path::PathBuf {
    std::env::temp_dir().join("txtx_test_anvil.pid")
}

fn write_pid_file(pid: u32) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;
    let path = pid_file_path();
    fs::write(&path, pid.to_string())?;
    eprintln!("   📝 Wrote PID {} to {}", pid, path.display());
    Ok(())
}

fn read_pid_file() -> Result<u32, Box<dyn std::error::Error>> {
    use std::fs;
    let path = pid_file_path();
    let content = fs::read_to_string(&path)?;
    Ok(content.trim().parse()?)
}

fn remove_pid_file() {
    use std::fs;
    let path = pid_file_path();
    let _ = fs::remove_file(&path);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_singleton_behavior() {
        let manager1 = AnvilManager::instance();
        let manager2 = AnvilManager::instance();
        
        // Both should be the same instance
        assert!(Arc::ptr_eq(&manager1, &manager2));
        eprintln!("✅ Singleton behavior verified");
    }

    #[test]
    fn test_with_guard() -> Result<(), Box<dyn std::error::Error>> {
        let guard = AnvilGuard::new()?;
        let url = guard.rpc_url();
        assert!(url.contains("127.0.0.1"));
        eprintln!("✅ Guard pattern works");
        Ok(())
    }
    
    #[test]
    fn test_with_harness() {
        with_anvil(|url, accounts| {
            assert!(url.contains("127.0.0.1"));
            assert_eq!(accounts.alice.address_string(), "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266");
            eprintln!("✅ Test harness works");
        });
    }
}