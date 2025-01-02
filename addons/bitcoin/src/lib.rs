#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate txtx_addon_kit;

pub mod codec;
mod commands;
pub mod constants;
mod functions;
mod typing;

use constants::NAMESPACE;
use txtx_addon_kit::{
    types::{
        commands::PreCommandSpecification, functions::FunctionSpecification,
        signers::SignerSpecification,
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
        txtx_addon_kit::indoc! {r#"
            The Bitcoin `txtx` addon enables building Runbooks that interact with the Bitcoin blockchain.
            Currently the Bitcoin addon can be used to encode Bitcoin Script.
        "#}
    }

    fn get_namespace(&self) -> &str {
        NAMESPACE
    }

    fn get_functions(&self) -> Vec<FunctionSpecification> {
        functions::FUNCTIONS.clone()
    }

    fn get_actions(&self) -> Vec<PreCommandSpecification> {
        commands::actions::ACTIONS.clone()
    }

    fn get_signers(&self) -> Vec<SignerSpecification> {
        vec![]
    }
}
