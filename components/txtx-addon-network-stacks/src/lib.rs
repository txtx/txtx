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

use std::collections::HashMap;

use txtx_addon_kit::{
    types::{
        commands::{CommandInputsEvaluationResult, CommandInstance, PreCommandSpecification},
        diagnostics::Diagnostic,
        functions::FunctionSpecification,
        wallets::WalletSpecification,
        ConstructDid,
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
        "Stacks Blockchain (beta)"
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

    fn get_domain_specific_commands_inputs_dependencies<'a>(
        self: &Self,
        commands_instances: &'a Vec<(
            ConstructDid,
            &'a CommandInstance,
            Option<&'a CommandInputsEvaluationResult>,
        )>,
    ) -> Result<HashMap<ConstructDid, Vec<ConstructDid>>, Diagnostic> {
        // Isolate all the contract deployments
        // - Loop 1: For each construct did, compute the contract address, create a lookup: address -> (construct_did, action.name)
        // - Loop 2: For each construct that has some contract_ids_dependencies (lazy or not),
        //      Lookup
        //      Augment `depends_on` with action.name
        let mut overrides = HashMap::new();
        let mut contracts_lookup = HashMap::new();
        for (construct_did, command_instance, inputs_simulation) in commands_instances.into_iter() {
            if command_instance.specification.matcher.eq("deploy_contract") {
                let Some(simulated_inputs) = inputs_simulation else {
                    continue;
                };
                let contract_id = simulated_inputs.inputs.get_expected_string("contract_id").unwrap();
                contracts_lookup.insert(contract_id, construct_did.clone());
            }
        }

        for (construct_did, command_instance, inputs_simulation) in commands_instances.into_iter() {
            let mut consolidated_dependencies = vec![];
            if command_instance.specification.matcher.eq("deploy_contract") {
                let Some(simulated_inputs) = inputs_simulation else {
                    continue;
                };

                let dependencies = simulated_inputs
                    .inputs
                    .get_expected_array("contracts_ids_dependencies")?;
                for dep in dependencies.iter() {
                    let contract_id = dep.expect_string();
                    let Some(construct) = contracts_lookup.get(contract_id) else {
                        println!("Missing dependencies");
                        continue;
                    };
                    consolidated_dependencies.push(construct.clone());
                }

                let dependencies = simulated_inputs
                    .inputs
                    .get_expected_array("contracts_ids_lazy_dependencies")?;
                for dep in dependencies.iter() {
                    let contract_id = dep.expect_string();
                    let Some(construct) = contracts_lookup.get(contract_id) else {
                        println!("Missing dependencies");
                        continue;
                    };
                    consolidated_dependencies.push(construct.clone());
                }
            }
            overrides.insert(construct_did.clone(), consolidated_dependencies);
        }
        Ok(overrides)
    }
}
