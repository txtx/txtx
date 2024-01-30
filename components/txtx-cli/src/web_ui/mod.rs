use rust_embed::RustEmbed;

pub mod http;

#[derive(RustEmbed)]
#[folder = "../txtx-web-ui/dist/"]
pub struct Asset;
