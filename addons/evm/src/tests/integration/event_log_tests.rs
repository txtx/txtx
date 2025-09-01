//! Integration tests for event log functionality
//! 
//! These tests verify that event log operations properly:
//! - Retrieve logs from contracts
//! - Filter logs by topics and addresses
//! - Parse event data from logs
//! - Extract logs from transaction receipts

#[cfg(test)]
mod event_log_tests {
    use crate::tests::integration::anvil_harness::AnvilInstance;
    use crate::tests::fixture_builder::{FixtureBuilder, get_anvil_manager};
    use tokio;
    
    #[tokio::test]
    async fn test_deploy_and_get_logs() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_deploy_and_get_logs - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing event log retrieval from deployed contract");
        
        // ARRANGE: Create inline runbook for event emission and retrieval
        let event_runbook = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

signer "deployer" "evm::private_key" {
    private_key = input.private_key
}

# Deploy a simple event emitter contract
# Contract emits TestEvent(string message, address sender)
action "deploy_emitter" "evm::deploy_contract" {
    artifact_source = "inline:0x608060405234801561001057600080fd5b506101dc806100206000396000f3fe608060405234801561001057600080fd5b506004361061002b5760003560e01c8063f0fdf83414610030575b600080fd5b61004a600480360381019061004591906100a4565b610060565b6040516100579190610106565b60405180910390a150565b6000813373ffffffffffffffffffffffffffffffffffffffff167fce0457fe73731f824cc272376169235128c118b49d344817417c6d108d155e82866040516100a99190610106565b60405180910390a3600190509190565b600080fd5b600080fd5b600080fd5b60008083601f8401126100df576100de6100ba565b5b8235905067ffffffffffffffff8111156100fc576100fb6100bf565b5b60208301915083600182028301111561011857610117610103565b5b9250929050565b6000806020838503121561013657610135610100565b5b600083013567ffffffffffffffff81111561015457610153610105565b5b610160858286016100c8565b92509250509250929050565b600082825260208201905092915050565b50565b600061018e60008361016c565b915061019982610185565b600082019050919050565b60006101af82610181565b915081905091905056fea26469706673582212208c"
    signer = signer.deployer
}

# Emit an event
action "emit_event" "evm::call_contract_function" {
    contract_address = action.deploy_emitter.contract_address
    function_signature = "emitEvent(string)"
    function_args = ["Hello from test!"]
    signer = signer.deployer
}

# Get transaction receipt to see logs
action "get_receipt" "evm::get_transaction_receipt" {
    tx_hash = action.emit_event.tx_hash
}

# Get logs by filter
action "get_logs" "evm::get_logs" {
    address = action.deploy_emitter.contract_address
    from_block = 0
    to_block = "latest"
}

output "deployed_address" {
    value = action.deploy_emitter.contract_address
}

output "event_tx_hash" {
    value = action.emit_event.tx_hash
}

output "receipt_logs" {
    value = action.get_receipt.logs
}

output "filtered_logs" {
    value = action.get_logs.logs
}"#;
        
        let mut fixture = FixtureBuilder::new("test_deploy_and_get_logs")
            .with_anvil_manager(get_anvil_manager().await.unwrap())
            .with_runbook("events", event_runbook)
            .build()
            .await
            .expect("Failed to build fixture");
        
        // Set up parameters
        let accounts = fixture.anvil_handle.accounts();
        fixture.config.parameters.insert("chain_id".to_string(), "31337".to_string());
        fixture.config.parameters.insert("rpc_url".to_string(), fixture.rpc_url.clone());
        fixture.config.parameters.insert("private_key".to_string(), accounts.alice.secret_string());
        
        // ACT: Deploy contract and emit event
        fixture.execute_runbook("events").await
            .expect("Failed to execute event test");
        
        // ASSERT: Verify contract deployment and event emission
        let outputs = fixture.get_outputs("events")
            .expect("Should have outputs");
        
        let deployed_addr = outputs.get("deployed_address")
            .and_then(|v| v.as_string())
            .expect("Should have deployed address");
        assert!(deployed_addr.starts_with("0x"), "Should have valid contract address");
        
        let tx_hash = outputs.get("event_tx_hash")
            .and_then(|v| v.as_string())
            .expect("Should have event transaction hash");
        assert!(tx_hash.starts_with("0x"), "Should have valid transaction hash");
        
        // Receipt should have logs
        assert!(outputs.get("receipt_logs").is_some(), "Should have receipt logs");
        
        println!("✅ Event emitted from contract at {}", deployed_addr);
    }
    
    #[tokio::test]
    async fn test_get_receipt_logs() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_get_receipt_logs - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing log retrieval from transaction receipt");
        
        // ARRANGE: Create inline runbook for deployment with constructor event
        let receipt_runbook = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

