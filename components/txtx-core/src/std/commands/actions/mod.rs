use txtx_addon_kit::types::commands::PreCommandSpecification;

pub mod http;
lazy_static! {
    pub static ref ACTIONS: Vec<PreCommandSpecification> = vec![http::SEND_HTTP_REQUEST.clone(),];
}
