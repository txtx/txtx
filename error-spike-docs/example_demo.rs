#!/usr/bin/env cargo +nightly -Zscript

//! ```cargo
//! [dependencies]
//! error-stack = "0.5"
//! ```

use error_stack::{Report, ResultExt, Context};
use std::fmt;

// Define error types similar to what we implemented
#[derive(Debug)]
enum AppError {
    Network,
    Validation,
    InsufficientFunds,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Network => write!(f, "Network operation failed"),
            AppError::Validation => write!(f, "Validation failed"),
            AppError::InsufficientFunds => write!(f, "Insufficient funds"),
        }
    }
}

impl Context for AppError {}

// Example attachments
#[derive(Debug)]
struct AccountInfo {
    address: String,
    balance: String,
    required: String,
}

#[derive(Debug)]
struct HelpText {
    message: String,
    suggestion: String,
}

fn main() {
    println!("=== Error-Stack Demo ===\n");
    
    // Example 1: Simple error with context
    println!("1. Simple error with context:");
    let result = validate_address("invalid");
    if let Err(e) = result {
        println!("{:?}\n", e);
    }
    
    // Example 2: Error with rich attachments
    println!("2. Error with rich attachments:");
    let result = transfer_funds("0x123", "0x456", 100.0);
    if let Err(e) = result {
        println!("{:?}\n", e);
    }
    
    // Example 3: Error chain with multiple contexts
    println!("3. Error chain with multiple contexts:");
    let result = complex_operation();
    if let Err(e) = result {
        println!("{:?}\n", e);
    }
}

fn validate_address(addr: &str) -> Result<String, Report<AppError>> {
    if addr.len() < 42 {
        return Err(Report::new(AppError::Validation)
            .attach_printable(format!("Invalid address format: '{}'", addr))
            .attach_printable("Address must be 42 characters long")
            .attach_printable("Example: 0x742d35Cc6634C0532925a3b844Bc9e7595f89590"));
    }
    Ok(addr.to_string())
}

fn check_balance(address: &str) -> Result<f64, Report<AppError>> {
    // Simulate checking balance
    if address == "0x123" {
        Ok(50.0) // Has 50 tokens
    } else {
        Err(Report::new(AppError::Network)
            .attach_printable("Failed to connect to RPC endpoint")
            .attach_printable("Timeout after 30 seconds"))
    }
}

fn transfer_funds(from: &str, to: &str, amount: f64) -> Result<String, Report<AppError>> {
    let balance = check_balance(from)
        .attach_printable(format!("Checking balance for {}", from))?;
    
    if balance < amount {
        return Err(Report::new(AppError::InsufficientFunds)
            .attach(AccountInfo {
                address: from.to_string(),
                balance: format!("{:.2} ETH", balance),
                required: format!("{:.2} ETH", amount),
            })
            .attach(HelpText {
                message: "Your account doesn't have enough funds for this transaction".to_string(),
                suggestion: format!("Send at least {:.2} ETH to {} to proceed", amount - balance, from),
            })
            .attach_printable(format!("Transaction from {} to {} for {} ETH", from, to, amount)));
    }
    
    Ok("0xTRANSACTION_HASH".to_string())
}

fn complex_operation() -> Result<(), Report<AppError>> {
    validate_address("bad")
        .change_context(AppError::Validation)
        .attach_printable("Failed during initialization phase")?;
    
    Ok(())
}