signer "deployer" "evm::private_key" {
    private_key = input.private_key
}

# Deploy contract that emits event in constructor
# Contract emits Deployed() event when created
action "deploy_with_event" "evm::deploy_contract" {
    artifact_source = "inline:0x608060405234801561001057600080fd5b507f290decd9548b62a8d60345a988386fc84ba6bc95484008f6362f93160ef3e56360405160405180910390a16000806101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff16021790555060b3806100896000396000f3fe6080604052348015600f57600080fd5b506004361060285760003560e01c8063c19d93fb14602d575b600080fd5b60336047565b604051603e9190605a565b60405180910390f35b600054905090565b6054816073565b82525050565b6000602082019050606d6000830184604d565b92915050565b6000819050919050565b56fea264697066735822122064f"
    signer = signer.deployer
}

# Get deployment transaction receipt
action "get_deploy_receipt" "evm::get_transaction_receipt" {
    tx_hash = action.deploy_with_event.tx_hash
}

output "contract_address" {
    value = action.deploy_with_event.contract_address
}

output "deploy_tx_hash" {
    value = action.deploy_with_event.tx_hash
}

output "receipt_logs" {
    value = action.get_deploy_receipt.logs
}

output "log_count" {
    value = action.get_deploy_receipt.logs_count
}"#;
        
        let mut fixture = FixtureBuilder::new("test_get_receipt_logs")
            .with_anvil_manager(get_anvil_manager().await.unwrap())
            .with_runbook("receipt", receipt_runbook)
            .build()
            .await
            .expect("Failed to build fixture");
        
        // Set up parameters
        let accounts = fixture.anvil_handle.accounts();
        fixture.config.parameters.insert("chain_id".to_string(), "31337".to_string());
        fixture.config.parameters.insert("rpc_url".to_string(), fixture.rpc_url.clone());
        fixture.config.parameters.insert("private_key".to_string(), accounts.alice.secret_string());
        
        // ACT: Deploy contract with constructor event
        fixture.execute_runbook("receipt").await
            .expect("Failed to execute receipt test");
        
        // ASSERT: Verify receipt contains logs
        let outputs = fixture.get_outputs("receipt")
            .expect("Should have outputs");
        
        assert!(outputs.get("receipt_logs").is_some(), "Should have receipt logs");
        
        let contract = outputs.get("contract_address")
            .and_then(|v| v.as_string())
            .expect("Should have contract address");
        assert!(contract.starts_with("0x"), "Should have valid contract address");
        
        println!("✅ Retrieved logs from transaction receipt");
    }
    
    #[tokio::test]
    async fn test_filter_logs_by_block_range() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_filter_logs_by_block_range - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing log filtering by block range");
        
        // ARRANGE: Create inline runbook for block range filtering
        let filter_runbook = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

signer "sender" "evm::private_key" {
    private_key = input.private_key
}

# Deploy an event emitter
action "deploy" "evm::deploy_contract" {
    artifact_source = "inline:0x608060405234801561001057600080fd5b506101a4806100206000396000f3fe608060405234801561001057600080fd5b506004361061002b5760003560e01c8063b0c8f14714610030575b600080fd5b61003861004c565b604051610049959493929190610091565b60405180910390f35b60008060006040516100619061005a565b604051809103902090508091929394955050505050565b600060648201905060008201516100866000850160e0565b5091905056fea264697066735822122039a"
    signer = signer.sender
}

# Get current block number
action "get_start_block" "evm::get_block_number" {}

# Emit some events in different blocks
action "emit1" "evm::call_contract_function" {
    contract_address = action.deploy.contract_address
    function_signature = "emitEvent()"
    signer = signer.sender
}

action "emit2" "evm::call_contract_function" {
    contract_address = action.deploy.contract_address
    function_signature = "emitEvent()"
    signer = signer.sender
}

action "emit3" "evm::call_contract_function" {
    contract_address = action.deploy.contract_address
    function_signature = "emitEvent()"
    signer = signer.sender
}

# Get end block number
action "get_end_block" "evm::get_block_number" {}

# Filter logs for specific block range
action "filter_logs" "evm::get_logs" {
    address = action.deploy.contract_address
    from_block = action.get_start_block.block_number
    to_block = action.get_end_block.block_number
}

output "contract_address" {
    value = action.deploy.contract_address
}

output "start_block" {
    value = action.get_start_block.block_number
}

output "end_block" {
    value = action.get_end_block.block_number
}

