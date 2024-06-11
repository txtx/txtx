#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate txtx_addon_kit;

#[macro_use]
extern crate serde_derive;

pub mod codec;
mod commands;
mod constants;
mod functions;
pub mod rpc;
mod stacks_helpers;
mod typing;
mod utils;
mod wallets;

use txtx_addon_kit::{
    types::{
        commands::PreCommandSpecification, functions::FunctionSpecification,
        wallets::WalletSpecification,
    },
    Addon,
};

#[derive(Debug)]
pub struct StacksNetworkAddon;

impl StacksNetworkAddon {
    pub fn new() -> Self {
        Self {}
    }
}

impl Addon for StacksNetworkAddon {
    fn get_name(&self) -> &str {
        "Stacks Blockchain"
    }

    fn get_description(&self) -> &str {
        txtx_addon_kit::indoc! {r#"
            The Stacks `txtx` plugin enables building Runbooks that interact with the Stacks blockchain. 
            The plugin provides utility functions that allow you to encode data in the proper Clarity format that is required by contracts on the Stacks blockchain.
            The actions can be used to create valid transfer, contract call, and contract deployment transactions that can be signed via a mnemonic phrase or via your browser wallet. 
            "#}
    }

    fn get_namespace(&self) -> &str {
        "stacks"
    }

    fn get_functions(&self) -> Vec<FunctionSpecification> {
        functions::FUNCTIONS.clone()
    }

    fn get_actions(&self) -> Vec<PreCommandSpecification> {
        commands::actions::ACTIONS.clone()
    }

    fn get_wallets(&self) -> Vec<WalletSpecification> {
        wallets::WALLETS.clone()
    }
}
