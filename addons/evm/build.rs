fn main() {
    // evm contract builds
    {
        let out_dir = std::env::var_os("OUT_DIR").unwrap();
        let out_dir =
            std::path::Path::new(&format!("{}", out_dir.to_str().unwrap())).join("contracts");
        use std::process::Command;
        let src_contracts_dir = "./src/contracts/.";

        println!("cargo:warning=------------ EVM Build Script ------------");
        println!("cargo:rerun-if-changed=build.rs");
        println!("cargo:rerun-if-changed={}", src_contracts_dir);

        println!("cargo:warning=Build script evm contracts dir: {:?}", src_contracts_dir);
        println!("cargo:warning=Build script evm output dir: {:?}", out_dir);

        if !out_dir.exists() {
            std::fs::create_dir_all(&out_dir).expect("Failed to create output directory");
        }

        let cp_status = Command::new("cp")
            .args(&["-a", src_contracts_dir, out_dir.to_str().unwrap()])
            .status()
            .expect("Failed to copy contracts directory");
        println!("cargo:warning=Copied contracts directory to output directory: {:?}", cp_status);

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
