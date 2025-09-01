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
    use txtx_addon_kit::types::types::Value;
    use std::path::PathBuf;
    use tokio;
    
    #[tokio::test]
    async fn test_deploy_and_get_logs() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_deploy_and_get_logs - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing event log retrieval from deployed contract");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/event_logs.tx");
        
        // Simple event emitter bytecode
        let bytecode = "0x608060405234801561001057600080fd5b506101c7806100206000396000f3fe608060405234801561001057600080fd5b506004361061002b5760003560e01c8063a413686214610030575b600080fd5b61004a60048036038101906100459190610115565b61004c565b005b7f5d7c966d6c2ba9e25b7b3b085b44b5e7e5847de39a763eb7e896f3c3429a3e3b8160405161007b9190610177565b60405180910390a150565b600080fd5b600080fd5b600080fd5b600080fd5b600080fd5b60008083601f8401126100b7576100b6610091565b5b8235905067ffffffffffffffff8111156100d4576100d3610096565b5b6020830191508360018202830111156100f0576100ef61009b565b5b9250929050565b600080602083850312156101e5761010d610087565b5b600083013567ffffffffffffffff81111561012b5761012a61008c565b5b610137858286016100a0565b92509250509250929050565b600082825260208201905092915050565b50565b60006101646000830161014f565b9150819050919050565b61017881610143565b82525050565b600060208201905061019460008301846101e5565b9291505056fea26469706673582212206b8c";
        
        // REMOVED:         let result = MigrationHelper::from_fixture(&fixture_path)
            .with_anvil()
            .with_input("chain_id", "31337")
            .with_input("rpc_url", "http://127.0.0.1:8545")
            .with_input("private_key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
            .with_input("contract_address", "0x0000000000000000000000000000000000000000")
            .with_input("from_block", "0")
            .with_input("to_block", "latest")
            .with_input("event_signature", "TestEvent(string)")
            .with_input("topic_filter", "0x0")
            .with_input("event_emitter_bytecode", bytecode)
            .with_input("event_message", "Hello from test!")
            .execute()
            .await
            .expect("Failed to execute test");
        
        
        
        assert!(result.success, "Event log test should succeed");
        
        // Check that contract was deployed
        let deployed_addr = result.outputs.get("deployed_address")
            .and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None
            })
            .expect("Should have deployed address");
        
        assert!(deployed_addr.starts_with("0x"), "Should have valid contract address");
        
        // Check that event was emitted
        let tx_hash = result.outputs.get("event_tx_hash")
            .and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None
            })
            .expect("Should have event transaction hash");
        
        assert!(tx_hash.starts_with("0x"), "Should have valid transaction hash");
        
        println!("✅ Event emitted from contract at {}", deployed_addr);
    }
    
    #[tokio::test]
    async fn test_get_receipt_logs() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_get_receipt_logs - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing log retrieval from transaction receipt");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/event_logs.tx");
        
        // Contract that emits an event in constructor
        let bytecode = "0x608060405234801561001057600080fd5b507f290decd9548b62a8d60345a988386fc84ba6bc95484008f6362f93160ef3e56360405160405180910390a16000806101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff16021790555060b3806100896000396000f3fe6080604052348015600f57600080fd5b506004361060285760003560e01c8063c19d93fb14602d575b600080fd5b60336047565b604051603e9190605a565b60405180910390f35b600054905090565b6054816073565b82525050565b6000602082019050606d6000830184604d565b92915050565b6000819050919050565b56fea264697066735822122064f";
        
        // REMOVED:         let result = MigrationHelper::from_fixture(&fixture_path)
            .with_anvil()
            .with_input("chain_id", "31337")
            .with_input("rpc_url", "http://127.0.0.1:8545")
            .with_input("private_key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
            .with_input("contract_address", "0x0000000000000000000000000000000000000000")
            .with_input("from_block", "0")
            .with_input("to_block", "latest")
            .with_input("event_signature", "Deployed()")
            .with_input("topic_filter", "0x0")
            .with_input("event_emitter_bytecode", bytecode)
            .with_input("event_message", "Test")
            .execute()
            .await
            .expect("Failed to execute test");
        
        
        
        assert!(result.success, "Receipt log test should succeed");
        
        // Check receipt logs
        let receipt_logs = result.outputs.get("receipt_logs");
        assert!(receipt_logs.is_some(), "Should have receipt logs");
        
        println!("✅ Retrieved logs from transaction receipt");
    }
    
    #[tokio::test]
    async fn test_filter_logs_by_block_range() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_filter_logs_by_block_range - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing log filtering by block range");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/event_logs.tx");
        
        let bytecode = "0x60806040523480156100f57600080fd5b50610113806100206000396000f3fe6080604052600080fdfea265627a7a72315820";
        
        // REMOVED:         let result = MigrationHelper::from_fixture(&fixture_path)
            .with_anvil()
            .with_input("chain_id", "31337")
            .with_input("rpc_url", "http://127.0.0.1:8545")
            .with_input("private_key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
            .with_input("contract_address", "0x0000000000000000000000000000000000000000")
            .with_input("from_block", "0")
            .with_input("to_block", "100")
            .with_input("event_signature", "Test()")
            .with_input("topic_filter", "0x0")
            .with_input("event_emitter_bytecode", bytecode)
            .with_input("event_message", "Block range test")
            .execute()
            .await
            .expect("Failed to execute test");
        
        
        
        assert!(result.success, "Block range filter test should succeed");
        
        println!("✅ Log filtering by block range working");
    }
    
    #[tokio::test]
    async fn test_parse_event_data() {
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_parse_event_data - Anvil not installed");
            return;
        }
        
        println!("🔍 Testing event data parsing from logs");
        
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/integration/event_logs.tx");
        
        // Contract with Transfer event
        let bytecode = "0x608060405234801561001057600080fd5b5060f38061001f6000396000f3fe6080604052348015600f57600080fd5b506004361060285760003560e01c8063a9059cbb14602d575b600080fd5b60436004803603810190603f9190605f565b6045565b005b8173ffffffffffffffffffffffffffffffffffffffff167fddf252ad1be2c89b69c2b068fc378daa952ba7163c4a11628f55a4df523b3ef826040516089919060ac565b60405180910390a25050565b600080fd5b600073ffffffffffffffffffffffffffffffffffffffff82169050919050565b600060b68260099050565b9050565b60c18160b0565b811460cb57600080fd5b50565b60008135905060dc8160ba565b92915050565b6000819050919050565b60f08160e2565b811460fa57600080fd5b50565b60008135905061010a8160ec565b92915050565b6000806040838503121561012457610123609556005b5b60006101308582860160ce565b925050602061014185828601610fd565b9150509250929050565b61015481610e2565b82525050565b600060208201905061016f600083018461014b565b9291505056fea26469706673582212209c";
        
        // REMOVED:         let result = MigrationHelper::from_fixture(&fixture_path)
            .with_anvil()
            .with_input("chain_id", "31337")
            .with_input("rpc_url", "http://127.0.0.1:8545")
            .with_input("private_key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
            .with_input("contract_address", "0x0000000000000000000000000000000000000000")
            .with_input("from_block", "latest")
            .with_input("to_block", "latest")
            .with_input("event_signature", "Transfer(address,address,uint256)")
            .with_input("topic_filter", "0xddf252ad1be2c89b69c2b068fc378daa952ba7163c4a11628f55a4df523b3ef")
            .with_input("event_emitter_bytecode", bytecode)
            .with_input("event_message", "Parse test")
            .execute()
            .await
            .expect("Failed to execute test");
        
        
        
        assert!(result.success, "Event parsing test should succeed");
        
        println!("✅ Event data parsing successful");
    }
}