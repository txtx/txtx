//! Tests for the ProjectTestHarness itself
//! 
//! These tests verify that the test harness correctly sets up project structures
//! and handles different compilation frameworks.

#[cfg(test)]
mod harness_tests {
    use super::super::{ProjectTestHarness, CompilationFramework};

    #[test]
    fn test_foundry_project_setup() {
        let runbook = r#"
addon "evm" {
    chain_id = 31337
    rpc_api_url = "http://127.0.0.1:8545"
}

action "deploy" "evm::deploy_contract" {
    from = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
    contract = "SimpleStorage"
}
"#;

        let harness = ProjectTestHarness::new_foundry("test.tx", runbook.to_string());
        harness.setup().expect("Failed to setup Foundry project");

        // Verify structure
        assert!(harness.project_path.join("txtx.yml").exists());
        assert!(harness.project_path.join("runbooks/test.tx").exists());
        assert!(harness.project_path.join("out").exists());
        assert!(harness.project_path.join("foundry.toml").exists());
    }

    #[test]
    fn test_hardhat_project_setup() {
        let runbook = r#"
addon "evm" {
    chain_id = 31337
    rpc_api_url = "http://127.0.0.1:8545"
}

action "deploy" "evm::deploy_contract" {
    from = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
    contract = "SimpleStorage"
}
"#;

        let harness = ProjectTestHarness::new_hardhat("test.tx", runbook.to_string());
        harness.setup().expect("Failed to setup Hardhat project");

        // Verify structure
        assert!(harness.project_path.join("txtx.yml").exists());
        assert!(harness.project_path.join("runbooks/test.tx").exists());
        assert!(harness.project_path.join("artifacts").exists());
        assert!(harness.project_path.join("hardhat.config.js").exists());
    }
}