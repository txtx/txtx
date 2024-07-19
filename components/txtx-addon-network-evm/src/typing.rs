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
        constructor_args: {
            documentation: "The abi encoded constructor arguments, if provided.",
            typing: Type::string(),
            optional: true,
            interpolable: false
        },
        init_code: {
            documentation: "The compiled contract bytecode concatenated with the abi encoded constructor arguments, if provided.",
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
