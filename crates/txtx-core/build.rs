fn main() {
    // evm contract builds
    {
        use std::process::Command;
        let contracts_dir = "../../addons/evm/src/contracts";

        println!("cargo:rerun-if-changed=build.rs");
        println!("cargo:rerun-if-changed={}", contracts_dir);

        println!("cargo:warning=Build script evm contracts dir: {:?}", contracts_dir);

        let exit_status = Command::new("forge")
            .args(&["build"])
            .current_dir(contracts_dir)
            .status()
            .expect("Failed to run `forge build`. Ensure Foundry is installed.");
        println!("cargo:warning=EVM contract build script completed: {:?}", exit_status);
        println!("cargo:info={}", exit_status);
    }
}
