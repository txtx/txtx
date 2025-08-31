//! Test to validate the project setup and identify what's missing

#[cfg(test)]
mod validate_tests {
    use crate::tests::test_harness::{ProjectTestHarness, CompilationFramework};
    use crate::tests::integration::anvil_harness::AnvilInstance;
    use std::fs;
    use std::path::PathBuf;
    
    #[test]
    fn validate_project_setup() {
        println!("🔍 Validating project setup for ETH transfer test");
        
        // Check Anvil
        if !AnvilInstance::is_available() {
            println!("Anvil not available - install Foundry");
            return;
        }
        println!("Anvil is available");
        
        // Create harness without Anvil first
        let mut harness = ProjectTestHarness::new_foundry_from_fixture("integration/simple_send_eth_with_env.tx");
        
        println!("📁 Project path: {}", harness.project_path.display());
        
        // Setup the project structure
        match harness.setup() {
            Ok(_) => println!("Basic setup completed"),
            Err(e) => {
                println!("Setup failed: {}", e);
                return;
            }
        }
        
        // Check what was created
        let txtx_yml = harness.project_path.join("txtx.yml");
        if txtx_yml.exists() {
            println!("txtx.yml exists");
            if let Ok(content) = fs::read_to_string(&txtx_yml) {
                println!("   Lines: {}", content.lines().count());
            }
        } else {
            println!("txtx.yml missing");
        }
        
        let runbook = harness.project_path.join("runbooks").join(&harness.runbook_name);
        if runbook.exists() {
            println!("Runbook exists: {}", runbook.display());
            if let Ok(content) = fs::read_to_string(&runbook) {
                println!("   Lines: {}", content.lines().count());
                // Check for key elements
                if content.contains("addon \"evm\"") {
                    println!("   Has EVM addon");
                }
                if content.contains("signer") {
                    println!("   Has signer definition");
                }
                if content.contains("action") {
                    println!("   Has action");
                }
            }
        } else {
            println!("Runbook missing");
        }
        
        // Now test with Anvil
        println!("\n🔧 Testing with Anvil...");
        let mut harness_with_anvil = ProjectTestHarness::new_foundry_from_fixture("integration/simple_send_eth_with_env.tx")
            .with_anvil();
        
        match harness_with_anvil.setup() {
            Ok(_) => println!("Setup with Anvil completed"),
            Err(e) => {
                println!("Setup with Anvil failed: {}", e);
                return;
            }
        }
        
        // Check Anvil instance
        if let Some(anvil) = &harness_with_anvil.anvil {
            println!("Anvil running at: {}", anvil.url);
            println!("   Chain ID: {}", anvil.chain_id);
            println!("   Accounts: {}", anvil.accounts.len());
            if anvil.accounts.len() > 0 {
                println!("   First account: {:?}", anvil.accounts[0].address);
            }
        } else {
            println!("Anvil instance not available");
        }
        
        // Check inputs
        println!("\n📝 Inputs configured: {}", harness_with_anvil.inputs.len());
        for (key, value) in &harness_with_anvil.inputs {
            // Mask private keys
            if key.contains("key") {
                println!("   {} = [MASKED]", key);
            } else {
                println!("   {} = {}", key, value);
            }
        }
        
        // Try to validate without executing
        println!("\n🔬 Validating runbook syntax...");
        
        // Copy to debug location
        let debug_dir = PathBuf::from("/tmp/txtx_validate_setup");
        if debug_dir.exists() {
            fs::remove_dir_all(&debug_dir).ok();
        }
        copy_dir_all(&harness_with_anvil.project_path, &debug_dir).unwrap();
        println!("\n📦 Project structure preserved at: {}", debug_dir.display());
    }
    
    fn copy_dir_all(src: &PathBuf, dst: &PathBuf) -> std::io::Result<()> {
        fs::create_dir_all(&dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            
            if ty.is_dir() {
                copy_dir_all(&src_path, &dst_path)?;
            } else {
                fs::copy(&src_path, &dst_path)?;
            }
        }
        Ok(())
    }
}