fn main() {
    #[cfg(feature = "web_ui")]
    {
        use npm_rs::*;

        println!("cargo:rerun-if-changed=build.rs");
        println!("cargo:rerun-if-changed=../../../txtx-supervisor-ui/src");
        let exit_status = NpmEnv::default()
            .set_path(std::path::Path::new("../../../txtx-supervisor-ui"))
            .with_node_env(&NodeEnv::Production)
            .init_env()
            .install(None)
            .run("build")
            .exec()
            .unwrap();
        println!("cargo:info={}", exit_status);
    }
    // evm contract builds
    {
        use std::process::Command;
        let contracts_dir = "../../addons/evm/src/contracts";

        println!("cargo:rerun-if-changed=build.rs");
        println!("cargo:rerun-if-changed={}", contracts_dir);

        let exit_status = Command::new("forge")
            .args(&["build"])
            .current_dir(contracts_dir)
            .status()
            .expect("Failed to run `forge build`. Ensure Foundry is installed.");
        println!("cargo:info={}", exit_status);
    }
}
