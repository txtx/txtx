fn get_action_doc_url(action_type: &str) -> Option<String> {
    let parts: Vec<&str> = action_type.split("::").collect();
    if parts.len() != 2 {
        return None;
    }
    
    let addon = parts[0];
    let action = parts[1];
    let action_slug = action.replace("_", "-");
    
    match addon {
        "evm" => Some(format!("https://docs.txtx.sh/addons/evm/actions#{}", action_slug)),
        "stacks" => Some(format!("https://docs.txtx.sh/addons/stacks/actions#{}", action_slug)),
        "bitcoin" => Some(format!("https://docs.txtx.sh/addons/bitcoin/actions#{}", action_slug)),
        "svm" => Some(format!("https://docs.txtx.sh/addons/svm/actions#{}", action_slug)),
        _ => None,
    }
}

fn main() {
    println!("Testing documentation URL generation:\n");
    
    let test_cases = vec![
        "evm::send_eth",
        "evm::call_contract",
        "evm::deploy_contract",
        "evm::check_confirmations",
        "stacks::call_contract",
        "stacks::deploy_contract",
        "bitcoin::send_btc",
        "svm::call_program",
        "unknown::action",
    ];
    
    for action in test_cases {
        match get_action_doc_url(action) {
            Some(url) => println!("✅ {} → {}", action, url),
            None => println!("❌ {} → No documentation URL", action),
        }
    }
    
    println!("\nThe URLs follow the pattern:");
    println!("- Addon name matches the docs section");
    println!("- Action names convert from snake_case to kebab-case");
    println!("- Anchors link directly to the specific action");
}