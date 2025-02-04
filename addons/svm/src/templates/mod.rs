pub fn get_interpolated_header_template(title: &str) -> String {
    return format!(
        r#"################################################################
# {}
################################################################
"#,
        title
    );
}

pub fn get_interpolated_addon_template(rpc_url: &str) -> String {
    return format!(
        r#"
addon "svm" {{
    rpc_api_url = "{}"
}}
"#,
        rpc_url
    );
}

pub fn get_interpolated_signer_template(keypair_path: &str) -> String {
    return format!(
        r#"
signer "deployer" "svm::secret_key" {{
    keypair_json = "{}"
}}
"#,
        keypair_path
    );
}

pub fn get_interpolated_anchor_program_deployment_template(program_name: &str) -> String {
    return format!(
        r#"
action "deploy_{}" "svm::deploy_program" {{
    description = "Deploy {} program"
    program = svm::get_program_from_anchor_project("{}") 
    authority = signer.deployer
    payer = signer.deployer
}}
"#,
        program_name, program_name, program_name
    );
}
