use std::fs;
use std::{env, path::Path};

fn main() {
    return; // TODO: commented for local build
    #[cfg(not(feature = "bypass_supervisor_build"))]
    {
        use npm_rs::*;
        let out_dir = env::var_os("OUT_DIR").unwrap();
        let out_dir = Path::new(&out_dir.to_str().unwrap().to_string()).join("supervisor");
        let local_dist_dir = Path::new("supervisor-dist");

        println!("cargo:warning=------------ Supervisor Build Script ------------");
        println!("cargo:rerun-if-changed=build.rs");
        println!("cargo:warning=Supervisor build script output dir: {:?}", out_dir);

        if local_dist_dir.exists() {
            println!("cargo:warning=Using prebuilt `supervisor-dist` for cargo publish");

            fs::create_dir_all(&out_dir).unwrap();

            copy_dir_recursive(local_dist_dir, &out_dir)
                .expect("Failed to copy supervisor dist directory");
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

            copy_dir_recursive(&dist_dir, &out_dir)
                .expect("Failed to copy supervisor dist directory");

            println!("cargo:warning=Copied supervisor dist directory to output directory: ok");
            println!("cargo:info={}", exit_status);
        }
    }
}

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
