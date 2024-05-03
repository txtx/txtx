#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate txtx_addon_kit;

mod commands;
mod functions;
mod typing;

use txtx_addon_kit::{
    types::{commands::PreCommandSpecification, functions::FunctionSpecification},
    Addon,
};

#[derive(Debug)]
pub struct BitcoinNetworkAddon;

impl BitcoinNetworkAddon {
    pub fn new() -> Self {
        Self {}
    }
}

impl Addon for BitcoinNetworkAddon {
    fn get_namespace(&self) -> &str {
        "bitcoin"
    }

    fn get_functions(&self) -> Vec<FunctionSpecification> {
        functions::FUNCTIONS.clone()
    }

    fn get_actions(&self) -> Vec<PreCommandSpecification> {
        commands::actions::ACTIONS.clone()
    }

    fn get_prompts(&self) -> Vec<PreCommandSpecification> {
        commands::prompts::PROMPTS.clone()
    }
}
