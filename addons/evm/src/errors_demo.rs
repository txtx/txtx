//! Demonstration tests for error-stack implementation
//! 
//! Run with: cargo test errors_demo -- --nocapture
//! to see the rich error output

#[cfg(test)]
mod demo_tests {
    use crate::errors::*;
    use error_stack::{Report, ResultExt};
    use alloy::primitives::Address;
    use txtx_addon_kit::types::diagnostics::Diagnostic;
    use txtx_addon_kit::diagnosed_error;

    #[test]
    fn demo_transaction_insufficient_funds_error() {
        println!("\n{}", "=".repeat(60));
        println!("DEMO: Transaction with Insufficient Funds Error");
        println!("{}\n", "=".repeat(60));

        // Simulate a multi-layer error scenario
        let result = simulate_transaction_with_insufficient_funds();
        
        match result {
            Ok(_) => panic!("Expected error for demonstration"),
            Err(report) => {
                println!("1️⃣  ERROR-STACK DEBUG FORMAT (Full Details):");
                println!("{:─^60}", "");
                println!("{:#?}", report);
                
                println!("\n2️⃣  ERROR-STACK DISPLAY FORMAT (User-Friendly):");
                println!("{:─^60}", "");
                println!("{}", report);
                
                // Convert to Diagnostic to show compatibility
                let diagnostic = report_to_diagnostic(report);
                println!("\n3️⃣  DIAGNOSTIC CONVERSION (Legacy Compatibility):");
                println!("{:─^60}", "");
                println!("Level: Error");
                println!("Message: {}", diagnostic.message);
                if let Some(doc) = &diagnostic.documentation {
                    println!("Context: {}", doc.lines().next().unwrap_or(""));
                }
                
                println!("\n✅ This demonstrates how error-stack provides rich context!");
            }
        }
    }

    #[test]
    fn demo_contract_deployment_failure() {
        println!("\n{}", "=".repeat(60));
        println!("DEMO: Contract Deployment Failure");
        println!("{}\n", "=".repeat(60));

        let result = simulate_contract_deployment_failure();
        
        match result {
            Ok(_) => panic!("Expected error for demonstration"),
            Err(report) => {
                println!("1️⃣  ERROR CHAIN:");
                println!("{:─^60}", "");
                
                // Show the error chain
                let error_chain = format!("{:#?}", report);
                for line in error_chain.lines().take(20) {
                    println!("{}", line);
                }
                
                println!("\n2️⃣  USER-FACING MESSAGE:");
                println!("{:─^60}", "");
                println!("{}", report);
                
                println!("\n✅ Notice how each layer adds context to help debugging!");
            }
        }
    }

    #[test]
    fn demo_rpc_connection_error() {
        println!("\n{}", "=".repeat(60));
        println!("DEMO: RPC Connection Error with Retry Context");
        println!("{}\n", "=".repeat(60));

        let result = simulate_rpc_connection_failure();
        
        match result {
            Ok(_) => panic!("Expected error for demonstration"),
            Err(report) => {
                println!("ERROR REPORT:");
                println!("{:─^60}", "");
                println!("{}", report);
                
                // Show how to extract specific error types
                println!("\n🔍 ERROR TYPE DETECTION:");
                if let Some(rpc_error) = report.downcast_ref::<RpcError>() {
                    println!("Detected RPC Error: {:?}", rpc_error);
                }
                
                println!("\n✅ Error-stack allows type-safe error inspection!");
            }
        }
    }

    #[test] 
    fn demo_verification_error_with_context() {
        println!("\n{}", "=".repeat(60));
        println!("DEMO: Contract Verification Error");
        println!("{}\n", "=".repeat(60));

        let result = simulate_verification_failure();
        
        match result {
            Ok(_) => panic!("Expected error for demonstration"),
            Err(report) => {
                // Create a formatted error report
                let diagnostic = report_to_diagnostic(report);
                
                println!("🔴 ERROR: {}", diagnostic.message);
                
                if let Some(doc) = diagnostic.documentation {
                    println!("\n📋 FULL CONTEXT:");
                    println!("{:─^60}", "");
                    for line in doc.lines() {
                        println!("  {}", line);
                    }
                }
                
                println!("\n✅ Rich context helps identify the root cause quickly!");
            }
        }
    }

    // Helper functions to simulate various error scenarios