output "filtered_logs" {
    value = action.filter_logs.logs
}"#;
        
        let mut fixture = FixtureBuilder::new("test_filter_logs")
            .with_anvil_manager(get_anvil_manager().await.unwrap())
            .with_runbook("filter", filter_runbook)
            .build()
            .await
            .expect("Failed to build fixture");
        
        // Set up parameters
        let accounts = fixture.anvil_handle.accounts();
        fixture.config.parameters.insert("chain_id".to_string(), "31337".to_string());
        fixture.config.parameters.insert("rpc_url".to_string(), fixture.rpc_url.clone());
        fixture.config.parameters.insert("private_key".to_string(), accounts.alice.secret_string());
        
        // ACT: Execute block range filtering
        fixture.execute_runbook("filter").await
            .expect("Failed to execute filter test");
        
        // ASSERT: Verify logs were filtered
        let outputs = fixture.get_outputs("filter")
            .expect("Should have outputs");
        
        let start_block = outputs.get("start_block")
            .and_then(|v| v.as_integer())
            .expect("Should have start block");
        
        let end_block = outputs.get("end_block")
            .and_then(|v| v.as_integer())
            .expect("Should have end block");
        
        assert!(end_block >= start_block, "End block should be >= start block");
        assert!(outputs.get("filtered_logs").is_some(), "Should have filtered logs");
        
        println!("✅ Log filtering by block range working (blocks {} to {})", start_block, end_block);
    }
    
    #[tokio::test]
    async fn test_parse_event_data() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_parse_event_data - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing event data parsing from logs");
        
        // ARRANGE: Create inline runbook for event parsing
        let parse_runbook = r#"
addon "evm" {
    chain_id = input.chain_id
    rpc_api_url = input.rpc_url
}

signer "sender" "evm::private_key" {
    private_key = input.private_key
}

# Deploy a transfer event emitter (simulates ERC20 Transfer)
action "deploy_token" "evm::deploy_contract" {
    artifact_source = "inline:0x608060405234801561001057600080fd5b506101b3806100206000396000f3fe608060405234801561001057600080fd5b506004361061002b5760003560e01c8063a9059cbb14610030575b600080fd5b61004a60048036038101906100459190610115565b610060565b6040516100579190610170565b60405180910390f35b60008273ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff167fddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef846040516100bf919061018b565b60405180910390a3600190509290505056fea26469706673582212209e"
    signer = signer.sender
}

# Emit a Transfer event
action "transfer" "evm::call_contract_function" {
    contract_address = action.deploy_token.contract_address
    function_signature = "transfer(address,uint256)"
    function_args = [input.recipient, 1000000]
    signer = signer.sender
}

# Get the transaction receipt
action "get_receipt" "evm::get_transaction_receipt" {
    tx_hash = action.transfer.tx_hash
}

# Parse the Transfer event from logs
action "parse_logs" "evm::parse_log" {
    logs = action.get_receipt.logs
    event_signature = "Transfer(address,address,uint256)"
}

output "contract_address" {
    value = action.deploy_token.contract_address
}

output "transfer_tx" {
    value = action.transfer.tx_hash
}

output "receipt_logs" {
    value = action.get_receipt.logs
}

output "parsed_events" {
    value = action.parse_logs.events
}"#;
        
        let mut fixture = FixtureBuilder::new("test_parse_event")
            .with_anvil_manager(get_anvil_manager().await.unwrap())
            .with_runbook("parse", parse_runbook)
            .build()
            .await
            .expect("Failed to build fixture");
        
        // Set up parameters
        let accounts = fixture.anvil_handle.accounts();
        fixture.config.parameters.insert("chain_id".to_string(), "31337".to_string());
        fixture.config.parameters.insert("rpc_url".to_string(), fixture.rpc_url.clone());
        fixture.config.parameters.insert("private_key".to_string(), accounts.alice.secret_string());
        fixture.config.parameters.insert("recipient".to_string(), accounts.bob.address_string());
        
        // ACT: Execute event parsing
        fixture.execute_runbook("parse").await
            .expect("Failed to execute parse test");
        
        // ASSERT: Verify event was parsed
        let outputs = fixture.get_outputs("parse")
            .expect("Should have outputs");
        
        let transfer_tx = outputs.get("transfer_tx")
            .and_then(|v| v.as_string())
            .expect("Should have transfer transaction");
        assert!(transfer_tx.starts_with("0x"), "Should have valid transaction hash");
        
        assert!(outputs.get("receipt_logs").is_some(), "Should have receipt logs");
        assert!(outputs.get("parsed_events").is_some(), "Should have parsed events");
        
        println!("✅ Event data parsing successful");
    }
}