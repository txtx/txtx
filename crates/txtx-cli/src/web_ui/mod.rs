pub mod cloud_relayer;
pub mod http;

#[cfg(feature = "web_ui")]
use rust_embed::RustEmbed;

#[cfg(feature = "web_ui")]
#[derive(RustEmbed)]
#[folder = "../../../txtx-supervisor-ui/dist/"]
pub struct Asset;