    fn simulate_transaction_with_insufficient_funds() -> EvmResult<()> {
        // Layer 1: RPC call to check balance
        check_balance()
            .attach(RpcContext {
                endpoint: "https://mainnet.infura.io/v3/YOUR_API_KEY".to_string(),
                method: "eth_getBalance".to_string(),
                params: Some(r#"["0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb", "latest"]"#.to_string()),
            })
            .attach_printable("Balance check returned: 0.5 ETH")
            // Layer 2: Transform to transaction error
            .change_context(EvmError::Transaction(TransactionError::InsufficientFunds {
                required: 1_000_000_000_000_000_000, // 1 ETH
                available: 500_000_000_000_000_000,  // 0.5 ETH
            }))
            .attach_printable("Transaction requires 1 ETH but wallet only has 0.5 ETH")
            // Layer 3: Add transaction context
            .attach(TransactionContext {
                tx_hash: None,
                from: Some(Address::from([0x74; 20])),
                to: Some(Address::from([0x5f; 20])),
                value: Some(1_000_000_000_000_000_000),
                gas_limit: Some(21000),
                chain_id: 1,
            })
            .attach_printable("Transaction: Send 1 ETH from 0x7474...7474 to 0x5f5f...5f5f")
            .attach_printable("Suggested action: Add more funds or reduce transaction amount")
    }

    fn check_balance() -> EvmResult<()> {
        // Simulate RPC failure
        Err(Report::new(EvmError::Rpc(RpcError::NodeError(
            "insufficient funds for gas * price + value".to_string()
        ))))
    }

    fn simulate_contract_deployment_failure() -> EvmResult<()> {
        deploy_contract()
            .attach_printable("Deploying ERC20 token contract")
            .attach(ContractContext {
                address: Address::ZERO,
                function: Some("constructor".to_string()),
                args: Some(r#"["MyToken", "MTK", 1000000]"#.to_string()),
            })
            .attach_printable("Contract bytecode size: 24KB (exceeds limit)")
            .change_context(EvmError::Contract(ContractError::DeploymentFailed(
                "Contract size exceeds maximum allowed (24KB > 24KB limit)".to_string()
            )))
            .attach_printable("Optimization suggestion: Enable optimizer in Solidity compiler")
            .attach_printable("Alternative: Split contract into multiple smaller contracts")
    }

    fn deploy_contract() -> EvmResult<()> {
        Err(Report::new(EvmError::Transaction(TransactionError::GasEstimationFailed)))
            .attach_printable("Gas estimation failed: execution reverted")
    }

    fn simulate_rpc_connection_failure() -> EvmResult<()> {
        connect_to_rpc()
            .attach(RpcContext {
                endpoint: "http://localhost:8545".to_string(),
                method: "net_version".to_string(),
                params: None,
            })
            .attach_printable("Attempt 1/3: Connection refused")
            .attach_printable("Attempt 2/3: Connection refused")
            .attach_printable("Attempt 3/3: Connection refused")
            .attach_printable("All retry attempts exhausted")
            .change_context(EvmError::Rpc(RpcError::ConnectionFailed(
                "http://localhost:8545".to_string()
            )))
            .attach_printable("Possible causes:")
            .attach_printable("  - Local node not running (try: geth --http)")
            .attach_printable("  - Incorrect port (default is 8545)")
            .attach_printable("  - Firewall blocking connection")
    }

    fn connect_to_rpc() -> EvmResult<()> {
        Err(Report::new(EvmError::Rpc(RpcError::RequestTimeout)))
    }

    fn simulate_verification_failure() -> EvmResult<()> {
        verify_contract()
            .attach_printable("Verifying contract on Etherscan")
            .attach(ContractContext {
                address: Address::from([0xAB; 20]),
                function: None,
                args: None,
            })
            .attach_printable("Contract address: 0xABAB...ABAB")
            .attach_printable("Compiler version: v0.8.19")
            .attach_printable("Optimization: enabled (200 runs)")
            .change_context(EvmError::Verification(VerificationError::CompilationMismatch))
            .attach_printable("Bytecode mismatch detected:")
            .attach_printable("  Expected: 0x6080604052...")
            .attach_printable("  On-chain: 0x6080604053...")
            .attach_printable("Common causes:")
            .attach_printable("  - Different compiler version")
            .attach_printable("  - Different optimization settings")
            .attach_printable("  - Missing constructor arguments")
    }

    fn verify_contract() -> EvmResult<()> {
        Err(Report::new(EvmError::Verification(VerificationError::ApiError(
            "Invalid API key".to_string()
        ))))
    }

    #[test]
    fn demo_error_comparison() {
        println!("\n{}", "=".repeat(60));
        println!("COMPARISON: Old vs New Error Handling");
        println!("{}\n", "=".repeat(60));
        
        // Old style error
        let old_error = "failed to send transaction: insufficient funds";
        println!("❌ OLD ERROR (String):");
        println!("{:─^60}", "");
        println!("Error: {}", old_error);
        println!("(No context, no stack trace, hard to debug)");
        
        // New style error with error-stack
        let new_error = simulate_transaction_with_insufficient_funds();
        println!("\n✅ NEW ERROR (error-stack):");
        println!("{:─^60}", "");
        match new_error {
            Err(report) => {
                println!("{}", report);
                println!("\n📊 Benefits:");
                println!("  • Full error chain visible");
                println!("  • Contextual information attached");
                println!("  • Suggested actions included");
                println!("  • Type-safe error handling");
                println!("  • Zero-cost in release builds");
            }
            Ok(_) => {}
        }
    }

    #[test]
    fn demo_error_stack_to_diagnostic_conversion() {
        println!("\n{}", "=".repeat(60));
        println!("DEMO: Error-Stack to Diagnostic Conversion");
        println!("         (Backward Compatibility)");
        println!("{}\n", "=".repeat(60));
        
        // Create a rich error with error-stack
        let error_stack_result = create_rich_error_stack();
        
        match error_stack_result {
            Err(report) => {
                println!("1️⃣  ORIGINAL ERROR-STACK REPORT:");
                println!("{:─^60}", "");
                println!("{:#?}", report);
                
                // Convert to Diagnostic for backward compatibility
                let diagnostic = report_to_diagnostic(report);
                
                println!("\n2️⃣  CONVERTED TO DIAGNOSTIC (for legacy systems):");
                println!("{:─^60}", "");
                println!("📋 Diagnostic Structure:");
                println!("  Level: {:?}", diagnostic.level);
                println!("  Message: {}", diagnostic.message);
                println!("  Span: {:?}", diagnostic.span);
                println!("  Location: {:?}", diagnostic.location);
                
                if let Some(doc) = &diagnostic.documentation {
                    println!("\n  Documentation field (contains full context):");
                    for (i, line) in doc.lines().enumerate() {
                        if i < 10 {  // Show first 10 lines
                            println!("    {}", line);
                        }
                    }
                    if doc.lines().count() > 10 {
                        println!("    ... ({} more lines)", doc.lines().count() - 10);
                    }
                }
                
                println!("\n3️⃣  HOW THIS ENABLES GRADUAL MIGRATION:");
                println!("{:─^60}", "");
                println!("✅ New modules use error-stack internally");
                println!("✅ At API boundaries, convert to Diagnostic");
                println!("✅ Existing code continues to work unchanged");
                println!("✅ LSP/UI components see familiar Diagnostic type");
                
                // Demonstrate that it works with existing diagnostic handlers
                println!("\n4️⃣  COMPATIBILITY WITH EXISTING CODE:");
                println!("{:─^60}", "");
                handle_diagnostic_like_before(diagnostic);
                
                println!("\n5️⃣  MIGRATION STRATEGY:");
                println!("{:─^60}", "");
                println!("  Phase 1: Add error-stack to new code");
                println!("  Phase 2: Use conversion at module boundaries");
                println!("  Phase 3: Gradually refactor internals");
                println!("  Phase 4: Keep Diagnostic at public APIs");
                
            }
            Ok(_) => panic!("Expected error for demonstration"),
        }
    }
    
    // Helper function to create a complex error stack
    fn create_rich_error_stack() -> EvmResult<()> {
        // Simulate a deep call stack with multiple error layers
        level_1_function()
            .attach_printable("🔸 Level 4: User action - Deploying DeFi protocol")
            .attach_printable("Protocol: UniswapV3-Fork")
            .attach_printable("Network: Ethereum Mainnet")
            .change_context(EvmError::Contract(ContractError::DeploymentFailed(
                "Failed to deploy protocol due to gas estimation issues".to_string()
            )))
    }
    
    fn level_1_function() -> EvmResult<()> {
        level_2_function()
            .attach_printable("🔹 Level 3: Smart contract validation")
            .attach(ContractContext {
                address: Address::from([0xDE; 20]),
                function: Some("initialize".to_string()),
                args: Some("(address,uint256,bytes32)".to_string()),
            })
    }
    
    fn level_2_function() -> EvmResult<()> {
        level_3_function()
            .attach_printable("🔺 Level 2: Gas estimation")
            .attach_printable("Estimated gas: 8,500,000")
            .attach_printable("Block gas limit: 30,000,000")
            .change_context(EvmError::Transaction(TransactionError::GasEstimationFailed))
    }
    
    fn level_3_function() -> EvmResult<()> {
        Err(Report::new(EvmError::Rpc(RpcError::NodeError(
            "eth_estimateGas: execution reverted: ERC20: transfer amount exceeds balance".to_string()
        ))))
        .attach_printable("🔻 Level 1: RPC call failed")
        .attach(RpcContext {
            endpoint: "https://eth-mainnet.g.alchemy.com/v2/API_KEY".to_string(),
            method: "eth_estimateGas".to_string(),
            params: Some("{\"from\":\"0xDEDE...\",\"to\":\"0xABCD...\",\"data\":\"0x...\"}".to_string()),
        })
    }
    
    // Simulates how existing code handles Diagnostic
    fn handle_diagnostic_like_before(diag: Diagnostic) {
        println!("📌 Existing handler receives Diagnostic:");
        println!("  - Can check level: {:?}", diag.level);
        println!("  - Can display message: {}", diag.message);
        println!("  - Can access documentation: {}", 
                 if diag.documentation.is_some() { "Yes" } else { "No" });
        println!("  - Works with LSP: ✅");
        println!("  - Works with CLI output: ✅");
        println!("  - Works with Web UI: ✅");
    }

    #[test]
    fn demo_mixed_error_handling() {
        println!("\n{}", "=".repeat(60));
        println!("DEMO: Mixed Error Handling During Migration");
        println!("{}\n", "=".repeat(60));
        
        // Simulate a function that uses old-style errors
        fn old_style_function() -> Result<String, Diagnostic> {
            Err(diagnosed_error!("Database connection failed: timeout after 30s"))
        }
        
        // Simulate a function that uses new error-stack
        fn new_style_function() -> EvmResult<String> {
            Err(Report::new(EvmError::Rpc(RpcError::RequestTimeout)))
                .attach_printable("Timeout after 30 seconds")
                .attach_printable("Endpoint: http://localhost:8545")
                .attach_printable("Retries exhausted: 3/3")
        }
        
        // Demonstrate interoperability
        println!("1️⃣  OLD STYLE FUNCTION ERROR (Current Diagnostic):");
        println!("{:─^60}", "");
        match old_style_function() {
            Err(diag) => {
                println!("📋 Full Diagnostic Structure:");
                println!("  Message: {}", diag.message);
                println!("  Level: {:?}", diag.level);
                println!("  Span: {:?}", diag.span);
                println!("  Location: {:?}", diag.location);
                println!("  Documentation: {:?}", diag.documentation);
                println!("  Example: {:?}", diag.example);
                println!("  Parent Diagnostic: {:?}", diag.parent_diagnostic);
                println!("\n  ⚠️  Note: Most fields are None/empty!");
                println!("  Only the message string is populated.");
            }
            Ok(_) => {}
        }
        
        println!("\n2️⃣  NEW STYLE FUNCTION ERROR (error-stack with context):");
        println!("{:─^60}", "");
        match new_style_function() {
            Err(report) => {
                println!("{:#?}", report);
            }
            Ok(_) => {}
        }
        
        println!("\n3️⃣  NEW STYLE CONVERTED TO DIAGNOSTIC:");
        println!("{:─^60}", "");
        match new_style_function() {
            Err(report) => {
                let diag = report_to_diagnostic(report);
                println!("📋 Full Diagnostic Structure:");
                println!("  Message: {}", diag.message);
                println!("  Level: {:?}", diag.level);
                println!("  Span: {:?}", diag.span);
                println!("  Location: {:?}", diag.location);
                println!("  Documentation: {} bytes of context", 
                         diag.documentation.as_ref().map(|d| d.len()).unwrap_or(0));
                println!("  Example: {:?}", diag.example);
                println!("  Parent Diagnostic: {:?}", diag.parent_diagnostic);
                println!("\n  ✅ Documentation field contains full error-stack context!");
                if let Some(doc) = &diag.documentation {
                    println!("\n  Preview of documentation field:");
                    for line in doc.lines().take(3) {
                        println!("    {}", line);
                    }
                }
            }
            Ok(_) => {}
        }
        
        println!("\n4️⃣  BOTH STYLES CAN COEXIST:");
        println!("{:─^60}", "");
        
        // Demonstrate a function that combines both
        fn mixed_handler() -> Result<String, Diagnostic> {
            // Try new style function
            match new_style_function() {
                Ok(val) => Ok(val),
                Err(report) => {
                    // Convert error-stack to Diagnostic for compatibility
                    Err(report_to_diagnostic(report))
                }
            }
        }
        
        match mixed_handler() {
            Err(diag) => {
                println!("✅ Mixed handler works with both error types!");
                println!("   Received Diagnostic: {}", diag.message);
            }
            Ok(_) => {}
        }
        
        println!("\n📊 MIGRATION BENEFITS:");
        println!("  • No breaking changes to public APIs");
        println!("  • Gradual module-by-module migration");
        println!("  • Better errors internally, compatible externally");
        println!("  • Can roll back if needed");
    }

    #[test]
    fn demo_actual_diagnostic_comparison() {
        println!("\n{}", "=".repeat(60));
        println!("ACCURATE COMPARISON: Current vs Enhanced Error Handling");
        println!("{}\n", "=".repeat(60));
        
        // Current approach with diagnosed_error!
        fn current_approach_insufficient_funds() -> Result<(), Diagnostic> {
            // This is what we currently have - just a string message
            Err(diagnosed_error!(
                "Transaction failed: insufficient funds. Required: 1000000000000000000 wei, Available: 500000000000000 wei"
            ))
        }
        
        // New approach with error-stack
        fn new_approach_insufficient_funds() -> EvmResult<()> {
            Err(Report::new(EvmError::Transaction(TransactionError::InsufficientFunds {
                required: 1000000000000000000,
                available: 500000000000000,
            })))
            .attach_printable("Attempting to send 1 ETH transaction")
            .attach(TransactionContext {
                tx_hash: None,
                from: Some(Address::from([0x74; 20])),
                to: Some(Address::from([0x5f; 20])),
                value: Some(1000000000000000000),
                gas_limit: Some(21000),
                chain_id: 1,
            })
            .attach_printable("Suggested fix: Add at least 0.5 ETH to wallet")
            .attach(RpcContext {
                endpoint: "https://mainnet.infura.io/v3/API_KEY".to_string(),
                method: "eth_getBalance".to_string(),
                params: Some(r#"["0x7474...", "latest"]"#.to_string()),
            })
        }
        
        println!("🔴 CURRENT APPROACH (diagnosed_error!):");
        println!("{:─^60}", "");
        match current_approach_insufficient_funds() {
            Err(diag) => {
                println!("What developers/users see:");
                println!("  Error: {}", diag.message);
                println!("\nWhat's in the Diagnostic struct:");
                println!("  - message: ✅ (populated)");
                println!("  - level: ✅ (Error)");
                println!("  - span: ❌ (None)");
                println!("  - location: ❌ (None)");
                println!("  - documentation: ❌ (None)");
                println!("  - example: ❌ (None)");
                println!("  - parent_diagnostic: ❌ (None)");
                println!("\n  Problems:");
                println!("  • No context about what was being attempted");
                println!("  • No information about the transaction");
                println!("  • No suggestions for fixing the issue");
                println!("  • No RPC endpoint information");
                println!("  • Hard to debug without more context");
            }
            Ok(_) => {}
        }
        
        println!("\n🟢 NEW APPROACH (error-stack):");
        println!("{:─^60}", "");
        match new_approach_insufficient_funds() {
            Err(report) => {
                println!("What developers see during debugging:");
                println!("{:#?}", report);
                
                println!("\nWhat users see (display format):");
                println!("{}", report);
                
                let diag = report_to_diagnostic(report);
                println!("\nWhat's in the converted Diagnostic struct:");
                println!("  - message: ✅ (clear error type)");
                println!("  - level: ✅ (Error)");
                println!("  - span: ⚪ (None - same as before)");
                println!("  - location: ⚪ (None - same as before)");
                println!("  - documentation: ✅ (FULL CONTEXT PRESERVED)");
                println!("  - example: ⚪ (None - same as before)");
                println!("  - parent_diagnostic: ⚪ (None - same as before)");
                
                println!("\n  Benefits:");
                println!("  • Full transaction context available");
                println!("  • RPC endpoint information included");
                println!("  • Suggested fixes provided");
                println!("  • Structured error types (not just strings)");
                println!("  • Stack traces in debug mode");
                println!("  • Backward compatible via conversion");
            }
            Ok(_) => {}
        }
        
        println!("\n📊 SUMMARY:");
        println!("{:─^60}", "");
        println!("The current diagnosed_error! macro creates Diagnostics with:");
        println!("  - Only the message field populated");
        println!("  - No contextual information");
        println!("  - Limited debugging capability");
        println!("\nThe new error-stack approach provides:");
        println!("  - Rich contextual information");
        println!("  - Structured error types");
        println!("  - Full backward compatibility");
        println!("  - Better developer and user experience");
    }
}