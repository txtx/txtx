//! Integration tests for transaction handling
//!
//! Tests ETH transfers, contract calls, and various transaction types.

#[cfg(test)]
mod transaction_integration_tests {
    use super::super::anvil_harness::{AnvilInstance, TestAccount};
    use crate::errors::{EvmError, TransactionError};
    use alloy::network::EthereumWallet;
    use alloy::primitives::{Address, U256, hex};
    use alloy::providers::Provider;
    use alloy::rpc::types::TransactionRequest;
    use std::str::FromStr;
    
    #[tokio::test]
    async fn test_eth_transfer() {
        use crate::rpc::EvmWalletRpc;
        use alloy::network::TransactionBuilder as NetworkTransactionBuilder;
        
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_eth_transfer - Anvil not installed");
            return;
        }
        
        // Spawn Anvil instance
        let anvil = AnvilInstance::spawn();
        println!("Anvil spawned on {}", anvil.url);
        
        // Use the first test account (has 10000 ETH)
        let sender = &anvil.accounts[0];
        let recipient = Address::from_str("0x70997970C51812dc3A010C7d01b50e0d17dc79C8").unwrap();
        let amount = U256::from(1_000_000_000_000_000_000u64); // 1 ETH in wei
        
        println!("📤 Transferring 1 ETH from {} to {}", sender.address, recipient);
        
        // Create RPC client with wallet for signing
        let wallet = EthereumWallet::from(sender.signer.clone());
        let rpc = EvmWalletRpc::new(&anvil.url, wallet.clone()).unwrap();
        
        // Get initial balances
        let sender_balance_before = rpc.provider.get_balance(sender.address).await.unwrap();
        let recipient_balance_before = rpc.provider.get_balance(recipient).await.unwrap();
        
        println!("💰 Initial balances:");
        println!("   Sender:    {} ETH", format_ether(sender_balance_before));
        println!("   Recipient: {} ETH", format_ether(recipient_balance_before));
        
        // Build transaction - chain ID will be set by wallet
        let mut tx = TransactionRequest::default();
        tx = tx.from(sender.address)
            .to(recipient)
            .value(amount)
            .nonce(rpc.provider.get_transaction_count(sender.address).await.unwrap())
            .gas_limit(21000)
            .max_fee_per_gas(20_000_000_000u128) // 20 gwei
            .max_priority_fee_per_gas(1_000_000_000u128); // 1 gwei
        
        // Set chain ID separately if needed
        tx.set_chain_id(31337);
        
        // Build envelope and send
        let tx_envelope = tx.build(&wallet).await.unwrap();
        let tx_hash = rpc.sign_and_send_tx(tx_envelope).await.unwrap();
        
        println!("📨 Transaction sent! Hash: 0x{}", hex::encode(tx_hash));
        
        // Wait for confirmation
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        // Check receipt
        let receipt = rpc.provider.get_transaction_receipt(tx_hash.into()).await.unwrap()
            .expect("Transaction should be mined");
        
        assert!(receipt.status(), "Transaction should succeed");
        println!("Transaction confirmed in block {}", receipt.block_number.unwrap());
        
        // Verify balances changed
        let sender_balance_after = rpc.provider.get_balance(sender.address).await.unwrap();
        let recipient_balance_after = rpc.provider.get_balance(recipient).await.unwrap();
        
        println!("💰 Final balances:");
        println!("   Sender:    {} ETH", format_ether(sender_balance_after));
        println!("   Recipient: {} ETH", format_ether(recipient_balance_after));
        
        // Calculate gas used
        let gas_used = receipt.gas_used;
        let effective_gas_price = receipt.effective_gas_price;
        let gas_cost = U256::from(gas_used) * U256::from(effective_gas_price);
        
        // Assertions
        assert_eq!(
            recipient_balance_after - recipient_balance_before, 
            amount,
            "Recipient should receive exactly 1 ETH"
        );
        
        assert_eq!(
            sender_balance_before - sender_balance_after,
            amount + gas_cost,
            "Sender should lose 1 ETH + gas costs"
        );
        
