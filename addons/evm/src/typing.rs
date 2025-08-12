use std::str::FromStr;

use alloy::{
    json_abi::{Function, Param},
    primitives::Address,
};
use alloy_rpc_types::Log;
use foundry_compilers_artifacts_solc::Metadata;
use txtx_addon_kit::{
    hex,
    indexmap::IndexMap,
    types::{
        diagnostics::Diagnostic,
        stores::ValueStore,
        types::{ObjectType, Type, Value},
    },
};

use crate::{codec::foundry::BytecodeData, constants::LINKED_LIBRARIES};

pub const EVM_ADDRESS: &str = "evm::address";
pub const EVM_BYTES: &str = "evm::bytes";
pub const EVM_BYTES32: &str = "evm::bytes32";
pub const EVM_TRANSACTION: &str = "evm::transaction";
pub const EVM_TX_HASH: &str = "evm::tx_hash";
pub const EVM_INIT_CODE: &str = "evm::init_code";
pub const EVM_SIGNER_FIELD_BYTES: &str = "evm::signer_field_bytes";
pub const EVM_UINT256: &str = "evm::uint256";
pub const EVM_UINT32: &str = "evm::uint32";
pub const EVM_UINT8: &str = "evm::uint8";
pub const EVM_FUNCTION_CALL: &str = "evm::function_call";
pub const EVM_SIM_RESULT: &str = "evm::sim_result";
pub const EVM_KNOWN_SOL_PARAM: &str = "evm::known_sol_param";
pub const EVM_FOUNDRY_COMPILED_METADATA: &str = "evm::foundry_compiled_metadata";
pub const EVM_FOUNDRY_BYTECODE_DATA: &str = "evm::foundry_bytecode_data";

pub struct EvmValue {}

pub fn is_hex(str: &str) -> bool {
    decode_hex(str).map(|_| true).unwrap_or(false)
}

pub fn decode_hex(str: &str) -> Result<Vec<u8>, Diagnostic> {
    let stripped = if str.starts_with("0x") { &str[2..] } else { &str[..] };
    hex::decode(stripped)
        .map_err(|e| diagnosed_error!("string '{}' could not be decoded to hex bytes: {}", str, e))
}

impl EvmValue {
    pub fn address(address: &Address) -> Value {
        let bytes = address.0 .0.to_vec();
        Value::addon(bytes, EVM_ADDRESS)
    }
    pub fn to_address(value: &Value) -> Result<Address, Diagnostic> {
        match value.as_string() {
            Some(s) => {
                if is_hex(s) {
                    let hex = decode_hex(s).map_err(|e| e)?;
                    if hex.len() != 20 {
                        return Err(diagnosed_error!(
                            "expected 20 bytes for address, got {}",
                            hex.len()
                        ));
                    }
                    let bytes: [u8; 20] = hex[0..20]
                        .try_into()
                        .map_err(|e| diagnosed_error!("could not convert value to address: {e}"))?;
                    return Ok(Address::from_slice(&bytes));
                }
                return Address::from_str(s)
                    .map_err(|e| diagnosed_error!("could not convert value to address: {e}"));
            }
            None => {}
        };
        let bytes = value.to_bytes();
        if bytes.len() != 20 {
            return Err(diagnosed_error!("expected 20 bytes for address, got {}", bytes.len()));
        }
        let bytes: [u8; 20] = bytes[0..20]
            .try_into()
            .map_err(|e| diagnosed_error!("could not convert value to address: {e}"))?;
        Ok(Address::from_slice(&bytes))
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

    pub fn uint256(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, EVM_UINT256)
    }

