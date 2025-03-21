use std::fs;
use std::process::Command;
use std::{env, path::Path};

fn main() {
    {
        use npm_rs::*;
        let out_dir = env::var_os("OUT_DIR").unwrap();
        let out_dir = Path::new(&format!("{}", out_dir.to_str().unwrap())).join("supervisor");
        let local_dist_dir = Path::new("supervisor-dist");

        println!("cargo:warning=------------ Supervisor Build Script ------------");
        println!("cargo:rerun-if-changed=build.rs");
        println!("cargo:warning=Supervisor build script output dir: {:?}", out_dir);

        if local_dist_dir.exists() {
            println!("cargo:warning=Using prebuilt `supervisor-dist` for cargo publish");

            fs::create_dir_all(&out_dir).unwrap();

            let cp_status = Command::new("cp")
                .args(&["-a", local_dist_dir.to_str().unwrap(), out_dir.to_str().unwrap()])
                .status()
                .expect("Failed to copy supervisor dist directory");
            println!("cargo:info={}", cp_status);
        } else {
            println!("cargo:warning=Running npm build in txtx-supervisor-ui");

            let supervisor_dir = Path::new("../../../txtx-supervisor-ui");
            let src_dir = supervisor_dir.join("src");
            let dist_dir = supervisor_dir.join("dist");

            println!("cargo:rerun-if-changed={}", src_dir.display());
            println!("cargo:warning=Supervisor build script supervisor dir: {:?}", supervisor_dir);

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
}
