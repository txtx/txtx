use crate::codec::idl::IdlRef;
use convert_case::{Case, Casing};
use serde::{Deserialize, Serialize};

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
    description = "Pays fees for program deployments and operations"
    keypair_json = {}
    // See documentation for other options (mnemonic, etc): https://docs.surfpool.run/iac/svm/signers
}}
    
signer "authority" "svm::secret_key" {{
    description = "Can upgrade programs and manage critical ops"
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
    description = "Pays fees for program deployments and operations"
    // Optional: the public key of the signer can be enforced at runtime by setting an expected value
    // expected_address = "zbBjhHwuqyKMmz8ber5oUtJJ3ZV4B6ePmANfGyKzVGV"
}}

signer "authority" "svm::web_wallet" {{
    description = "Can upgrade programs and manage critical ops"
    // expected_address = input.expected_payer_address
    // See documentation for other options (squads, etc): https://docs.surfpool.run/iac/svm/signers
}}
"#,
    );
}

pub fn get_interpolated_mainnet_signer_template(_keypair_path: &str) -> String {
    return format!(
        r#"
// For mainnet deployment, use web wallets, hardware wallets or multisig to improve key security.

signer "payer" "svm::web_wallet" {{
    description = "Pays fees for program deployments and operations"
    // Optional: the public key of the signer can be enforced at runtime by setting an expected value
    // expected_address = "zbBjhHwuqyKMmz8ber5oUtJJ3ZV4B6ePmANfGyKzVGV"
}}

signer "authority" "svm::web_wallet" {{
    description = "Can upgrade programs and manage critical ops"
    // expected_address = input.expected_payer_address
    // See documentation for other options (squads, etc): https://docs.surfpool.run/iac/svm/signers
}}
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
    // Optional: if you want to deploy the program via a cheatcode when targeting a Surfnet, set `instant_surfnet_deployment = true`
    // Deploying via a cheatcode will write the program data directly to the program account, rather than sending transactions.
    // This will make deployments instantaneous, but is deviating from how the deployments will take place on devnet/mainnet.
    // instant_surfnet_deployment = true
}}
"#,
        program_name, program_name, program_name
    );
}

pub fn get_in_memory_interpolated_anchor_program_deployment_template(program_name: &str) -> String {
    return format!(
        r#"
action "deploy_{}" "svm::deploy_program" {{
    description = "Deploy {} program"
    program = svm::get_program_from_anchor_project("{}") 
    authority = signer.authority
    payer = signer.payer
    instant_surfnet_deployment = true
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
    // Optional: if you want to deploy the program via a cheatcode when targeting a Surfnet, set `instant_surfnet_deployment = true`
    // Deploying via a cheatcode will write the program data directly to the program account, rather than sending transactions.
    // This will make deployments instantaneous, but is deviating from how the deployments will take place on devnet/mainnet.
    // instant_surfnet_deployment = true
}}
"#,
        program_name, program_name, program_name
    );
}

pub fn get_in_memory_interpolated_native_program_deployment_template(program_name: &str) -> String {
    return format!(
        r#"
action "deploy_{}" "svm::deploy_program" {{
    description = "Deploy {} program"
    program = svm::get_program_from_native_project("{}") 
    authority = signer.authority
    payer = signer.payer
    instant_surfnet_deployment = true
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisEntry {
    // Base58 pubkey string.
    pub address: String,
    // Filepath to the compiled program to embed into the genesis.
    pub program: String,
    // Whether the genesis program is upgradeable.
    pub upgradeable: Option<bool>,
}

impl GenesisEntry {
    pub fn get_deploy_template(&self) -> String {
        format!(
            r#"
    deploy_program {{
        program_id = "{}"
        binary_path = "{}"
        authority = svm::system_program_id()
    }}
"#,
            self.address, self.program
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountEntry {
    // Base58 pubkey string.
    pub address: String,
    // Name of JSON file containing the account data.
    pub filename: String,
}

impl AccountEntry {
    pub fn get_account_update_template(&self) -> String {
        format!(
            r#"
    set_account {{
        public_key = "{}"
        account_path = "{}"
    }}
"#,
            self.address, self.filename
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountDirEntry {
    // Directory containing account JSON files
    pub directory: String,
}
impl AccountDirEntry {
    pub fn get_account_update_templates(&self) -> Vec<String> {
        let dir = std::path::Path::new(&self.directory);
        let mut templates = vec![];
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    templates.push(format!(
                        r#"
    set_account {{
        account_path = "{}"
    }}
"#,
                        path.to_string_lossy()
                    ));
                }
            }
        }
        templates
    }
}

pub fn get_interpolated_setup_surfnet_template(
    genesis_accounts: &Vec<GenesisEntry>,
    accounts: &Vec<AccountEntry>,
    account_dirs: &Vec<AccountDirEntry>,
) -> Option<String> {
    if genesis_accounts.is_empty() && accounts.is_empty() && account_dirs.is_empty() {
        return None;
    }
    let deployments = genesis_accounts
        .iter()
        .map(|entry| entry.get_deploy_template())
        .collect::<Vec<_>>()
        .join("\n");

    let account_updates = accounts
        .iter()
        .map(|entry| entry.get_account_update_template())
        .collect::<Vec<_>>()
        .join("\n");

    let dir_account_updates = account_dirs
        .iter()
        .flat_map(|entry| entry.get_account_update_templates())
        .collect::<Vec<_>>()
        .join("\n");

    Some(format!(
        r#"
action "setup_surfnet" "svm::setup_surfnet" {{
    description = "Sets up a local Surfnet with genesis accounts"
{}
{}
{}
}}"#,
        deployments, account_updates, dir_account_updates
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_interpolated_setup_surfnet() {
        let result = get_interpolated_setup_surfnet_template(&vec![], &vec![], &vec![]);
        assert!(result.is_none(), "Expected None for empty genesis accounts");

        let genesis_accounts = vec![
            GenesisEntry {
                address: "Address1".to_string(),
                program: "Path/To/Program1.so".to_string(),
                upgradeable: Some(true),
            },
            GenesisEntry {
                address: "Address2".to_string(),
                program: "Path/To/Program2.so".to_string(),
                upgradeable: Some(false),
            },
        ];
        let accounts = vec![
            AccountEntry {
                address: "Account1".to_string(),
                filename: "Path/To/Account1.json".to_string(),
            },
            AccountEntry {
                address: "Account2".to_string(),
                filename: "Path/To/Account2.json".to_string(),
            },
        ];

        let result = get_interpolated_setup_surfnet_template(&genesis_accounts, &accounts, &vec![]);
        assert!(result.is_some(), "Expected Some for non-empty genesis accounts");
        let expected = r#"
action "setup_surfnet" "svm::setup_surfnet" {
    description = "Sets up a local Surfnet with genesis accounts"

    deploy_program {
        program_id = "Address1"
        binary_path = "Path/To/Program1.so"
        authority = svm::system_program_id()
    }


    deploy_program {
        program_id = "Address2"
        binary_path = "Path/To/Program2.so"
        authority = svm::system_program_id()
    }


    set_account {
        public_key = "Account1"
        account_path = "Path/To/Account1.json"
    }


    set_account {
        public_key = "Account2"
        account_path = "Path/To/Account2.json"
    }


}
"#;
        assert_eq!(result.unwrap().trim(), expected.trim());
    }
}