    pub fn uint8(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, EVM_UINT8)
    }

    pub fn function_call(bytes: Vec<u8>) -> Value {
        Value::addon(bytes, EVM_FUNCTION_CALL)
    }

    /// The result from a transaction simulation. Stored with the ABI's function specification bytes,
    /// if available. This allows for decoding the result in the future.
    pub fn sim_result(sim_result_bytes: Vec<u8>, function_spec: Option<Vec<u8>>) -> Value {
        let bytes = serde_json::to_vec(&(sim_result_bytes, function_spec)).unwrap();
        Value::addon(bytes, EVM_SIM_RESULT)
    }

    pub fn to_sim_result(value: &Value) -> Result<(Vec<u8>, Option<Function>), Diagnostic> {
        let err_msg = "could not convert value to sim result";
        let addon_data = value
            .as_addon_data()
            .ok_or_else(|| diagnosed_error!("{err_msg}: not an addon data type"))?;
        if addon_data.id != EVM_SIM_RESULT {
            return Err(diagnosed_error!(
                "{err_msg}: expected type {EVM_SIM_RESULT}, got {}",
                addon_data.id
            ));
        }
        let (sim_result_bytes, function_spec): (Vec<u8>, Option<Vec<u8>>) =
            serde_json::from_slice(&addon_data.bytes)
                .map_err(|e| diagnosed_error!("{err_msg}: {e}"))?;
        let fn_spec = if let Some(fn_spec) = function_spec {
            let fn_spec: Function = serde_json::from_slice(&fn_spec).map_err(|e| {
                diagnosed_error!("{err_msg}: could not deserialize expected type: {e}")
            })?;
            Some(fn_spec)
        } else {
            None
        };
        Ok((sim_result_bytes, fn_spec))
    }

    /// A value with a known Solidity Param that it should be decoded as.
    pub fn known_sol_param(value: &Value, param: &Param) -> Value {
        let bytes = serde_json::to_vec(&(value, param)).unwrap();
        Value::addon(bytes, EVM_KNOWN_SOL_PARAM)
    }

    pub fn to_known_sol_param(value: &Value) -> Result<(Value, Param), Diagnostic> {
        let err_msg = "could not convert value to known sol type";
        let addon_data = value
            .as_addon_data()
            .ok_or_else(|| diagnosed_error!("{err_msg}: not an addon data type"))?;
        if addon_data.id != EVM_KNOWN_SOL_PARAM {
            return Err(diagnosed_error!(
                "{err_msg}: expected type {EVM_KNOWN_SOL_PARAM}, got {}",
                addon_data.id
            ));
        }
        let (value, ty): (Value, Param) = serde_json::from_slice(&addon_data.bytes)
            .map_err(|e| diagnosed_error!("{err_msg}: {e}"))?;
        Ok((value, ty))
    }

    pub fn foundry_compiled_metadata(value: &Metadata) -> Result<Value, Diagnostic> {
        let bytes = serde_json::to_vec(value)
            .map_err(|e| diagnosed_error!("could not serialize foundry metadata: {e}"))?;
        Ok(Value::addon(bytes, EVM_FOUNDRY_COMPILED_METADATA))
    }

    pub fn to_foundry_compiled_metadata(value: &Value) -> Result<Metadata, Diagnostic> {
        let err_msg = "could not convert value to foundry metadata";
        let addon_data = value
            .as_addon_data()
            .ok_or_else(|| diagnosed_error!("{err_msg}: not an addon data type"))?;
        if addon_data.id != EVM_FOUNDRY_COMPILED_METADATA {
            return Err(diagnosed_error!(
                "{err_msg}: expected type {EVM_FOUNDRY_COMPILED_METADATA}, got {}",
                addon_data.id
            ));
        }
        let metadata: Metadata = serde_json::from_slice(&addon_data.bytes)
            .map_err(|e| diagnosed_error!("{err_msg}: {e}"))?;
        Ok(metadata)
    }

    pub fn foundry_bytecode_data(value: &BytecodeData) -> Result<Value, Diagnostic> {
        let bytes = serde_json::to_vec(value)
            .map_err(|e| diagnosed_error!("could not serialize foundry bytecode data: {e}"))?;
        Ok(Value::addon(bytes, EVM_FOUNDRY_BYTECODE_DATA))
    }

    pub fn to_foundry_bytecode_data(value: &Value) -> Result<BytecodeData, Diagnostic> {
        let err_msg = "could not convert value to foundry bytecode data";
        let addon_data = value
            .as_addon_data()
            .ok_or_else(|| diagnosed_error!("{err_msg}: not an addon data type"))?;
        if addon_data.id != EVM_FOUNDRY_BYTECODE_DATA {
            return Err(diagnosed_error!(
                "{err_msg}: expected type {EVM_FOUNDRY_BYTECODE_DATA}, got {}",
                addon_data.id
            ));
        }
        let bytecode: BytecodeData = serde_json::from_slice(&addon_data.bytes)
            .map_err(|e| diagnosed_error!("{err_msg}: {e}"))?;
        Ok(bytecode)
    }

    pub fn parse_linked_libraries(
        values: &ValueStore,
    ) -> Result<Option<IndexMap<String, Address>>, Diagnostic> {
        let linked_libraries = values
            .get_object(LINKED_LIBRARIES)
            .map(|lib| {
                lib.iter()
                    .map(|(k, v)| EvmValue::to_address(v).map(|a| (k.clone(), a)))
                    .collect::<Result<IndexMap<String, Address>, _>>()
            })
            .transpose()
            .map_err(|d| {
                diagnosed_error!("each entry of a linked library must be an address: {d}")
            })?;

        Ok(linked_libraries)
    }
}

