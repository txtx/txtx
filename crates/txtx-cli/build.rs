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
}
