#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate txtx_addon_kit;

#[macro_use]
extern crate serde_derive;

mod actions;
pub mod codec;
#[allow(dead_code)]
mod constants;
mod functions;
mod signers;
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
pub struct OvmNetworkAddon;

impl OvmNetworkAddon {
    pub fn new() -> Self {
        Self {}
    }
}

impl Addon for OvmNetworkAddon {
    fn get_name(&self) -> &str {
        "Coming Soon"
    }

    fn get_description(&self) -> &str {
        txtx_addon_kit::indoc! {r#"
            Coming Soon
        "#}
    }

    fn get_namespace(&self) -> &str {
        NAMESPACE
    }

    fn get_functions(&self) -> Vec<FunctionSpecification> {
        functions::FUNCTIONS.clone()
    }

    fn get_actions(&self) -> Vec<PreCommandSpecification> {
        actions::ACTIONS.clone()
    }

    fn get_signers(&self) -> Vec<SignerSpecification> {
        signers::SIGNERS.clone()
    }
}
