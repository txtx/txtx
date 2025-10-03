//! Addon registry for tests
//! Simplified version of the CLI addon registry

use std::collections::HashMap;
use txtx_addon_kit::{types::commands::CommandSpecification, Addon};
use txtx_core::std::StdAddon;

/// Get all available addons for testing
pub fn get_all_addons() -> Vec<Box<dyn Addon>> {
    vec![
        Box::new(StdAddon::new()),
        Box::new(txtx_addon_network_bitcoin::BitcoinNetworkAddon::new()),
        Box::new(txtx_addon_network_evm::EvmNetworkAddon::new()),
        Box::new(txtx_addon_network_svm::SvmNetworkAddon::new()),
        Box::new(txtx_addon_telegram::TelegramAddon::new()),
    ]
}

/// Extract addon specifications from addon instances
pub fn extract_addon_specifications(
    addons: &[Box<dyn Addon>],
) -> HashMap<String, Vec<(String, CommandSpecification)>> {
    use txtx_addon_kit::types::commands::PreCommandSpecification;
    let mut specifications = HashMap::new();

    for addon in addons {
        let namespace = addon.get_namespace();
        let mut actions = Vec::new();

        for action in addon.get_actions() {
            match action {
                PreCommandSpecification::Atomic(spec) => {
                    actions.push((spec.matcher.clone(), spec));
                }
                PreCommandSpecification::Composite(spec) => {
                    // For composite actions, use simplified representation
                    if let Some(first_action) = spec.parts.first() {
                        if let PreCommandSpecification::Atomic(first_spec) = first_action {
                            let mut simplified = first_spec.clone();
                            simplified.name = spec.name.clone();
                            simplified.matcher = spec.matcher.clone();
                            simplified.documentation = spec.documentation.clone();
                            actions.push((spec.matcher.clone(), simplified));
                        }
                    }
                }
            }
        }

        specifications.insert(namespace.to_string(), actions);
    }

    specifications
}
