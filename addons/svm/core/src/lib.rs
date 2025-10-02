#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate txtx_addon_kit;

pub mod codec;
mod commands;
mod constants;
pub mod functions;
pub mod rpc;
mod signers;
pub mod templates;
pub mod utils;
pub use solana_pubkey::Pubkey;

pub mod typing {
    pub use txtx_addon_network_svm_types::*;
}

use constants::NAMESPACE;
use txtx_addon_kit::{
    types::{
        commands::PreCommandSpecification, functions::FunctionSpecification,
        signers::SignerSpecification,
    },
    Addon,
};
use txtx_addon_network_svm_types::SvmValue;

#[derive(Debug)]
pub struct SvmNetworkAddon;

impl SvmNetworkAddon {
    pub fn new() -> Self {
        Self {}
    }
}

impl Addon for SvmNetworkAddon {
    fn get_name(&self) -> &str {
        "Solana and SVM Compatible Blockchains (beta)"
    }

    fn get_description(&self) -> &str {
        txtx_addon_kit::indoc! {r#"
            The SVM `txtx` plugin enables building Runbooks that interact with Solana and SVM compatible blockchains. 
            The plugin provides utility functions that allow you to deploy anchor programs and encode instruction calls according to program IDLs.
            The actions can be used to create valid transfer, program call, and program deployment transactions that can be signed via a mnemonic phrase, secret key, or via your browser signer.
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

    fn to_json(
        &self,
        value: &txtx_addon_kit::types::types::Value,
    ) -> Result<Option<serde_json::Value>, txtx_addon_kit::types::diagnostics::Diagnostic> {
        SvmValue::to_json(value)
    }
}
