use txtx_addon_kit::{
    types::{commands::PreCommandSpecification, functions::FunctionSpecification},
    Addon,
};

use self::{commands::actions::ACTIONS, functions::FUNCTIONS};

pub mod commands;
pub mod functions;

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
        FUNCTIONS.clone()
    }

    fn get_actions(&self) -> Vec<PreCommandSpecification> {
        ACTIONS.clone()
    }

    fn get_prompts(&self) -> Vec<PreCommandSpecification> {
        vec![]
    }
}
