use crate::codec::idl::IdlRef;
use convert_case::{Case, Casing};

pub fn get_interpolated_header_template(title: &str) -> String {
    return format!(
        r#"################################################################
# {}
################################################################
"#,
        title
    );
}

pub fn get_interpolated_addon_template(rpc_url: &str, network_id: &str) -> String {
    return format!(
        r#"
addon "svm" {{
    rpc_api_url = {}
    network_id = {}
}}
"#,
        rpc_url, network_id
    );
}

pub fn get_interpolated_localnet_signer_template(keypair_path: &str) -> String {
    return format!(
        r#"
signer "payer" "svm::secret_key" {{
    description = "Pays for the cost of deploying the program (e.g. rent for the program account)."
    keypair_json = {}
}}
    
signer "authority" "svm::secret_key" {{
    description = "Has permission to upgrade the program in the future (if the program is upgradable)."
    keypair_json = {}
}}
"#,
        keypair_path, keypair_path
    );
}

pub fn get_interpolated_devnet_signer_template() -> String {
    return format!(
        r#"
signer "payer" "svm::web_wallet" {{
    description = "Pays for the cost of deploying the program (e.g. rent for the program account)."
    // expected_address = "PUBLIC_KEY"
}}
    
signer "authority" "svm::web_wallet" {{
    description = "Has permission to upgrade the program in the future (if the program is upgradable)."
    // expected_address = "PUBLIC_KEY"
}}
"#,
    );
}

pub fn get_interpolated_mainnet_signer_template(_keypair_path: &str) -> String {
    return format!(
        r#"
// For mainnet deployment, use web wallets, hardware wallets, or multisig for key security.

// signer "payer" "svm::web_wallet" {{
//   description = "Pays for the cost of deploying the program (e.g. rent for the program account)."
//   address = "PUBLIC_KEY"
// }}

// signer "authority" "svm::squads" {{
//    description = "Has permission to upgrade the program in the future (if the program is upgradable)."
//    address = "SQUAD_PUBLIC_KEY"
//    initiator = "INITIATOR_PUBLIC_KEY"
// }}
"#
    );
}

pub fn get_interpolated_anchor_program_deployment_template(program_name: &str) -> String {
    return format!(
        r#"
action "deploy_{}" "svm::deploy_program" {{
    description = "Deploy {} program"
    program = svm::get_program_from_anchor_project("{}") 
    authority = signer.authority
    payer = signer.payer
}}
"#,
        program_name, program_name, program_name
    );
}

pub fn get_interpolated_native_program_deployment_template(program_name: &str) -> String {
    return format!(
        r#"
action "deploy_{}" "svm::deploy_program" {{
    description = "Deploy {} program"
    program = svm::get_program_from_native_project("{}") 
    authority = signer.authority
    payer = signer.payer
}}
"#,
        program_name, program_name, program_name
    );
}

pub fn get_interpolated_anchor_subgraph_template(
    program_name: &str,
    idl_str: &str,
) -> Result<Option<String>, String> {
    let idl =
        IdlRef::from_str(idl_str).map_err(|e| format!("failed to parse program idl: {e}"))?.idl;

    let subgraph_runbook = if idl.events.is_empty() {
        None
    } else {
        Some(
            idl.events
                .iter()
                .map(|event| {
                    let event_slug = Casing::to_case(&event.name, Case::Snake);
                    format!(
                        r#"
action "{program_name}_{event_slug}" "svm::deploy_subgraph" {{
    program_id = action.deploy_{program_name}.program_id
    program_idl = action.deploy_{program_name}.program_idl
    block_height = 0 // action.deploy_{program_name}.block_height
    event {{
        name = "{}"
    }}
}}"#,
                        event.name,
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"),
        )
    };
    Ok(subgraph_runbook)
}
