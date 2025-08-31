//! Unit tests for VerificationError types used by check_confirmations action
//! 
//! These tests verify error formatting and context attachment for transaction
//! verification failures.

#[cfg(test)]
mod verification_error_tests {
    use crate::errors::{EvmError, VerificationError};
    use error_stack::Report;

    #[test]
    fn test_verification_error_display() {
        // Test TransactionNotFound error
        let err = VerificationError::TransactionNotFound {
            tx_hash: "0xabc123".to_string(),
        };
        assert_eq!(err.to_string(), "Transaction 0xabc123 not found");

        // Test TransactionReverted with reason
        let err = VerificationError::TransactionReverted {
            tx_hash: "0xdef456".to_string(),
            reason: Some("Insufficient balance".to_string()),
        };
        assert_eq!(
            err.to_string(),
            "Transaction 0xdef456 reverted: Insufficient balance"
        );

        // Test TransactionReverted without reason
        let err = VerificationError::TransactionReverted {
            tx_hash: "0xdef456".to_string(),
            reason: None,
        };
        assert_eq!(err.to_string(), "Transaction 0xdef456 reverted");

        // Test LogDecodingFailed
        let err = VerificationError::LogDecodingFailed {
            tx_hash: "0x789abc".to_string(),
            error: "Invalid ABI".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Failed to decode logs for transaction 0x789abc: Invalid ABI"
        );

        // Test InsufficientConfirmations
        let err = VerificationError::InsufficientConfirmations {
            required: 12,
            current: 3,
        };
        assert_eq!(
            err.to_string(),
            "Insufficient confirmations: 12 required, 3 current"
        );
    }

    #[test]
    fn test_verification_error_with_context() {
        let base_error = VerificationError::TransactionReverted {
            tx_hash: "0x123".to_string(),
            reason: Some("execution reverted: ERC20: transfer amount exceeds balance".to_string()),
        };

        let report = Report::new(EvmError::Verification(base_error))
            .attach_printable("Failed during token transfer")
            .attach_printable("Account: 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8")
            .attach_printable("Required: 1000 USDC")
            .attach_printable("Available: 500 USDC");

        // Verify error type
        matches!(
            report.current_context(),
            EvmError::Verification(VerificationError::TransactionReverted { tx_hash, reason })
            if tx_hash == "0x123" && reason.as_ref().map(|r| r.contains("transfer amount exceeds balance")).unwrap_or(false)
        );

        // Also check that the error chain contains our key information for debugging
        // Verify error type
        matches!(
            report.current_context(),
            EvmError::Verification(VerificationError::LogDecodingFailed { tx_hash, error })
            if tx_hash == "0xfeedface" && error.contains("Unknown event signature")
        );

        // Also check message formatting
        let error_str = format!("{:?}", report);
        assert!(error_str.contains("Failed to decode logs"));
        assert!(error_str.contains("0xfeedface"));
    }
}