pub struct RawLog;
impl RawLog {
    pub fn to_value(log: &Log) -> Value {
        let log_address = log.address();
        let topics = log
            .topics()
            .iter()
            .map(|topic| Value::string(hex::encode(topic.0.to_vec())))
            .collect::<Vec<Value>>();
        let data = hex::encode(log.data().data.to_vec());
        let obj = ObjectType::from(vec![
            ("address", EvmValue::address(&log_address)),
            ("topics", Value::array(topics)),
            ("data", Value::string(data)),
        ]);
        obj.to_value()
    }
}

pub struct DecodedLog;
impl DecodedLog {
    pub fn to_value(event_name: &str, address: &Address, data: Value) -> Value {
        let obj = ObjectType::from(vec![
            ("event_name", Value::string(event_name.to_string())),
            ("address", EvmValue::address(address)),
            ("data", data),
        ]);
        obj.to_value()
    }
}

lazy_static! {
    pub static ref CONTRACT_METADATA: Type = define_strict_object_type! {
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
    pub static ref DEPLOYMENT_ARTIFACTS_TYPE: Type = define_strict_object_type! {
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
    pub static ref CHAIN_DEFAULTS: Type = define_strict_object_type! {
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
    pub static ref CREATE2_OPTS: Type = define_strict_map_type! {
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
    pub static ref PROXY_CONTRACT_OPTS: Type = define_strict_map_type! {
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
        }
    };
    pub static ref PROXIED_CONTRACT_INITIALIZER: Type = define_strict_map_type! {
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
    pub static ref DECODED_LOG_OUTPUT: Type = define_strict_object_type! {
        event_name: {
            documentation: "The decoded name of the event.",
            typing: Type::string(),
            optional: false,
            tainting: true
        },
        address: {
            documentation: "The address of the contract that emitted the event.",
            typing: Type::addon(EVM_ADDRESS),
            optional: false,
            tainting: true
        },
        data: {
            documentation: "The decoded data of the event.",
            typing: Type::arbitrary_object(),
            optional: false,
            tainting: true
        }
    };
    pub static ref RAW_LOG_OUTPUT: Type = define_strict_object_type! {
        topics: {
            documentation: "The event topics.",
            typing: Type::array(Type::string()),
            optional: false,
            tainting: true
        },
        address: {
            documentation: "The address of the contract that emitted the event.",
            typing: Type::addon(EVM_ADDRESS),
            optional: false,
            tainting: true
        },
        data: {
            documentation: "The raw data of the event.",
            typing: Type::string(),
            optional: false,
            tainting: true
        }
    };
    pub static ref CONTRACT_VERIFICATION_OPTS_TYPE: Type = define_strict_map_type! {
        provider_api_url: {
            documentation: "The verification provider API url.",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        provider_url: {
            documentation: "The verification provider url, used to display a link to the verified contract.",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        provider: {
            documentation: "The provider to use for contract verification; either 'etherscan', 'blockscout', or 'sourcify'.",
            typing: Type::string(),
            optional: false,
            tainting: true
        },
        api_key: {
            documentation: "The verification provider API key.",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        throw_on_error: {
            documentation: "Dictates if the verification process should throw an error if the contract is not verified. The default is `false`.",
            typing: Type::bool(),
            optional: true,
            tainting: true
        }
    };
    pub static ref VERIFICATION_RESULT_TYPE: Type = define_strict_object_type! {
        provider: {
            documentation: "The verification provider.",
            typing: Type::string(),
            optional: false,
            tainting: true
        },
        url: {
            documentation: "The URL of the verified contract on the associated provider's explorer.",
            typing: Type::string(),
            optional: true,
            tainting: true
        },
        contract_address: {
            documentation: "The address of the contract that was verified.",
            typing: Type::string(),
            optional: false,
            tainting: true
        },
        verified: {
            documentation: "Whether the contract was verified successfully.",
            typing: Type::bool(),
            optional: false,
            tainting: true
        },
        error: {
            documentation: "The error message, if the contract was not verified successfully.",
            typing: Type::string(),
            optional: false,
            tainting: true
        }
    };
    pub static ref LINKED_LIBRARIES_TYPE: Type = define_documented_arbitrary_object_type! {
        contract_name: {
            documentation: "A contract name (key) mapped to an address. If a contract deployment requires a linked library, this contract address will be used for all occurrences of the specified library name.",
            typing: Type::addon(EVM_ADDRESS),
            optional: true,
            tainting: true
        }
    };
}
