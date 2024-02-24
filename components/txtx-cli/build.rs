use npm_rs::*;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../../txtx-web-ui");
    let exit_status = NpmEnv::default()
        .set_path(std::path::Path::new("../../../txtx-web-ui"))
        .with_node_env(&NodeEnv::Production)
        .with_env("FOO", "bar")
        .init_env()
        .install(None)
        .run("build")
        .exec()
        .unwrap();
    println!("cargo:warning={}", exit_status);
}
