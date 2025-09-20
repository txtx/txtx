use txtx_test_utils::builders::parser::{
    extract_signers, find_action_references, find_env_references, find_signer_references,
    parse_runbook_content,
};

#[test]
fn test_parse_runbook_blocks() {
    let content = r#"
addon "evm" "ethereum" {
    rpc_url = "https://example.com"
}

signer "deployer" "evm::web_wallet" {
    expected_address = "0x123..."
}

action "deploy" "evm::deploy_contract" {
    contract_name = "MyToken"
    signer = signer.deployer
}

output "contract_address" {
    value = action.deploy.contract_address
}
"#;

    let blocks = parse_runbook_content(content).unwrap();
    assert_eq!(blocks.len(), 4);

    assert_eq!(blocks[0].block_type, "addon");
    assert_eq!(blocks[0].labels, vec!["evm", "ethereum"]);

    assert_eq!(blocks[1].block_type, "signer");
    assert_eq!(blocks[1].labels, vec!["deployer", "evm::web_wallet"]);

    assert_eq!(blocks[2].block_type, "action");
    assert_eq!(blocks[2].labels, vec!["deploy", "evm::deploy_contract"]);

    assert_eq!(blocks[3].block_type, "output");
    assert_eq!(blocks[3].labels, vec!["contract_address"]);
}

#[test]
fn test_extract_signers() {
    let content = r#"
signer "alice" "evm::web_wallet" {}
signer "bob" "evm::ledger" {}
action "test" "evm::send_eth" {}
"#;

    let blocks = parse_runbook_content(content).unwrap();
    let signers = extract_signers(&blocks);

    assert_eq!(signers.len(), 2);
    assert!(signers.contains(&"alice".to_string()));
    assert!(signers.contains(&"bob".to_string()));
}

#[test]
fn test_find_signer_references() {
    let content = r#"
action "send" "evm::send_eth" {
    signer = signer.alice
    from = signers.bob
}
output "test" {
    value = signer.charlie
}
"#;

    let refs = find_signer_references(content);
    assert_eq!(refs.len(), 3);
    assert!(refs.contains(&"alice".to_string()));
    assert!(refs.contains(&"bob".to_string()));
    assert!(refs.contains(&"charlie".to_string()));
}

#[test]
fn test_find_action_references() {
    let content = r#"
output "tx_hash" {
    value = action.deploy.tx_hash
}
variable "contract" {
    value = action.deploy.contract_address
}
action "next" "evm::call" {
    contract = action.deploy.contract_address
}
"#;

    let refs = find_action_references(content);
    assert_eq!(refs.len(), 1);
    assert!(refs.contains(&"deploy".to_string()));
}

#[test]
fn test_undefined_signer_detection() {
    let content = r#"
signer "alice" "evm::web_wallet" {}

action "send" "evm::send_eth" {
    signer = signer.bob  // undefined!
}
"#;

    let blocks = parse_runbook_content(content).unwrap();
    let defined_signers = extract_signers(&blocks);
    let signer_refs = find_signer_references(content);

    assert_eq!(defined_signers, vec!["alice"]);
    assert!(signer_refs.contains(&"bob".to_string()));

    // Find undefined signers
    let undefined: Vec<_> = signer_refs.iter().filter(|r| !defined_signers.contains(r)).collect();

    assert_eq!(undefined.len(), 1);
    assert_eq!(undefined[0], "bob");
}

#[test]
fn test_find_env_references() {
    let content = r#"
variable "api_key" {
    value = env.API_KEY
}

action "call" "evm::call_contract" {
    endpoint = env.RPC_URL
    auth = env.AUTH_TOKEN
}

output "result" {
    value = concat(env.PREFIX, action.call.result)
}
"#;

    let refs = find_env_references(content);
    assert_eq!(refs.len(), 4);
    assert!(refs.contains(&"API_KEY".to_string()));
    assert!(refs.contains(&"RPC_URL".to_string()));
    assert!(refs.contains(&"AUTH_TOKEN".to_string()));
    assert!(refs.contains(&"PREFIX".to_string()));
}
