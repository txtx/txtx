#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate txtx_addon_kit;

mod commands;
mod functions;
mod typing;

use txtx_addon_kit::{
    types::{
        commands::PreCommandSpecification, functions::FunctionSpecification,
        wallets::WalletSpecification,
    },
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
    fn get_name(&self) -> &str {
        "Bitcoin"
    }

    fn get_description(&self) -> &str {
        "Lorem ipsum"
    }

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

    fn get_wallets(&self) -> Vec<WalletSpecification> {
        vec![]
    }
}
