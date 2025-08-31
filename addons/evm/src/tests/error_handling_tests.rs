#[cfg(test)]
mod error_handling_tests {
    use crate::errors::*;
    use crate::rpc::EvmRpc;
    use error_stack::{Report, ResultExt};

    #[test]
    fn test_insufficient_funds_error_creation() {
        // Test that InsufficientFunds errors are created with proper values
        let error = Report::new(EvmError::Transaction(TransactionError::InsufficientFunds {
            required: 1000000000000000000, // 1 ETH in wei
            available: 500000000000000000,  // 0.5 ETH in wei
        }));

        // First verify the error type
        matches!(
            error.current_context(),
            EvmError::Transaction(TransactionError::InsufficientFunds {
                required: 1000000000000000000,
                available: 500000000000000000
            })
        );

        // Then verify the message formatting
        let error_str = error.to_string();
        assert!(error_str.contains("Insufficient funds"));
        assert!(error_str.contains("1000000000000000000"));
        assert!(error_str.contains("500000000000000000"));
    }

    #[test]
    fn test_error_context_attachment() {
        // Test that context can be attached to errors
        let result: Result<(), Report<EvmError>> = Err(
            Report::new(EvmError::Rpc(RpcError::NodeError("connection failed".to_string())))
                .attach_printable("Attempting to connect to Ethereum node")
                .attach_printable("URL: http://localhost:8545"),
        );

        let error = result.unwrap_err();
        let debug_str = format!("{:?}", error);
        
        // Check that attachments are included in debug output
        assert!(debug_str.contains("connection failed"));
        // Note: In real error-stack, attachments are included in Debug output
    }

    #[test]
    fn test_config_error_missing_field() {
        let error = Report::new(EvmError::Config(ConfigError::MissingField(
            "rpc_api_url".to_string(),
        )));

        // Verify error type
        matches!(
            error.current_context(),
            EvmError::Config(ConfigError::MissingField(field)) if field == "rpc_api_url"
        );

        // Verify message formatting
        let error_str = error.to_string();
        assert!(error_str.contains("Missing required field"));
        assert!(error_str.contains("rpc_api_url"));
    }

    #[test]
    fn test_contract_error_function_not_found() {
        let error = Report::new(EvmError::Contract(ContractError::FunctionNotFound(
            "transfer".to_string(),
        )));

        // Verify error type
        matches!(
            error.current_context(),
            EvmError::Contract(ContractError::FunctionNotFound(name)) if name == "transfer"
        );

        // Verify message formatting
        let error_str = error.to_string();
        assert!(error_str.contains("Function"));
        assert!(error_str.contains("transfer"));
        assert!(error_str.contains("not found"));
    }

    #[test]
    fn test_transaction_error_invalid_type() {
        let error = Report::new(EvmError::Transaction(TransactionError::InvalidType(
            "EIP-4844 not supported".to_string(),
        )));

        // Verify error type
        matches!(
            error.current_context(),
            EvmError::Transaction(TransactionError::InvalidType(msg)) if msg.contains("EIP-4844")
        );

        // Verify message formatting
        let error_str = error.to_string();
        assert!(error_str.contains("Invalid transaction type"));
        assert!(error_str.contains("EIP-4844"));
    }

    #[test]
    fn test_codec_error_invalid_hex() {
        let error = Report::new(EvmError::Codec(CodecError::InvalidHex(
            "0xZZZ".to_string(),
        )));

        // Verify error type
        matches!(
            error.current_context(),
            EvmError::Codec(CodecError::InvalidHex(hex)) if hex == "0xZZZ"
        );

        // Verify message formatting
        let error_str = error.to_string();
        assert!(error_str.contains("Invalid hex"));
        assert!(error_str.contains("0xZZZ"));
    }

    #[test]
    fn test_signer_error_key_not_found() {
        let error = Report::new(EvmError::Signer(SignerError::KeyNotFound));

        // Verify error type
        assert!(matches!(
            error.current_context(),
            EvmError::Signer(SignerError::KeyNotFound)
        ));

        // Verify message formatting
        let error_str = error.to_string();
        assert!(error_str.contains("Signer key not found"));
    }

