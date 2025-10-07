//! Function documentation generation for LSP hover support
//!
//! This module generates hover documentation for all functions from all addons
//! at compile time, ensuring we always have up-to-date documentation.

use lazy_static::lazy_static;
use std::collections::HashMap;
use txtx_addon_kit::types::functions::FunctionSpecification;
use txtx_addon_kit::types::signers::SignerSpecification;
use txtx_addon_kit::Addon;

/// Generate hover documentation for a function specification
fn generate_function_hover_text(spec: &FunctionSpecification) -> String {
    let mut content = String::new();

    // Function signature
    content.push_str(&format!("### `{}`\n\n", spec.name));

    // Documentation
    content.push_str(&spec.documentation);
    content.push_str("\n\n");

    // Parameters
    if !spec.inputs.is_empty() {
        content.push_str("**Parameters:**\n");
        for input in &spec.inputs {
            let optional = if input.optional { " _(optional)_" } else { "" };
            content.push_str(&format!("- `{}`: {}{}\n", input.name, input.documentation, optional));
        }
        content.push_str("\n");
    }

    // Return type
    content.push_str(&format!("**Returns:** {}\n", spec.output.documentation));

    // Example
    if !spec.example.is_empty() {
        content.push_str("\n**Example:**\n```hcl\n");
        content.push_str(&spec.example);
        content.push_str("\n```");
    }

    content
}

/// Get all available addons
fn get_available_addons() -> Vec<Box<dyn Addon>> {
    use txtx_addon_telegram::TelegramAddon;
    use txtx_core::std::StdAddon;

    let addons: Vec<Box<dyn Addon>> = vec![
        Box::new(StdAddon::new()),
        Box::new(txtx_addon_network_bitcoin::BitcoinNetworkAddon::new()),
        Box::new(txtx_addon_network_evm::EvmNetworkAddon::new()),
        Box::new(txtx_addon_network_svm::SvmNetworkAddon::new()),
        Box::new(TelegramAddon::new()),
    ];

    // Add optional addons if available
    #[cfg(feature = "ovm")]
    addons.push(Box::new(txtx_addon_network_ovm::OvmNetworkAddon::new()));

    #[cfg(feature = "stacks")]
    addons.push(Box::new(txtx_addon_network_stacks::StacksNetworkAddon::new()));

    #[cfg(feature = "sp1")]
    addons.push(Box::new(txtx_addon_sp1::Sp1NetworkAddon::new()));

    addons
}

/// Build a map of all function names to their hover documentation
pub fn build_function_hover_map() -> HashMap<String, String> {
    let mut hover_map = HashMap::new();
    let addons = get_available_addons();

    for addon in addons {
        let namespace = addon.get_namespace();
        let functions = addon.get_functions();

        for func_spec in functions {
            let full_name = format!("{}::{}", namespace, func_spec.name);
            let hover_text = generate_function_hover_text(&func_spec);
            hover_map.insert(full_name, hover_text);
        }
    }

    hover_map
}

/// Get hover documentation for a function by its full name (e.g., "evm::get_contract_from_foundry_project")
pub fn get_function_hover(function_name: &str) -> Option<String> {
    lazy_static! {
        static ref FUNCTION_HOVER_MAP: HashMap<String, String> = build_function_hover_map();
    }

    FUNCTION_HOVER_MAP.get(function_name).cloned()
}

/// Get hover documentation for an action by its full name
pub fn get_action_hover(action_name: &str) -> Option<String> {
    // Similar to functions, we can generate action documentation
    use txtx_addon_kit::types::commands::PreCommandSpecification;

    lazy_static! {
        static ref ACTION_HOVER_MAP: HashMap<String, String> = {
            let mut hover_map = HashMap::new();
            let addons = get_available_addons();

            for addon in addons {
                let namespace = addon.get_namespace();
                let actions = addon.get_actions();

                for action in actions {
                    if let PreCommandSpecification::Atomic(spec) = action {
                        let full_name = format!("{}::{}", namespace, spec.matcher);
                        let hover_text = generate_action_hover_text(&spec);
                        hover_map.insert(full_name, hover_text);
                    }
                }
            }

            hover_map
        };
    }

    ACTION_HOVER_MAP.get(action_name).cloned()
}

