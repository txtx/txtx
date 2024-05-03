use txtx_addon_kit::{
    types::{commands::PreCommandSpecification, functions::FunctionSpecification},
    Addon,
};

pub mod commands;
pub mod functions;

lazy_static! {
    pub static ref ACTIONS: Vec<PreCommandSpecification> =
        vec![commands::http::SEND_HTTP_REQUEST.clone(),];
}

#[derive(Debug)]
pub struct StdAddon;

impl StdAddon {
    pub fn new() -> Self {
        Self {}
    }
}

impl Addon for StdAddon {
    fn get_namespace(&self) -> &str {
        "std"
    }

    fn get_functions(&self) -> Vec<FunctionSpecification> {
        vec![]
    }

    fn get_actions(&self) -> Vec<PreCommandSpecification> {
        ACTIONS.clone()
    }

    fn get_prompts(&self) -> Vec<PreCommandSpecification> {
        vec![]
    }
}
