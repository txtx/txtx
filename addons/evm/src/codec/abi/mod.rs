// ABI encoding and decoding module
// This module contains all ABI-related functions for Ethereum

pub mod encoding;
pub mod decoding;
pub mod types;

// Re-export commonly used functions
pub use encoding::{
    value_to_abi_function_args,
    value_to_abi_constructor_args,
    value_to_abi_param,
    value_to_struct_abi_type,
};

pub use decoding::abi_decode_logs;

pub use types::{
    value_to_sol_value,
    value_to_sol_value_compat,
};