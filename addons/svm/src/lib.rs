#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate txtx_addon_kit;

pub mod codec;
mod commands;
mod constants;
mod functions;
pub mod rpc;
mod signers;
pub mod typing;

use constants::NAMESPACE;
use txtx_addon_kit::{
    types::{
        commands::PreCommandSpecification, functions::FunctionSpecification,
        signers::SignerSpecification,
    },
    Addon,
};

#[derive(Debug)]
pub struct SolanaNetworkAddon;

impl SolanaNetworkAddon {
    pub fn new() -> Self {
        Self {}
    }
}

impl Addon for SolanaNetworkAddon {
    fn get_name(&self) -> &str {
        "Solana Blockchain (alpha)"
    }

    fn get_description(&self) -> &str {
        txtx_addon_kit::indoc! {r#"
            Coming soon. 
            "#}
    }

    fn get_namespace(&self) -> &str {
        NAMESPACE
    }

    fn get_functions(&self) -> Vec<FunctionSpecification> {
        functions::FUNCTIONS.clone()
    }

    fn get_actions(&self) -> Vec<PreCommandSpecification> {
        commands::ACTIONS.clone()
    }

    fn get_signers(&self) -> Vec<SignerSpecification> {
        signers::SIGNERS.clone()
    }
}
