#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate txtx_addon_kit;

#[macro_use]
extern crate serde_derive;

mod codec;
mod commands;
#[allow(dead_code)]
mod constants;
mod functions;
pub mod rpc;
mod signers;
mod typing;

use txtx_addon_kit::{
    types::{
        commands::PreCommandSpecification, functions::FunctionSpecification,
        signers::SignerSpecification,
    },
    Addon,
};

#[derive(Debug)]
pub struct EVMNetworkAddon;

impl EVMNetworkAddon {
    pub fn new() -> Self {
        Self {}
    }
}

impl Addon for EVMNetworkAddon {
    fn get_name(&self) -> &str {
        "Ethereum and EVM Compatible Blockchains (alpha)"
    }

    fn get_description(&self) -> &str {
        txtx_addon_kit::indoc! {r#"
            The EVM `txtx` plugin enables building Runbooks that interact with Ethereum and EVM compatible blockchains. 
            The plugin provides utility functions that allow you to encode data in the proper RLP format that is required by contracts on EVM compatible blockchains.
            The actions can be used to create valid transfer, contract call, and contract deployment transactions that can be signed via a mnemonic phrase or via your browser signer. 
            "#}
    }

    fn get_namespace(&self) -> &str {
        "evm"
    }

    fn get_functions(&self) -> Vec<FunctionSpecification> {
        functions::FUNCTIONS.clone()
    }

    fn get_actions(&self) -> Vec<PreCommandSpecification> {
        commands::actions::ACTIONS.clone()
    }

    fn get_signers(&self) -> Vec<SignerSpecification> {
        signers::WALLETS.clone()
    }
}
