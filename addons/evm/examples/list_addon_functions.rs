// Test to see what functions are available in EVM addon
use txtx_addon_kit::Addon;
use txtx_addon_network_evm::EvmNetworkAddon;

fn main() {
    let addon = EvmNetworkAddon::new();
    let functions = addon.get_functions();
    
    println!("EVM addon has {} functions:", functions.len());
    for func in &functions {
        println!("  - {}: {}", func.name, func.documentation);
        if func.name.contains("contract") || func.name.contains("foundry") {
            println!("    Found relevant function!");
        }
    }
    
    // Look specifically for get_contract_from_foundry_project
    let target = "get_contract_from_foundry_project";
    if functions.iter().any(|f| f.name == target) {
        println!("\n✓ Found {}!", target);
    } else {
        println!("\n✗ {} not found", target);
        println!("Similar functions:");
        for func in &functions {
            if func.name.contains("contract") || func.name.contains("get") {
                println!("  - {}", func.name);
            }
        }
    }
}