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

pub fn get_interpolated_signer_template(keypair_path: &str) -> String {
    return format!(
        r#"
signer "payer" "svm::secret_key" {{
    keypair_json = {}
}}
    
signer "authority" "svm::secret_key" {{
    keypair_json = {}
}}
"#,
        keypair_path, keypair_path
    );
}

pub fn get_interpolated_mainnet_signer_template(_keypair_path: &str) -> String {
    return format!(
        r#"
# For mainnet deployment, use web wallets, hardware wallets, or multisig for key security.

# signer "payer" "svm::web_wallet" {{
#   address = "YOUR_WEB_WALLET_PUBLIC_KEY"
# }}

# signer "authority" "svm::squads" {{
#   address = "YOUR_SQUAD_PUBLIC_KEY"
# }}
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
