use alloy::{
    dyn_abi::{DynSolValue, JsonAbiExt},
    json_abi::JsonAbi,
    primitives::Address,
};
use alloy_rpc_types::TransactionRequest;
use txtx_addon_kit::{indexmap::IndexMap, types::types::Value};

use crate::{
    commands::actions::get_expected_address,
    constants::{ERC_1967_PROXY_ABI_VALUE, PROXY_FACTORY_ABI_VALUE, PROXY_FACTORY_ADDRESS},
    typing::EvmValue,
};

pub mod create_opts;
pub mod proxy_opts;

pub fn create_init_code(
    bytecode: String,
    constructor_args: Option<Vec<DynSolValue>>,
    json_abi: &Option<JsonAbi>,
) -> Result<Vec<u8>, String> {
    let mut init_code = alloy::hex::decode(bytecode).map_err(|e| e.to_string())?;
    if let Some(constructor_args) = constructor_args {
        // if we have an abi, use it to validate the constructor arguments
        let mut abi_encoded_args = if let Some(json_abi) = json_abi {
            if let Some(constructor) = &json_abi.constructor {
                constructor
                    .abi_encode_input(&constructor_args)
                    .map_err(|e| format!("failed to encode constructor args: {e}"))?
            } else {
                return Err(format!(
                    "invalid arguments: constructor arguments provided, but abi has no constructor"
                ));
            }
        } else {
            constructor_args.iter().flat_map(|s| s.abi_encode()).collect::<Vec<u8>>()
        };

        init_code.append(&mut abi_encoded_args);
    } else {
        // if we have an abi, use it to validate whether constructor arguments are needed
        if let Some(json_abi) = json_abi {
            if let Some(constructor) = &json_abi.constructor {
                if constructor.inputs.len() > 0 {
                    return Err(format!(
                        "invalid arguments: no constructor arguments provided, but abi has constructor"
                    ));
                }
            }
        }
    };
    Ok(init_code)
}

pub enum ContractDeploymentTransaction {
    Create2(ContractDeploymentTransactionStatus),
    Create(ContractDeploymentTransactionStatus),
    Proxied(ProxiedDeploymentTransaction),
}

pub struct ProxiedDeploymentTransaction {
    pub tx: TransactionRequest,
    pub tx_cost: i128,
    pub expected_impl_address: Address,
    pub expected_proxy_address: Address,
}

pub enum ContractDeploymentTransactionStatus {
    AlreadyDeployed(Address),
    NotYetDeployed(TransactionDeploymentRequestData),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionDeploymentRequestData {
    pub tx: TransactionRequest,
    pub tx_cost: i128,
    pub expected_address: Address,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressAbiMap {
    pub map: IndexMap<Address, Vec<Value>>,
}
impl AddressAbiMap {
    pub fn new() -> Self {
        Self { map: IndexMap::new() }
    }
    pub fn insert_proxy_abis(&mut self, proxy_address: &Address, impl_abi: &Option<&Value>) {
        self.insert_opt(proxy_address, impl_abi);
        self.insert(proxy_address, &ERC_1967_PROXY_ABI_VALUE);
    }

    pub fn insert_proxy_factory_abi(&mut self) {
        self.insert(&PROXY_FACTORY_ADDRESS, &PROXY_FACTORY_ABI_VALUE);
    }

    pub fn insert_opt(&mut self, address: &Address, abi: &Option<&Value>) {
        if let Some(abi) = abi {
            self.insert(address, abi);
        }
    }
    pub fn insert(&mut self, address: &Address, abi: &Value) {
        self.map.entry(address.clone()).or_insert_with(Vec::new).push(abi.clone());
    }
    /// Returns a [Value::Array] representing the map, with each member of the array being a [Value::Object] with keys "address" (storing an [EvmValue::address]) and "abis" (storing an [Value::array]).
    pub fn to_value(&self) -> Value {
        let mut array = Vec::new();
        for (address, abis) in &self.map {
            let mut object = IndexMap::new();
            object.insert("address".to_string(), EvmValue::address(address));
            object.insert("abis".to_string(), Value::array(abis.clone()));
            array.push(Value::object(object));
        }
        Value::array(array)
    }
    pub fn parse_value(value: &Value) -> Result<IndexMap<Address, Vec<JsonAbi>>, String> {
        let array = value.as_array().ok_or("expected array")?;
        let mut map = IndexMap::new();
        for item in array.iter() {
            let object = item.as_object().ok_or("expected object")?;
            let address = get_expected_address(object.get("address").ok_or("missing address")?)?;
            let abis = object
                .get("abis")
                .ok_or("missing abi")?
                .as_array()
                .ok_or("abis must be an array")?
                .iter()
                .map(|abi| abi.as_string().ok_or("abi must be a string".to_string()))
                .collect::<Result<Vec<&str>, String>>()?;
            let abis: Vec<JsonAbi> = abis
                .iter()
                .map(|abi| {
                    serde_json::from_str::<JsonAbi>(abi)
                        .map_err(|e| format!("failed to decode abi: {e}"))
                })
                .collect::<Result<Vec<JsonAbi>, String>>()?;
            map.entry(address).or_insert(abis);
        }
        Ok(map)
    }
}
