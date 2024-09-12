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
mod signers;
pub mod typing;
mod utils;

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use clarity::vm::types::QualifiedContractIdentifier;
use constants::NAMESPACE;
use txtx_addon_kit::{
    types::{
        commands::{CommandInputsEvaluationResult, CommandInstance, PreCommandSpecification},
        diagnostics::Diagnostic,
        functions::FunctionSpecification,
        signers::SignerSpecification,
        AddonPostProcessingResult, ConstructDid, ContractSourceTransform,
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
            The actions can be used to create valid transfer, contract call, and contract deployment transactions that can be signed via a secret key, mnemonic phrase, or via your browser signer. 
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
        commands_instances: &'a Vec<(
            ConstructDid,
            &'a CommandInstance,
            Option<&'a CommandInputsEvaluationResult>,
        )>,
    ) -> Result<AddonPostProcessingResult, Diagnostic> {
        // Isolate all the contract deployments
        // - Loop 1: For each construct did, compute the contract address, create a lookup: address -> (construct_did, action.name)
        // - Loop 2: For each construct that has some contract_ids_dependencies (lazy or not),
        //      Lookup
        //      Augment `depends_on` with action.name
        let mut additional_transforms = HashMap::new();

        let mut overrides = HashMap::new();
        let mut contracts_lookup = HashMap::new();
        for (construct_did, command_instance, inputs_simulation) in commands_instances.into_iter() {
            if command_instance.specification.matcher.eq("deploy_contract")
                || command_instance.specification.matcher.eq("deploy_requirement")
            {
                let Some(simulated_inputs) = inputs_simulation else {
                    continue;
                };
                let contract_id = simulated_inputs.inputs.get_expected_string("contract_id")?;
                contracts_lookup.insert(contract_id.to_string(), construct_did.clone());
            }
        }

        for (construct_did, command_instance, inputs_simulation) in commands_instances.into_iter() {
            let mut consolidated_dependencies = vec![];
            if command_instance.specification.matcher.eq("deploy_contract")
                || command_instance.specification.matcher.eq("deploy_requirement")
            {
                let Some(simulated_inputs) = inputs_simulation else {
                    continue;
                };

                let dependencies =
                    simulated_inputs.inputs.get_expected_array("dependency_contract_ids")?;
                for dep in dependencies.iter() {
                    let contract_id = dep.expect_string();
                    let construct = match contracts_lookup.get(contract_id) {
                        Some(construct) => construct,
                        None => {
                            println!("Missing dependency: {}", contract_id);
                            continue;
                        }
                    };
                    consolidated_dependencies.push(construct.clone());
                }

                let dependencies =
                    simulated_inputs.inputs.get_expected_array("lazy_dependency_contract_ids")?;
                for dep in dependencies.iter() {
                    let contract_id = dep.expect_string();
                    let construct = match contracts_lookup.get(contract_id) {
                        Some(construct) => construct,
                        None => {
                            println!("Missing dependency: {}", contract_id);
                            continue;
                        }
                    };
                    consolidated_dependencies.push(construct.clone());
                }
            }
            overrides.insert(construct_did.clone(), consolidated_dependencies);
        }

        // we should handle call-contract dependencies (only if litteral)?

        for (construct_did, command_instance, inputs_simulation) in commands_instances.iter() {
            let Some(simulated_inputs) = inputs_simulation else {
                continue;
            };
            let Some(contract_id) = simulated_inputs.inputs.get_string("contract_id") else {
                continue;
            };
            let from = QualifiedContractIdentifier::parse(contract_id).unwrap().name.to_string();
            let Some(to) = simulated_inputs.inputs.get_string("contract_instance_name") else {
                continue;
            };
            if command_instance.specification.matcher.eq("deploy_contract") {
                for (contract, dependencies) in overrides.iter() {
                    if dependencies.contains(construct_did) {
                        additional_transforms
                            .entry(contract.clone())
                            .or_insert_with(Vec::new)
                            .push(ContractSourceTransform::FindAndReplace(
                                format!(".{}", from),
                                format!(".{}", to),
                            ));
                    }
                }
            } else if command_instance.specification.matcher.eq("deploy_requirement") {
                for (contract, _) in overrides.iter() {
                    additional_transforms.entry(contract.clone()).or_insert_with(Vec::new).push(
                        ContractSourceTransform::FindAndReplace(
                            format!("'{}", contract_id),
                            format!(".{}", to),
                        ),
                    );
                }
            }
        }
        let res = AddonPostProcessingResult {
            dependencies: overrides,
            transforms: additional_transforms,
        };
        Ok(res)
    }
}