/// Generate hover documentation for an action specification
fn generate_action_hover_text(
    spec: &txtx_addon_kit::types::commands::CommandSpecification,
) -> String {
    let mut content = String::new();

    // Action name
    content.push_str(&format!("### Action: `{}`\n\n", spec.matcher));

    // Documentation
    content.push_str(&spec.documentation);
    content.push_str("\n\n");

    // Inputs
    if !spec.inputs.is_empty() {
        content.push_str("**Inputs:**\n");
        for input in &spec.inputs {
            let optional = if input.optional { " _(optional)_" } else { "" };
            content.push_str(&format!("- `{}`: {}{}\n", input.name, input.documentation, optional));
        }
        content.push_str("\n");
    }

    // Outputs
    if !spec.outputs.is_empty() {
        content.push_str("**Outputs:**\n");
        for output in &spec.outputs {
            content.push_str(&format!("- `{}`: {}\n", output.name, output.documentation));
        }
        content.push_str("\n");
    }

    // Example
    if !spec.example.is_empty() {
        content.push_str("**Example:**\n```hcl\n");
        content.push_str(&spec.example);
        content.push_str("\n```");
    }

    content
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_hover_generation() {
        let hover_map = build_function_hover_map();

        // Print all available functions for debugging
        println!("Available functions:");
        for key in hover_map.keys() {
            println!("  - {}", key);
        }

        // Check that we have functions from key addons
        assert!(hover_map.contains_key("evm::get_contract_from_foundry_project"));

        // Check for std functions like encode_hex and decode_hex
        assert!(hover_map.contains_key("encode_hex") || hover_map.contains_key("std::encode_hex"));

        // Check that the hover text is properly formatted
        if let Some(evm_hover) = hover_map.get("evm::get_contract_from_foundry_project") {
            assert!(evm_hover.contains("### `get_contract_from_foundry_project`"));
            assert!(evm_hover.contains("**Parameters:**"));
            assert!(evm_hover.contains("**Returns:**"));
        }

        println!("Total functions with hover documentation: {}", hover_map.len());
    }

    #[test]
    fn test_action_hover_generation() {
        // Test action hover generation for deploy_contract
        let deploy_hover = get_action_hover("evm::deploy_contract");
        assert!(deploy_hover.is_some(), "Should have hover for evm::deploy_contract");

        if let Some(hover_text) = deploy_hover {
            assert!(hover_text.contains("### Action: `deploy_contract`"));
            assert!(hover_text.contains("**Inputs:**"));
            assert!(hover_text.contains("**Outputs:**"));
        }

        // Test action hover generation for call_contract
        let call_hover = get_action_hover("evm::call_contract");
        assert!(call_hover.is_some(), "Should have hover for evm::call_contract");

        if let Some(hover_text) = call_hover {
            println!("Hover for evm::call_contract:");
            println!("{}", hover_text);
            assert!(hover_text.contains("call_contract"));
            assert!(hover_text.contains("**Inputs:**"));
        }
    }

    #[test]
    fn test_signer_hover_generation() {
        // Test building signer hover map to see what's available
        lazy_static! {
            static ref SIGNER_HOVER_MAP: HashMap<String, String> = {
                let mut hover_map = HashMap::new();
                let addons = get_available_addons();

                for addon in addons {
                    let namespace = addon.get_namespace();
                    let signers = addon.get_signers();

                    for signer_spec in signers {
                        let full_name = format!("{}::{}", namespace, signer_spec.matcher);
                        println!("Signer found: {} (matcher: {})", signer_spec.name, full_name);
                        let hover_text = generate_signer_hover_text(&signer_spec);
                        hover_map.insert(full_name, hover_text);
                    }
                }

                hover_map
            };
        }

        println!("Available signers:");
        for key in SIGNER_HOVER_MAP.keys() {
            println!("  - {}", key);
        }

        // Test evm::web_wallet specifically
        let web_wallet_hover = get_signer_hover("evm::web_wallet");
        assert!(web_wallet_hover.is_some(), "Should have hover for evm::web_wallet");

        if let Some(hover_text) = web_wallet_hover {
            println!("Hover for evm::web_wallet:");
            println!("{}", hover_text);
            assert!(hover_text.contains("Signer: `EVM Web Wallet`"));
            assert!(hover_text.contains("wagmi"));
            assert!(hover_text.contains("Parameters"));
        }
    }

    #[test]
    fn test_specific_function_hover_content() {
        // Test that specific functions have proper hover documentation
        let evm_contract_hover = get_function_hover("evm::get_contract_from_foundry_project");
        assert!(
            evm_contract_hover.is_some(),
            "Should have hover for evm::get_contract_from_foundry_project"
        );

        if let Some(hover) = evm_contract_hover {
            println!("Hover content for evm::get_contract_from_foundry_project:");
            println!("{}", hover);
            assert!(hover.contains("get_contract_from_foundry_project"));
            assert!(hover.contains("Parameters"));
            assert!(hover.contains("Returns"));
        }

        // Test std function
        let encode_hex_hover = get_function_hover("std::encode_hex");
        assert!(encode_hex_hover.is_some(), "Should have hover for std::encode_hex");

        if let Some(hover) = encode_hex_hover {
            println!("\nHover content for std::encode_hex:");
            println!("{}", hover);
        }
    }
}

/// Generate hover documentation for a signer specification
fn generate_signer_hover_text(spec: &SignerSpecification) -> String {
    let mut content = String::new();

    // Signer name
    content.push_str(&format!("### Signer: `{}`\n\n", spec.name));

    // Documentation
    content.push_str(&spec.documentation);
    content.push_str("\n\n");

    // Inputs
    if !spec.inputs.is_empty() {
        content.push_str("**Parameters:**\n");
        for input in &spec.inputs {
            let optional = if input.optional { " _(optional)_" } else { "" };
            content.push_str(&format!("- `{}`: {}{}\n", input.name, input.documentation, optional));
        }
        content.push_str("\n");
    }

    // Example
    if !spec.example.is_empty() {
        content.push_str("**Example:**\n```hcl\n");
        content.push_str(&spec.example);
        content.push_str("\n```");
    }

    content
}

/// Get hover documentation for a signer by its full name
pub fn get_signer_hover(signer_name: &str) -> Option<String> {
    lazy_static! {
        static ref SIGNER_HOVER_MAP: HashMap<String, String> = {
            let mut hover_map = HashMap::new();
            let addons = get_available_addons();

            for addon in addons {
                let namespace = addon.get_namespace();
                let signers = addon.get_signers();

                for signer_spec in signers {
                    let full_name = format!("{}::{}", namespace, signer_spec.matcher);
                    let hover_text = generate_signer_hover_text(&signer_spec);
                    hover_map.insert(full_name, hover_text);
                }
            }

            hover_map
        };
    }

    SIGNER_HOVER_MAP.get(signer_name).cloned()
}