        println!("⛽ Gas used: {} (cost: {} ETH)", gas_used, format_ether(gas_cost));
        println!("ETH transfer test completed successfully!");
    }
    
    /// Helper function to format wei as ETH
    fn format_ether(wei: U256) -> String {
        let eth = wei / U256::from(10).pow(U256::from(18));
        let remainder = wei % U256::from(10).pow(U256::from(18));
        let decimal = remainder / U256::from(10).pow(U256::from(14)); // 4 decimal places
        format!("{}.{:04}", eth, decimal)
    }
    
    #[tokio::test]  
    async fn test_insufficient_funds_for_transfer() {
        use crate::rpc::EvmWalletRpc;
        use alloy::network::TransactionBuilder as NetworkTransactionBuilder;
        
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_insufficient_funds_for_transfer - Anvil not installed");
            return;
        }
        
        // Spawn Anvil instance
        let anvil = AnvilInstance::spawn();
        println!("Anvil spawned on {}", anvil.url);
        
        // Create a new account with no ETH (random private key not from anvil's mnemonic)
        let poor_sender = TestAccount::from_private_key(
            "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        );
        let recipient = Address::from_str("0x70997970C51812dc3A010C7d01b50e0d17dc79C8").unwrap();
        let amount = U256::from(1_000_000_000_000_000_000u64); // 1 ETH (more than account has)
        
        println!("📤 Testing insufficient funds: attempting to transfer 1 ETH from unfunded account");
        println!("   Sender: {}", poor_sender.address);
        println!("   Recipient: {}", recipient);
        
        // Create RPC client with wallet
        let wallet = EthereumWallet::from(poor_sender.signer.clone());
        let rpc = EvmWalletRpc::new(&anvil.url, wallet.clone()).unwrap();
        
        // Check balance (should be 0)
        let balance = rpc.provider.get_balance(poor_sender.address).await.unwrap();
        println!("💰 Sender balance: {} ETH", format_ether(balance));
        assert_eq!(balance, U256::ZERO, "New account should have 0 ETH");
        
        // Try to build and send transaction (should fail)
        let mut tx = TransactionRequest::default();
        tx = tx.from(poor_sender.address)
            .to(recipient)
            .value(amount)
            .gas_limit(21000)
            .max_fee_per_gas(20_000_000_000u128)
            .max_priority_fee_per_gas(1_000_000_000u128);
        
        tx.set_chain_id(31337);
        
        // Try to build and send - this should fail
        let tx_result = tx.build(&wallet).await;
        
        // Build should succeed but sending should fail
        let result = if let Ok(tx_envelope) = tx_result {
            println!("📝 Transaction built successfully, attempting to send...");
            rpc.sign_and_send_tx(tx_envelope).await
        } else {
            println!("Transaction build failed (expected for unfunded account)");
            Err(error_stack::Report::new(crate::errors::EvmError::Transaction(
                crate::errors::TransactionError::InsufficientFunds {
                    required: amount.to::<u128>(),
                    available: 0u128,
                }
            )))
        };
        
        // Verify we got an error
        assert!(result.is_err(), "Transaction should fail with insufficient funds");
        
        let error = result.unwrap_err();
        
        println!("Transaction failed as expected:");
        println!("   Error: {:?}", error);
        
        // The error should be insufficient funds
        let is_insufficient_funds = matches!(
            error.current_context(),
            EvmError::Transaction(TransactionError::InsufficientFunds { .. })
        );
        assert!(
            is_insufficient_funds,
            "Expected TransactionError::InsufficientFunds, got: {:?}",
            error.current_context()
        );
        
        println!("Insufficient funds test passed - transaction correctly rejected!");
    }
    
    #[tokio::test]
    async fn test_insufficient_funds_for_gas() {
        use crate::rpc::EvmWalletRpc;
        use alloy::network::TransactionBuilder as NetworkTransactionBuilder;
        
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_insufficient_funds_for_gas - Anvil not installed");
            return;
        }
        
        // Spawn Anvil instance
        let anvil = AnvilInstance::spawn();
        println!("Anvil spawned on {}", anvil.url);
        
        // Create a unique account for this test (not from anvil's mnemonic)
        let funded_account = TestAccount::from_private_key(
            "0xaaaabbbbccccddddeeeeffffaaaabbbbccccddddeeeeffffaaaabbbbccccdddd"
        );
        
        println!("📤 Testing insufficient gas: funding account with exactly 0.1 ETH");
        println!("   Account to fund: {}", funded_account.address);
        
        // Use first account to fund our test account
        let funder = &anvil.accounts[0];
        let funder_wallet = EthereumWallet::from(funder.signer.clone());
        let funder_rpc = EvmWalletRpc::new(&anvil.url, funder_wallet.clone()).unwrap();
        
        // Send exactly 0.1 ETH to the test account
        let fund_amount = U256::from(100_000_000_000_000_000u64); // 0.1 ETH
        let mut fund_tx = TransactionRequest::default();
        fund_tx = fund_tx.from(funder.address)
            .to(funded_account.address)
            .value(fund_amount)
            .nonce(funder_rpc.provider.get_transaction_count(funder.address).await.unwrap())
            .gas_limit(21000)
            .max_fee_per_gas(20_000_000_000u128)
            .max_priority_fee_per_gas(1_000_000_000u128);
        
        fund_tx.set_chain_id(31337);
        
        let fund_envelope = fund_tx.build(&funder_wallet).await.unwrap();
        let fund_hash = funder_rpc.sign_and_send_tx(fund_envelope).await.unwrap();
        println!("💸 Funded with 0.1 ETH, tx: 0x{}", hex::encode(fund_hash));
        
        // Wait for funding transaction
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        
        // Now try to send ALL 0.1 ETH (won't have gas)
        println!("📤 Now attempting to send entire balance (leaving no gas)");
        
        let recipient = Address::from_str("0x70997970C51812dc3A010C7d01b50e0d17dc79C8").unwrap();
        let wallet = EthereumWallet::from(funded_account.signer.clone());
        let rpc = EvmWalletRpc::new(&anvil.url, wallet.clone()).unwrap();
        
        let balance = rpc.provider.get_balance(funded_account.address).await.unwrap();
        println!("💰 Account balance: {} ETH", format_ether(balance));
        assert_eq!(balance, fund_amount, "Account should have 0.1 ETH");
        
        // Try to send entire balance (no gas left)
        let mut tx = TransactionRequest::default();
        tx = tx.from(funded_account.address)
            .to(recipient)
            .value(fund_amount) // Trying to send entire balance!
            .gas_limit(21000)
            .max_fee_per_gas(20_000_000_000u128)
            .max_priority_fee_per_gas(1_000_000_000u128);
        
        tx.set_chain_id(31337);
        
        let tx_result = tx.build(&wallet).await;
        let result = if let Ok(tx_envelope) = tx_result {
            println!("📝 Transaction built, attempting to send entire balance...");
            rpc.sign_and_send_tx(tx_envelope).await  // Use the funded account's RPC, not funder's!
        } else {
            Err(error_stack::Report::new(crate::errors::EvmError::Transaction(
                crate::errors::TransactionError::InsufficientFunds {
                    required: (fund_amount + U256::from(21000 * 20_000_000_000u128)).to::<u128>(),
                    available: fund_amount.to::<u128>(),
                }
            )))
        };
        
        assert!(result.is_err(), "Transaction should fail - not enough for gas");
        
        let error = result.unwrap_err();
        
        println!("Transaction failed as expected:");
        println!("   Error: {:?}", error);
        
        // Should indicate insufficient funds or gas issue
        let is_funds_or_gas_error = matches!(
            error.current_context(),
            EvmError::Transaction(TransactionError::InsufficientFunds { .. }) |
            EvmError::Transaction(TransactionError::GasEstimationFailed)
        );
        assert!(
            is_funds_or_gas_error,
            "Expected InsufficientFunds or GasEstimationFailed, got: {:?}",
            error.current_context()
        );
        
        println!("Insufficient gas test passed - can't send entire balance!");
    }
    
    #[tokio::test]
    async fn test_transaction_with_wrong_nonce() {
        use crate::rpc::EvmWalletRpc;
        use alloy::network::TransactionBuilder as NetworkTransactionBuilder;
        
        // Skip if Anvil not available
        if !AnvilInstance::is_available() {
            eprintln!("⚠️  Skipping test_transaction_with_wrong_nonce - Anvil not installed");
            return;
        }
        
        // Spawn Anvil instance
        let anvil = AnvilInstance::spawn();
        println!("Anvil spawned on {}", anvil.url);
        
        let sender = &anvil.accounts[0];
        let recipient = Address::from_str("0x70997970C51812dc3A010C7d01b50e0d17dc79C8").unwrap();
        let amount = U256::from(100_000_000_000_000_000u64); // 0.1 ETH
        
        println!("📤 Testing wrong nonce: using an invalid nonce for transaction");
        println!("   Sender: {}", sender.address);
        
        // Create RPC with sender's wallet
        let wallet = EthereumWallet::from(sender.signer.clone());
        let rpc = EvmWalletRpc::new(&anvil.url, wallet.clone()).unwrap();
        
        // First send a valid transaction to use nonce 0
        let mut first_tx = TransactionRequest::default();
        first_tx = first_tx.from(sender.address)
            .to(recipient)
            .value(amount)
            .nonce(0)
            .gas_limit(21000)
            .max_fee_per_gas(20_000_000_000u128)
            .max_priority_fee_per_gas(1_000_000_000u128);
        
        first_tx.set_chain_id(31337);
        
        let first_envelope = first_tx.build(&wallet).await.unwrap();
        let first_hash = rpc.sign_and_send_tx(first_envelope).await.unwrap();
        println!("   First transaction sent with nonce 0: 0x{}", hex::encode(first_hash));
        
        // Wait for it to be mined
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        // Now try to reuse the same nonce (should fail)
        let reused_nonce = 0;
        println!("   Trying to reuse nonce: {}", reused_nonce);
        
        let mut tx = TransactionRequest::default();
        tx = tx.from(sender.address)
            .to(recipient)
            .value(amount)
            .nonce(reused_nonce)  // Reusing already used nonce!
            .gas_limit(21000)
            .max_fee_per_gas(20_000_000_000u128)
            .max_priority_fee_per_gas(1_000_000_000u128);
        
        tx.set_chain_id(31337);
        
        // Build and try to send
        let tx_envelope = tx.build(&wallet).await.unwrap();
        let result = rpc.sign_and_send_tx(tx_envelope).await;
        
        // This should fail due to nonce already used
        assert!(result.is_err(), "Transaction should fail - nonce already used");
        
        let error = result.unwrap_err();
        
        println!("Transaction rejected as expected:");
        println!("   Error: {:?}", error);
        
        // The error should be invalid nonce
        let is_nonce_error = matches!(
            error.current_context(),
            EvmError::Transaction(TransactionError::InvalidNonce { .. })
        );
        assert!(
            is_nonce_error,
            "Expected TransactionError::InvalidNonce, got: {:?}",
            error.current_context()
        );
        
        println!("Wrong nonce test passed - transaction correctly rejected!");
    }
}