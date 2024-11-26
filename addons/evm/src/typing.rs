use alloy::primitives::Address;
use txtx_addon_kit::types::types::{Type, Value};

pub const EVM_ADDRESS: &str = "evm::address";
pub const EVM_BYTES: &str = "evm::bytes";
pub const EVM_BYTES32: &str = "evm::bytes32";
pub const EVM_TRANSACTION: &str = "evm::transaction";
pub const EVM_TX_HASH: &str = "evm::tx_hash";
pub const EVM_INIT_CODE: &str = "evm::init_code";
pub const EVM_SIGNER_FIELD_BYTES: &str = "evm::signer_field_bytes";
pub const EVM_UINT32: &str = "evm::uint32";
pub const EVM_UINT8: &str = "evm::uint8";

pub struct EvmValue {}

impl EvmValue {
    pub fn address(address: &Address) -> Value {
        let bytes = address.0 .0.to_vec();
        Value::addon(bytes, EVM_ADDRESS)
    }

    pub fn bytes(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, EVM_BYTES)
    }

    pub fn bytes32(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, EVM_BYTES32)
    }

    pub fn transaction(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, EVM_TRANSACTION)
    }

    pub fn tx_hash(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, EVM_TX_HASH)
    }

    pub fn init_code(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, EVM_INIT_CODE)
    }

    pub fn signer_field_bytes(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, EVM_SIGNER_FIELD_BYTES)
    }

    pub fn uint32(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, EVM_UINT32)
    }

    pub fn uint8(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, EVM_UINT8)
    }
}

lazy_static! {
    pub static ref CONTRACT_METADATA: Type = define_object_type! {
        abi: {
            documentation: "The contract abi.",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        bytecode: {
            documentation: "The compiled contract bytecode.",
            typing: Type::string(),
            optional: false,
            tainting: true
        },
        source: {
            documentation: "The contract source code.",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        compiler_version: {
            documentation: "The solc version used to compile the contract.",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        contract_name: {
            documentation: "The name of the contract being deployed.",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        optimizer_enabled: {
            documentation: "Whether the optimizer is enabled during contract compilation.",
            typing: Type::bool(),
            optional: true,
            tainting: true
        },
        optimizer_runs: {
            documentation: "The number of runs the optimizer performed.",
            typing: Type::integer(),
            optional: true,
            tainting: true
        },
        evm_version: {
            documentation: "The EVM version used to compile the contract.",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        via_ir: {
            documentation: "Coming soon",
            typing: Type::string(),
            optional: true,
            tainting: true
        }
    };
    pub static ref DEPLOYMENT_ARTIFACTS_TYPE: Type = define_object_type! {
        abi: {
            documentation: "The contract abi.",
            typing: Type::string(),
            optional: false,
            tainting: true
        },
        bytecode: {
            documentation: "The compiled contract bytecode.",
            typing: Type::string(),
            optional: false,
            tainting: true
        },
        source: {
            documentation: "The contract source code.",
            typing: Type::string(),
            optional: false,
            tainting: true
        },
        compiler_version: {
            documentation: "The solc version used to compile the contract.",
            typing: Type::string(),
            optional: false,
            tainting: true
        },
        contract_name: {
            documentation: "The name of the contract being deployed.",
            typing: Type::string(),
            optional: false,
            tainting: true
        },
        optimizer_enabled: {
            documentation: "Whether the optimizer is enabled during contract compilation.",
            typing: Type::bool(),
            optional: false,
            tainting: true
        },
        optimizer_runs: {
            documentation: "The number of runs the optimizer performed.",
            typing: Type::integer(),
            optional: false,
            tainting: true
        },
        evm_version: {
            documentation: "The EVM version used to compile the contract.",
            typing: Type::string(),
            optional: false,
            tainting: true
        },
        via_ir: {
            documentation: "Coming soon",
            typing: Type::string(),
            optional: true,
            tainting: true
        }
    };
    pub static ref CHAIN_DEFAULTS: Type = define_object_type! {
        chain_id: {
            documentation: "The chain id.",
            typing: Type::integer(),
            optional: false,
            tainting: true
        },
        rpc_api_url: {
            documentation: "The RPC API URL for the chain.",
            typing: Type::string(),
            optional: false,
            tainting: true
        }
    };
    pub static ref CREATE2_OPTS: Type = define_map_type! {
        salt: {
            documentation: "The salt value used to calculate the contract address. This value must be a 32-byte hex string.",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        factory_address: {
            documentation: "To deploy the contract with an alternative factory, provide the address of the factory contract.",
            typing: Type::addon(EVM_ADDRESS),
            optional: true,
            tainting: true
        },
        factory_abi: {
            documentation: "The ABI of the alternative create2 factory contract, optionally used to check input arguments before sending the transaction to the chain.",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        factory_function_name: {
            documentation: "If an alternative create2 factory is used, the name of the function to call.",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        factory_function_args: {
            documentation: "If an alternative create2 factory is used, the arguments to pass to the function.",
            typing: Type::string(),
            optional: true,
            tainting: true
        }
    };
    pub static ref PROXY_CONTRACT_OPTS: Type = define_map_type! {
        create_opcode: {
            documentation: "The create opcode to use for deployment. Options are 'create' and 'create2'. The default is 'create2'.",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        create2: {
            documentation: "Options for deploying the contract with the CREATE2 opcode, overwriting txtx default options.",
            typing: CREATE2_OPTS.clone(),
            optional: true,
            tainting: true
        },
        contract: {
            documentation: indoc!{r#"
                The proxy contract to deploy. At a minimum, this should be an object with a key `bytecode` and the contract bytecode.
                The abi field can also be provided to add type checking for the constructor arguments.
                The `evm::get_contract_from_foundry_project` and `evm::get_contract_from_hardhat_project` functions can be used to retrieve the contract object.
            "#},
            typing: CONTRACT_METADATA.clone(),
            optional: true,
            tainting: true
        },
        constructor_args: {
            documentation: "The optional constructor arguments for the proxy contract.",
            typing: Type::array(Type::string()),
            optional: true,
            tainting: true
        }
    };
    pub static ref PROXIED_CONTRACT_INITIALIZER: Type = define_map_type! {
        function_name: {
            documentation: "The name of the initializer function to call.",
            typing: Type::string(),
            optional: false,
            tainting: true
        },
        function_args: {
            documentation: "The arguments to pass to the initializer function.",
            typing: Type::array(Type::string()),
            optional: true,
            tainting: true
        }
    };
}
