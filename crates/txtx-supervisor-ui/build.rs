fn main() {
    {
        use npm_rs::*;
        let out_dir = std::env::var_os("OUT_DIR").unwrap();
        let out_dir =
            std::path::Path::new(&format!("{}", out_dir.to_str().unwrap())).join("supervisor");
        use std::process::Command;
        let supervisor_dir = std::path::Path::new("../../../txtx-supervisor-ui");
        let src_dir = supervisor_dir.join("src");
        let dist_dir = supervisor_dir.join("dist");

        println!("cargo:warning=------------ Supervisor Build Script ------------");
        println!("cargo:rerun-if-changed=build.rs");
        println!("cargo:rerun-if-changed={}", src_dir.display());
        println!("cargo:warning=Supervisor build script supervisor dir: {:?}", supervisor_dir);
        println!("cargo:warning=Supervisor build script output dir: {:?}", out_dir);
        let exit_status = NpmEnv::default()
            .set_path(supervisor_dir)
            .with_node_env(&NodeEnv::Production)
            .init_env()
            .install(None)
            .run("build")
            .exec()
            .unwrap();
        println!("cargo:warning=Supervisor 'npm build' completed: {:?}", exit_status);

        let cp_status = Command::new("cp")
            .args(&["-a", dist_dir.to_str().unwrap(), out_dir.to_str().unwrap()])
            .status()
            .expect("Failed to copy supervisor dist directory");
        println!(
            "cargo:warning=Copied supervisor dist directory to output directory: {:?}",
            cp_status
        );
        println!("cargo:info={}", exit_status);
        println!("cargo:info={}", cp_status);
    }
}
