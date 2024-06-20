use rust_embed::RustEmbed;

mod cloud_relayer;
pub mod http;

#[derive(RustEmbed)]
#[folder = "../../../txtx-web-ui/dist/"]
pub struct Asset;
