use txtx_addon_kit::types::{
    diagnostics::Diagnostic,
    types::{Type, TypeImplementation, TypeSpecification},
};

lazy_static! {
    pub static ref ETH_ADDRESS: TypeSpecification = define_addon_type! {
        EthereumAddress => {
            name: "eth_address",
            documentation: "A 20-byte Ethereum address.",
        }
    };
    pub static ref BYTES: TypeSpecification = define_addon_type! {
        EthereumBytes => {
            name: "eth_bytes",
            documentation: "",
        }
    };
    pub static ref BYTES32: TypeSpecification = define_addon_type! {
        EthereumBytes32 => {
            name: "eth_bytes32",
            documentation: "",
        }
    };
    pub static ref ETH_TRANSACTION: TypeSpecification = define_addon_type! {
        EthereumTransaction => {
            name: "eth_transaction",
            documentation: "Ethereum transaction bytes.",
        }
    };
    pub static ref ETH_TX_HASH: TypeSpecification = define_addon_type! {
        EthereumTransactionHash => {
            name: "eth_tx_hash",
            documentation: "A 32-byte Ethereum transaction hash.",
        }
    };
    pub static ref ETH_INIT_CODE: TypeSpecification = define_addon_type! {
        EthereumBytes => {
            name: "eth_init_code",
            documentation: "",
        }
    };
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
            typing: Type::uint(),
            optional: true,
            interpolable: false
        },
        evm_version: {
            documentation: "The EVM version used to compile the contract.",
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
            typing: Type::uint(),
            optional: false,
            interpolable: false
        },
        evm_version: {
            documentation: "The EVM version used to compile the contract.",
            typing: Type::string(),
            optional: false,
            interpolable: false
        }
    };
    pub static ref CHAIN_DEFAULTS: Type = define_object_type! {
        chain_id: {
            documentation: "The chain id.",
            typing: Type::uint(),
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

pub struct EthereumAddress;
impl TypeImplementation for EthereumAddress {
    fn check(_ctx: &TypeSpecification, _lhs: &Type, _rhs: &Type) -> Result<bool, Diagnostic> {
        unimplemented!()
    }
}
pub struct EthereumBytes;
impl TypeImplementation for EthereumBytes {
    fn check(_ctx: &TypeSpecification, _lhs: &Type, _rhs: &Type) -> Result<bool, Diagnostic> {
        unimplemented!()
    }
}
pub struct EthereumBytes32;
impl TypeImplementation for EthereumBytes32 {
    fn check(_ctx: &TypeSpecification, _lhs: &Type, _rhs: &Type) -> Result<bool, Diagnostic> {
        unimplemented!()
    }
}
pub struct EthereumTransaction;
impl TypeImplementation for EthereumTransaction {
    fn check(_ctx: &TypeSpecification, _lhs: &Type, _rhs: &Type) -> Result<bool, Diagnostic> {
        unimplemented!()
    }
}

pub struct EthereumTransactionHash;
impl TypeImplementation for EthereumTransactionHash {
    fn check(_ctx: &TypeSpecification, _lhs: &Type, _rhs: &Type) -> Result<bool, Diagnostic> {
        unimplemented!()
    }
}
