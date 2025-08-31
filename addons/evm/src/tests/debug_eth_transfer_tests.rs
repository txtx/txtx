//! Debug version of ETH transfer test that preserves temp directory for inspection

#[cfg(test)]
mod debug_tests {
    use crate::tests::test_harness::{ProjectTestHarness, CompilationFramework};
    use crate::tests::integration::anvil_harness::AnvilInstance;
    use std::fs;
    use std::path::PathBuf;
    
    #[test]
    fn debug_eth_transfer_setup() {
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping debug test - Anvil not installed");
            return;
        }
        
        println!("🔍 Debug ETH transfer test - preserving temp directory");
        
        // Create the harness
        let mut harness = ProjectTestHarness::new_foundry_from_fixture("integration/simple_send_eth_with_env.tx")
            .with_anvil();
        
        // Get the temp directory path before setup
        let temp_path = harness.project_path.clone();
        println!("📁 Temp directory: {}", temp_path.display());
        
        // Setup the project
        println!("🔧 Setting up project...");
        match harness.setup() {
            Ok(_) => println!("Setup completed"),
            Err(e) => {
                println!("Setup failed: {:?}", e);
                print_directory_structure(&temp_path, 0);
                panic!("Setup failed");
            }
        }
        
        println!("\n📂 Directory structure after setup:");
        print_directory_structure(&temp_path, 0);
        
        // Print key file contents
        println!("\n📄 Key file contents:");
        
        // Check txtx.yml
        let txtx_yml = temp_path.join("txtx.yml");
        if txtx_yml.exists() {
            println!("\n=== txtx.yml ===");
            if let Ok(content) = fs::read_to_string(&txtx_yml) {
                println!("{}", content);
            }
        } else {
            println!("txtx.yml not found!");
        }
        
        // Check runbook
        let runbook_path = temp_path.join("runbooks").join(&harness.runbook_name);
        if runbook_path.exists() {
            println!("\n=== {} ===", harness.runbook_name);
            if let Ok(content) = fs::read_to_string(&runbook_path) {
                for (i, line) in content.lines().enumerate() {
                    println!("{:3}: {}", i + 1, line);
                }
            }
        } else {
            println!("Runbook not found at: {}", runbook_path.display());
        }
        
        // Check signers
        let signers_path = temp_path.join("runbooks").join("signers.tx");
        if signers_path.exists() {
            println!("\n=== signers.tx ===");
            if let Ok(content) = fs::read_to_string(&signers_path) {
                println!("{}", content);
            }
        } else {
            println!("⚠️  No signers.tx file");
        }
        
        // Try to execute
        println!("\n🔄 Attempting execution...");
        match harness.execute_runbook() {
            Ok(result) => {
                println!("Execution completed");
                println!("   Success: {}", result.success);
                println!("   Outputs: {:?}", result.outputs);
            },
            Err(e) => {
                println!("Execution failed: {:?}", e);
                
                // Check for run directory
                let run_dir = temp_path.join("run");
                if run_dir.exists() {
                    println!("\n📂 Run directory contents:");
                    print_directory_structure(&run_dir, 1);
                    
                    // Check for logs
                    let log_file = run_dir.join("txtx.log");
                    if log_file.exists() {
                        println!("\n=== txtx.log ===");
                        if let Ok(content) = fs::read_to_string(&log_file) {
                            for line in content.lines().take(50) {
                                println!("{}", line);
                            }
                        }
                    }
                }
            }
        }
        
        // Copy to persistent location for manual inspection
        let debug_dir = PathBuf::from("/tmp/txtx_debug_eth_transfer");
        if debug_dir.exists() {
            fs::remove_dir_all(&debug_dir).ok();
        }
        
        println!("\n📦 Copying to persistent location...");
        copy_dir_all(&temp_path, &debug_dir).unwrap();
        println!("Debug directory preserved at: {}", debug_dir.display());
        println!("   You can examine it with: ls -la {}", debug_dir.display());
        
        // Don't panic so temp dir gets preserved
    }
    
    fn print_directory_structure(dir: &PathBuf, indent: usize) {
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        
        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            
            let path = entry.path();
            let indent_str = "  ".repeat(indent);
            
            if path.is_dir() {
                println!("{}📁 {}/", indent_str, path.file_name().unwrap().to_string_lossy());
                print_directory_structure(&path, indent + 1);
            } else {
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                println!("{}📄 {} ({}B)", indent_str, path.file_name().unwrap().to_string_lossy(), size);
            }
        }
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