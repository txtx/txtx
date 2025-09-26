use std::{fs, path::PathBuf, str::FromStr};

fn main() {
    return; // temporarily disable
    // evm contract builds
    {
        let out_dir = std::env::var_os("OUT_DIR").unwrap();
        let out_dir =
            std::path::Path::new(&format!("{}", out_dir.to_str().unwrap())).join("contracts");
        use std::process::Command;
        let mut src_contracts_dir = PathBuf::from_str("src").unwrap();
        src_contracts_dir.push("contracts");

        println!("cargo:warning=------------ EVM Build Script ------------");
        println!("cargo:rerun-if-changed=build.rs");
        println!("cargo:rerun-if-changed={}", src_contracts_dir.display());

        println!("cargo:warning=Build script evm contracts dir: {:?}", src_contracts_dir);
        println!("cargo:warning=Build script evm output dir: {:?}", out_dir);

        if !out_dir.exists() {
            std::fs::create_dir_all(&out_dir).expect("Failed to create output directory");
        }

        copy_dir_recursive(src_contracts_dir.as_path(), out_dir.as_path())
            .expect("Failed to copy contracts directory");

        println!("cargo:warning=Copied contracts directory to output directory: ok");

        let exit_status = Command::new("forge")
            .args(&["build"])
            .current_dir(out_dir)
            .status()
            .expect("Failed to run `forge build`. Ensure Foundry is installed.");
        println!("cargo:warning=EVM contract build script completed: {:?}", exit_status);
        println!("cargo:info={}", exit_status);
        println!("cargo:warning=------------ EVM Build Script Complete ------------");
    }
}

use std::path::Path;

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
