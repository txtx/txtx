use std::path::{Path, PathBuf};
use std::fs;
use std::panic;
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Test directory that preserves itself on panic/failure
pub struct PanicAwareTestDir {
    path: PathBuf,
    test_name: String,
    preserve_on_failure: bool,
    test_failed: bool,
}

impl PanicAwareTestDir {
    /// Create a new test directory that will be preserved on panic
    pub fn new(test_name: &str) -> std::io::Result<Self> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let dir_name = format!("txtx_test_{}_{}", test_name, timestamp);
        let path = env::temp_dir().join(dir_name);
        
        fs::create_dir_all(&path)?;
        
        Ok(PanicAwareTestDir {
            path,
            test_name: test_name.to_string(),
            preserve_on_failure: true,
            test_failed: false,
        })
    }
    
    pub fn path(&self) -> &Path {
        &self.path
    }
    
    pub fn mark_success(&mut self) {
        self.test_failed = false;
    }
    
    pub fn mark_failure(&mut self) {
        self.test_failed = true;
    }
}

impl Drop for PanicAwareTestDir {
    fn drop(&mut self) {
        if !self.test_failed && !self.preserve_on_failure {
            let _ = fs::remove_dir_all(&self.path);
            eprintln!("✅ Test succeeded - cleaned up: {}", self.path.display());
        } else if self.test_failed || self.preserve_on_failure {
            eprintln!("\n════════════════════════════════════════");
            eprintln!("⚠️  TEST FAILED - Directory preserved:");
            eprintln!("📁 {}", self.path.display());
            eprintln!("════════════════════════════════════════");
            
            // List directory contents
            if let Ok(entries) = fs::read_dir(&self.path) {
                eprintln!("\nContents:");
                for entry in entries.flatten() {
                    if let Ok(metadata) = entry.metadata() {
                        let type_str = if metadata.is_dir() { "📂" } else { "📄" };
                        eprintln!("  {} {}", type_str, entry.file_name().to_string_lossy());
                    }
                }
            }
            
            eprintln!("\nTo inspect:");
            eprintln!("  cd {}", self.path.display());
            eprintln!("  find . -name '*.tx' | head -5");
            eprintln!("  cat txtx.yml");
            eprintln!("════════════════════════════════════════\n");
        }
    }
}

/// Execute a test with panic handling and directory preservation
pub async fn with_panic_handler<F, Fut, R>(
    test_name: &str,
    test_fn: F,
) -> R
where
    F: FnOnce(PathBuf) -> Fut + panic::UnwindSafe,
    Fut: std::future::Future<Output = R>,
{
    let mut test_dir = PanicAwareTestDir::new(test_name)
        .expect("Failed to create test directory");
    
    let path = test_dir.path().to_path_buf();
    

    
    // Run the async test (without custom panic handling since we can't move the hook)
    let result = test_fn(path).await;
    
    // Check if panic occurred via thread panicking status
    if std::thread::panicking() {
        test_dir.mark_failure();
    } else {
        test_dir.mark_success();
    }
    
    result
}

/// Wrapper for fixture-based tests with panic handling
pub struct PanicAwareFixture {
    pub project_dir: PathBuf,
    test_dir: Option<PanicAwareTestDir>,
    pub rpc_url: String,
}

impl PanicAwareFixture {
    pub async fn new(
        test_name: &str,
        rpc_url: String,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let test_dir = PanicAwareTestDir::new(test_name)?;
        let project_dir = test_dir.path().to_path_buf();
        
        // Create basic structure
        fs::create_dir_all(project_dir.join("src"))?;
        fs::create_dir_all(project_dir.join("runbooks"))?;
        fs::create_dir_all(project_dir.join("runs/testing"))?;
        
        Ok(PanicAwareFixture {
            project_dir,
            test_dir: Some(test_dir),
            rpc_url,
        })
    }
    
    pub fn mark_success(&mut self) {
        if let Some(ref mut dir) = self.test_dir {
            dir.mark_success();
        }
    }
    
    pub fn mark_failure(&mut self) {
        if let Some(ref mut dir) = self.test_dir {
            dir.mark_failure();
        }
    }
    
    /// Run test with automatic panic detection
    pub async fn run_test<F, R>(&mut self, test_fn: F) -> Result<R, Box<dyn std::error::Error + Send + Sync>>
    where
        F: FnOnce(&Path, &str) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<R, Box<dyn std::error::Error + Send + Sync>>> + Send>>,
        R: Send + 'static,
    {
        let project_dir = self.project_dir.clone();
        let rpc_url = self.rpc_url.clone();
        
        // Use tokio's panic handling
        let handle = tokio::spawn(async move {
            test_fn(&project_dir, &rpc_url).await
        });
        
        match handle.await {
            Ok(Ok(result)) => {
                self.mark_success();
                Ok(result)
            }
            Ok(Err(e)) => {
                self.mark_failure();
                Err(e)
            }
            Err(panic_err) => {
                self.mark_failure();
                Err(format!("Test panicked: {:?}", panic_err).into())
            }
        }
    }
}

/// Simple test runner that preserves on ANY failure or panic
pub async fn run_preserving_test<F>(test_name: &str, test_fn: F)
where
    F: FnOnce(&Path) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + Send>>,
{
    let test_dir = env::temp_dir().join(format!("txtx_test_{}_debug", test_name));
    let _ = fs::create_dir_all(&test_dir);
    
    eprintln!("🧪 Running test: {}", test_name);
    eprintln!("📁 Test directory: {}", test_dir.display());
    
    match test_fn(&test_dir).await {
        Ok(_) => {
            // Success - clean up
            let _ = fs::remove_dir_all(&test_dir);
            eprintln!("✅ Test passed - cleaned up directory");
        }
        Err(e) => {
            // Failed - preserve and show location
            eprintln!("\n════════════════════════════════════════");
            eprintln!("🔴 TEST FAILED: {}", e);
            eprintln!("📁 Debug files preserved at:");
            eprintln!("   {}", test_dir.display());
            eprintln!("════════════════════════════════════════");
            
            // List files for convenience
            if let Ok(entries) = fs::read_dir(&test_dir) {
                eprintln!("\nContents:");
                for entry in entries.flatten() {
                    eprintln!("  - {}", entry.file_name().to_string_lossy());
                }
            }
            eprintln!();
            
            panic!("Test failed: {}", e);
        }
    }
}