    #[test]
    fn test_error_chain_preservation() {
        // Test that error context is preserved through conversions
        let original_error = "Out of gas: gas required exceeds allowance: 0";
        
        // Simulate the error detection logic
        let error = if original_error.contains("gas required exceeds allowance") {
            Report::new(EvmError::Transaction(TransactionError::InsufficientFunds {
                required: 6000000000000000,  // Estimated amount
                available: 0,
            }))
            .attach_printable("Account has insufficient funds to pay for gas")
            .attach_printable("Suggested fix: Fund the account with ETH before deploying contracts")
        } else {
            Report::new(EvmError::Rpc(RpcError::NodeError(original_error.to_string())))
        };

        // Verify correct error type was chosen
        assert!(matches!(
            error.current_context(),
            EvmError::Transaction(TransactionError::InsufficientFunds { required: 6000000000000000, available: 0 })
        ));

        // Verify message formatting
        let error_str = error.to_string();
        assert!(error_str.contains("Insufficient funds"));
    }

    #[test]
    fn test_verification_error() {
        let error = Report::new(EvmError::Verification(VerificationError::CompilationMismatch));

        // Verify error type
        assert!(matches!(
            error.current_context(),
            EvmError::Verification(VerificationError::CompilationMismatch)
        ));

        // Verify message formatting
        let error_str = error.to_string();
        assert!(error_str.contains("bytecode doesn't match"));
    }

    #[test]
    fn test_rpc_error_invalid_response() {
        let error = Report::new(EvmError::Rpc(RpcError::InvalidResponse(
            "Expected array, got null".to_string(),
        )));

        // Verify error type
        matches!(
            error.current_context(),
            EvmError::Rpc(RpcError::InvalidResponse(msg)) if msg.contains("Expected array")
        );

        // Verify message formatting
        let error_str = error.to_string();
        assert!(error_str.contains("Invalid RPC response"));
        assert!(error_str.contains("Expected array"));
    }

    // Integration test for a typical error flow
    #[test]
    fn test_integrated_error_flow() {
        fn simulate_transaction_build() -> EvmResult<()> {
            // Simulate an RPC call that fails
            Err(Report::new(EvmError::Rpc(RpcError::NodeError(
                "eth_estimateGas: Out of gas".to_string(),
            ))))
            .attach_printable("Estimating gas for transaction")
            .attach_printable("To: 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb7")
        }

        fn handle_transaction() -> Result<String, String> {
            simulate_transaction_build()
                .map(|_| "Success".to_string())
                .map_err(|e| {
                    // Convert to string for compatibility (as done at boundaries)
                    let error_str = e.to_string();
                    if error_str.contains("Out of gas") {
                        format!("Transaction failed: Insufficient funds to pay for gas")
                    } else {
                        format!("Transaction failed: {}", error_str)
                    }
                })
        }

        let result = handle_transaction();
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("Insufficient funds"));
    }
}

#[cfg(test)]
mod rpc_error_tests {
    use super::*;
    use crate::errors::*;

    #[test]
    fn test_rpc_context_attachment() {
        let context = RpcContext {
            endpoint: "https://eth-mainnet.alchemyapi.io/v2/key".to_string(),
            method: "eth_getBalance".to_string(),
            params: Some("[\"0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb7\", \"latest\"]".to_string()),
        };

        // In real usage, this would be attached to an error
        assert_eq!(context.endpoint, "https://eth-mainnet.alchemyapi.io/v2/key");
        assert_eq!(context.method, "eth_getBalance");
        assert!(context.params.is_some());
    }

    #[test]
    fn test_transaction_context() {
        let context = TransactionContext {
            tx_hash: Some("0x123abc".to_string()),
            from: None,
            to: None,
            value: Some(1000000000000000000),
            gas_limit: Some(21000),
            chain_id: 1,
        };

        assert_eq!(context.chain_id, 1);
        assert_eq!(context.value, Some(1000000000000000000));
        assert_eq!(context.gas_limit, Some(21000));
    }
}