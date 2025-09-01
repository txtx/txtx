
#[cfg(test)]
mod confirmations_tests {
    use super::*;
    use crate::tests::integration::anvil_harness::AnvilInstance;

    #[tokio::test]
    async fn test_eth_transfer_no_confirmations() {
        eprintln!("🔍 TEST STARTING - test_eth_transfer_no_confirmations");
        
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test - Anvil not installed");
            return;
        }
        
        eprintln!("🚀 Testing ETH transfer with 0 confirmations");
        
        // Create test harness with the no-confirmations fixture
        let mut harness = ProjectTestHarness::new_foundry_from_fixture("integration/simple_send_eth_no_confirmations.tx")
            ;
        
        // Setup the project
        harness.setup().expect("Project setup should succeed");
        
        eprintln!("📋 Executing ETH transfer with 0 confirmations...");
        
        // Execute directly
        let execution_result = result.execute().await;
        
        match execution_result {
            Ok(result) => {
                eprintln!("✅ Execution completed successfully");
                eprintln!("Outputs: {:?}", result.outputs);
                assert!(result.success, "Execution should succeed");
                assert!(result.outputs.contains_key("tx_hash"), "Should have tx_hash output");
                eprintln!("Transaction hash: {:?}", result.outputs.get("tx_hash"));
            }
            Err(e) => {
                panic!("❌ Execution failed: {:?}", e);
            }
        }
        
        eprintln!("✅ Test completed successfully");
    }
    
    #[tokio::test]
    async fn test_eth_transfer_with_1_confirmation() {
        eprintln!("🔍 TEST STARTING - test_eth_transfer_with_1_confirmation");
        
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test - Anvil not installed");
            return;
        }
        
        eprintln!("🚀 Testing ETH transfer with default 1 confirmation");
        
        // Create test harness with the standard fixture (1 confirmation default)
        let mut harness = ProjectTestHarness::new_foundry_from_fixture("integration/simple_send_eth_with_env.tx")
            ;
        
        // Setup the project
        harness.setup().expect("Project setup should succeed");
        
        eprintln!("📋 Executing ETH transfer with 1 confirmation (may hang if confirmations are the issue)...");
        
        // Try to execute with a timeout mechanism
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::thread;
        use std::time::{Duration, Instant};
        
        let harness = Arc::new(std::sync::Mutex::new(harness));
        let completed = Arc::new(AtomicBool::new(false));
        let completed_clone = completed.clone();
        
        let handle = thread::spawn(move || {
            let mut harness = harness.lock().unwrap();
            let result = result.execute().await;
            completed_clone.store(true, Ordering::Relaxed);
            result
        });
        
        // Wait max 5 seconds
        let start = Instant::now();
        while !completed.load(Ordering::Relaxed) && start.elapsed() < Duration::from_secs(5) {
            thread::sleep(Duration::from_millis(100));
        }
        
        if completed.load(Ordering::Relaxed) {
            match handle.join().unwrap() {
                Ok(result) => {
                    eprintln!("✅ Execution completed within timeout");
                    eprintln!("Outputs: {:?}", result.outputs);
                    assert!(result.success, "Execution should succeed");
                }
                Err(e) => {
                    eprintln!("❌ Execution failed: {:?}", e);
                }
            }
        } else {
            eprintln!("⏱️ Test timed out after 5 seconds - confirmations are likely the issue!");
            eprintln!("This confirms that waiting for confirmations is blocking test execution.");
            panic!("Test execution hanging on confirmations");
        }
    }
}