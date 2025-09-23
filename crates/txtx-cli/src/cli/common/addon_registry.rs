//! Shared addon registry for CLI commands
//!
//! This module provides a central place to instantiate all available addons,
//! which can be used by docs, doctor, LSP, and other commands that need
//! access to addon specifications.

use std::sync::Arc;
use txtx_addon_network_bitcoin::BitcoinNetworkAddon;
use txtx_addon_network_evm::EvmNetworkAddon;
use txtx_addon_network_svm::SvmNetworkAddon;
use txtx_addon_telegram::TelegramAddon;
use txtx_core::kit::Addon;
use txtx_core::std::StdAddon;

/// Get all available addons as a shared reference
pub fn get_all_addons() -> Arc<Vec<Box<dyn Addon>>> {
    let addons: Vec<Box<dyn Addon>> = vec![
        Box::new(StdAddon::new()),
        Box::new(BitcoinNetworkAddon::new()),
        Box::new(EvmNetworkAddon::new()),
        Box::new(SvmNetworkAddon::new()),
        Box::new(TelegramAddon::new()),
    ];

    // Add optional addons if available
    #[cfg(feature = "ovm")]
    {
        use txtx_addon_network_ovm::OvmNetworkAddon;
        addons.push(Box::new(OvmNetworkAddon::new()));
    }

    #[cfg(feature = "stacks")]
    {
        use txtx_addon_network_stacks::StacksNetworkAddon;
        addons.push(Box::new(StacksNetworkAddon::new()));
    }

    #[cfg(feature = "sp1")]
    {
        use txtx_addon_sp1::Sp1NetworkAddon;
        addons.push(Box::new(Sp1NetworkAddon::new()));
    }

    Arc::new(addons)
}

/// Extract addon specifications from addon instances
pub fn extract_addon_specifications(
    addons: &[Box<dyn Addon>],
) -> std::collections::HashMap<
    String,
    Vec<(String, txtx_core::kit::types::commands::CommandSpecification)>,
> {
    use txtx_core::kit::types::commands::PreCommandSpecification;
    let mut specifications = std::collections::HashMap::new();

    for addon in addons {
        let namespace = addon.get_namespace();
        let mut actions = Vec::new();

        for action in addon.get_actions() {
            match action {
                PreCommandSpecification::Atomic(spec) => {
                    actions.push((spec.matcher.clone(), spec));
                }
                PreCommandSpecification::Composite(spec) => {
                    // For composite actions, we'll use a simplified representation
                    // The matcher is what matters for validation
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
