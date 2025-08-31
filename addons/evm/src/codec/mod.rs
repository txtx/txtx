pub mod contract_deployment;
pub mod crypto;
pub mod foundry;
pub mod hardhat;
pub mod verify;
pub mod transaction;
pub mod abi;
pub mod conversion;
pub mod display;

#[cfg(test)]
mod tests;

// Re-export transaction types and functions for backward compatibility
pub use transaction::{
    CommonTransactionFields,
    TransactionType,
    build_unsigned_transaction,
    build_unsigned_transaction_v2,
    format_transaction_cost,
};

// Re-export ABI functions
pub use abi::{
    // Error-stack versions
    value_to_abi_function_args,
    value_to_abi_constructor_args,
    abi_decode_logs,
    value_to_sol_value,
    // Diagnostic version (still used)
    value_to_sol_value_compat,
};

// Re-export conversion functions for backward compatibility
pub use conversion::{
    string_to_address,
    get_typed_transaction_bytes,
    typed_transaction_bytes,
};

// Re-export display functions for backward compatibility
pub use display::format_transaction_for_display;

// Imports needed by tests and internal use

