use txtx_addon_kit::{
    types::{
        commands::PreCommandSpecification, functions::FunctionSpecification,
        signers::SignerSpecification,
    },
    Addon,
};

use self::{commands::actions::ACTIONS, functions::FUNCTIONS};

pub mod commands;
pub mod functions;
pub mod signers;
pub mod typing;

#[derive(Debug)]
pub struct StdAddon;

impl StdAddon {
    pub fn new() -> Self {
        Self {}
    }
}

impl Addon for StdAddon {
    fn get_name(&self) -> &str {
        "Standard Library"
    }

    fn get_description(&self) -> &str {
        txtx_addon_kit::indoc! {r#"
      `txtx` standard commands and functions provide base functionality that can be used to build Runbooks.
      "#}
    }

    fn get_namespace(&self) -> &str {
        "std"
    }

    fn get_functions(&self) -> Vec<FunctionSpecification> {
        FUNCTIONS.clone()
    }

    fn get_actions(&self) -> Vec<PreCommandSpecification> {
        ACTIONS.clone()
    }

    fn get_signers(&self) -> Vec<SignerSpecification> {
        vec![]
    }
}
