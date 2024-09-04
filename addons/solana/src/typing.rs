
use txtx_addon_kit::types::types::{Type, Value};

pub const SOL_ADDRESS: &str = "evm::address";
pub const SOL_BYTES: &str = "evm::bytes";
pub const SOL_BYTES32: &str = "evm::bytes32";
pub const SOL_TRANSACTION: &str = "evm::transaction";
pub const SOL_TX_HASH: &str = "evm::tx_hash";
pub const SOL_INIT_CODE: &str = "evm::init_code";

pub struct EvmValue {}

impl EvmValue {
    pub fn address(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SOL_ADDRESS)
    }

    pub fn bytes(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SOL_BYTES)
    }

    pub fn bytes32(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SOL_BYTES32)
    }

    pub fn transaction(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SOL_TRANSACTION)
    }

    pub fn tx_hash(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SOL_TX_HASH)
    }

    pub fn init_code(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, SOL_INIT_CODE)
    }
}

lazy_static! {
    pub static ref CONTRACT_METADATA: Type = define_object_type! {
        abi: {
            documentation: "The contract abi.",
            typing: Type::string(),
            optional: true,
            interpolable: false
        },
        bytecode: {
            documentation: "The compiled contract bytecode.",
            typing: Type::string(),
            optional: false,
            interpolable: false
        },
        source: {
            documentation: "The contract source code.",
            typing: Type::string(),
            optional: true,
            interpolable: false
        },
        compiler_version: {
            documentation: "The solc version used to compile the contract.",
            typing: Type::string(),
            optional: true,
            interpolable: false
        },
        contract_name: {
            documentation: "The name of the contract being deployed.",
            typing: Type::string(),
            optional: true,
            interpolable: false
        },
        optimizer_enabled: {
            documentation: "Whether the optimizer is enabled during contract compilation.",
            typing: Type::bool(),
            optional: true,
            interpolable: false
        },
        optimizer_runs: {
            documentation: "The number of runs the optimizer performed.",
            typing: Type::integer(),
            optional: true,
            interpolable: false
        },
        evm_version: {
            documentation: "The SOL version used to compile the contract.",
            typing: Type::string(),
            optional: true,
            interpolable: false
        },
        via_ir: {
            documentation: "Coming soon",
            typing: Type::string(),
            optional: true,
            interpolable: false
        }
    };
    pub static ref DEPLOYMENT_ARTIFACTS_TYPE: Type = define_object_type! {
        abi: {
            documentation: "The contract abi.",
            typing: Type::string(),
            optional: false,
            interpolable: false
        },
        bytecode: {
            documentation: "The compiled contract bytecode.",
            typing: Type::string(),
            optional: false,
            interpolable: false
        },
        source: {
            documentation: "The contract source code.",
            typing: Type::string(),
            optional: false,
            interpolable: false
        },
        compiler_version: {
            documentation: "The solc version used to compile the contract.",
            typing: Type::string(),
            optional: false,
            interpolable: false
        },
        contract_name: {
            documentation: "The name of the contract being deployed.",
            typing: Type::string(),
            optional: false,
            interpolable: false
        },
        optimizer_enabled: {
            documentation: "Whether the optimizer is enabled during contract compilation.",
            typing: Type::bool(),
            optional: false,
            interpolable: false
        },
        optimizer_runs: {
            documentation: "The number of runs the optimizer performed.",
            typing: Type::integer(),
            optional: false,
            interpolable: false
        },
        evm_version: {
            documentation: "The SOL version used to compile the contract.",
            typing: Type::string(),
            optional: false,
            interpolable: false
        },
        via_ir: {
            documentation: "Coming soon",
            typing: Type::string(),
            optional: true,
            interpolable: false
        }
    };
    pub static ref CHAIN_DEFAULTS: Type = define_object_type! {
        chain_id: {
            documentation: "The chain id.",
            typing: Type::integer(),
            optional: false,
            interpolable: false
        },
        rpc_api_url: {
            documentation: "The RPC API URL for the chain.",
            typing: Type::string(),
            optional: false,
            interpolable: false
        }
    };
}
