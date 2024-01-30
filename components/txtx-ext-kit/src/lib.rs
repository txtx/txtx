#[macro_use]
extern crate serde_derive;

pub use hcl_edit as hcl;

pub mod helpers;
pub mod types;

pub trait Codec {
    /// Get network name
    fn get_supported_network(&self) -> String;

    /// Get supported encoders
    fn get_supported_encoders(&self) -> Vec<String>;

    /// Get supported decoders
    fn get_supported_decoders(&self) -> Vec<String>;
}
