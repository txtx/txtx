#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate txtx_addon_kit;

mod commands;
mod constants;
mod functions;
mod signers;
pub mod typing;

use constants::NAMESPACE;
use txtx_addon_kit::{
    types::{
        commands::{CommandInputsEvaluationResult, CommandInstance, PreCommandSpecification},
        diagnostics::Diagnostic,
        functions::FunctionSpecification,
        signers::SignerSpecification,
        AddonPostProcessingResult, ConstructDid,
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
        commands::actions::ACTIONS.clone()
    }

    fn get_signers(&self) -> Vec<SignerSpecification> {
        signers::WALLETS.clone()
    }

    fn get_domain_specific_commands_inputs_dependencies<'a>(
        self: &Self,
        _commands_instances: &'a Vec<(
            ConstructDid,
            &'a CommandInstance,
            Option<&'a CommandInputsEvaluationResult>,
        )>,
    ) -> Result<AddonPostProcessingResult, Diagnostic> {
        unimplemented!()
    }
}
