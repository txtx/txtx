#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate txtx_addon_kit;

#[macro_use]
extern crate serde_derive;

mod commands;
mod functions;
mod stacks_helpers;
mod typing;

use txtx_addon_kit::{
    types::{commands::PreCommandSpecification, functions::FunctionSpecification},
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
            Stacks is a Bitcoin Layer for smart contracts; it enables smart contracts and decentralized applications to use Bitcoin as an asset and settle transactions on the Bitcoin blockchain.\n
            Stacks has knowledge of the full Bitcoin state, thanks to its Proof of Transfer consensus and Clarity language, enabling it to read from Bitcoin at any time.\n
            All transactions on the Stacks layer are automatically hashed and settled on the Bitcoin L1. Stacks blocks are secured by 100% Bitcoin hashpower. In order to re-order Stacks blocks/transactions, an attacker would have to reorg Bitcoin."#}
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

    fn get_prompts(&self) -> Vec<PreCommandSpecification> {
        commands::prompts::PROMPTS.clone()
    }
}
