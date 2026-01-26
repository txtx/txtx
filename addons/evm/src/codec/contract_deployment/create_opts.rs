use alloy_dyn_abi::{DynSolValue, Word};
use alloy_json_abi::JsonAbi;
use alloy_primitives::{hex, keccak256, Address, Keccak256};
use alloy_rlp::{encode_list, BufMut, Encodable};
use txtx_addon_kit::types::stores::{ValueMap, ValueStore};
use txtx_addon_kit::types::types::Value;

use crate::codec::{build_unsigned_transaction, CommonTransactionFields, TransactionType};
use crate::commands::actions::call_contract::{
    encode_contract_call_inputs_from_abi_str, encode_contract_call_inputs_from_selector,
};
use crate::commands::actions::get_expected_address;
use crate::constants::{
    DEFAULT_CREATE2_FACTORY_ADDRESS, DEFAULT_CREATE2_SALT, FACTORY_ABI, FACTORY_ADDRESS,
    FACTORY_FUNCTION_NAME, PROXY_FACTORY_ADDRESS, SALT,
};
use crate::rpc::EvmRpc;

use super::proxy_opts::ProxiedCreationOpts;
use super::{
    ContractDeploymentTransaction, ContractDeploymentTransactionStatus,
    TransactionDeploymentRequestData,
};

#[derive(Clone, Debug)]
pub enum ContractCreationOpts {
    Create(CreateDeploymentOpts),
    Create2(Create2DeploymentOpts),
    Proxied(ProxiedCreationOpts),
}
impl ContractCreationOpts {
    pub fn new(values: &ValueStore, init_code: &Vec<u8>) -> Result<Self, String> {
        let create_opcode = values.get_string("create_opcode");
        let proxied = values.get_bool("proxied").unwrap_or(false);
        let proxy = values.get_value("proxy");
        let create2_opts = values.get_map("create2");

        match create_opcode {
            Some("create") => {
                if create2_opts.is_some() {
                    return Err("invalid arguments: 'create2' options specified, but 'create_opcode' field is set to 'create'".into());
                }
                if proxied {
                    return Err("invalid arguments: 'proxied' field is set to 'true', but 'create_opcode' field is set to 'create'".into());
                }
                if proxy.is_some() {
                    return Err("invalid arguments: 'proxy' field is set, but 'create_opcode' field is set to 'create'".into());
                }
                Ok(ContractCreationOpts::Create(CreateDeploymentOpts::new(init_code)))
            }
            None | Some("create2") => {
                let create2_opts = match create2_opts {
                    Some(opts) => Create2DeploymentOpts::new(&opts, &values.defaults, init_code)?,
                    None => Create2DeploymentOpts::default(init_code),
                };
                if let Some(proxy_opts) =
                    ProxiedCreationOpts::from_value_store(values, &create2_opts)?
                {
                    Ok(ContractCreationOpts::Proxied(proxy_opts))
                } else {
                    Ok(ContractCreationOpts::Create2(create2_opts))
                }
            }
            _ => Err(format!("invalid 'create_opcode' field: {}", create_opcode.unwrap())),
        }
    }

    pub async fn get_deployment_transaction(
        &self,
        rpc: &EvmRpc,
        sender_address: &Value,
        nonce: u64,
        chain_id: u64,
        amount: u64,
        gas_limit: Option<u64>,
        tx_type: &TransactionType,
        values: &ValueStore,
        abi: &Option<JsonAbi>,
    ) -> Result<ContractDeploymentTransaction, String> {
        match self {
            ContractCreationOpts::Create(opts) => {
                opts.get_deployment_transaction(
                    rpc,
                    sender_address,
                    nonce,
                    chain_id,
                    amount,
                    gas_limit,
                    tx_type,
                    values,
                )
                .await
            }
            ContractCreationOpts::Create2(opts) => {
                opts.get_deployment_transaction(
                    rpc,
                    sender_address,
                    nonce,
                    chain_id,
                    amount,
                    gas_limit,
                    tx_type,
                    values,
                )
                .await
            }
            ContractCreationOpts::Proxied(opts) => {
                opts.get_deployment_via_factory_transaction(
                    rpc,
                    sender_address,
                    nonce,
                    chain_id,
                    amount,
                    gas_limit,
                    tx_type,
                    values,
                    abi,
                )
                .await
            }
        }
    }

    pub async fn validate(&self, rpc: &EvmRpc) -> Result<(), String> {
        match self {
            ContractCreationOpts::Create2(opts) => opts.validate_create2_factory_address(rpc).await,
            ContractCreationOpts::Proxied(opts) => opts.validate(rpc).await,
            _ => Ok(()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CreateDeploymentOpts {
    init_code: Vec<u8>,
}
impl CreateDeploymentOpts {
    pub fn new(init_code: &Vec<u8>) -> Self {
        Self { init_code: init_code.clone() }
    }

    pub async fn get_deployment_transaction(
        &self,
        rpc: &EvmRpc,
        sender_address: &Value,
        nonce: u64,
        chain_id: u64,
        amount: u64,
        gas_limit: Option<u64>,
        tx_type: &TransactionType,
        values: &ValueStore,
    ) -> Result<ContractDeploymentTransaction, String> {
        let common = CommonTransactionFields {
            to: None,
            from: sender_address.clone(),
            nonce: Some(nonce),
            chain_id,
            amount,
            gas_limit,
            tx_type: tx_type.clone(),
            input: None,
            deploy_code: Some(self.init_code.clone()),
        };

        let (tx, tx_cost, _) = build_unsigned_transaction(rpc.clone(), values, common).await?;
        let sender_address = get_expected_address(sender_address)?;
        let expected_address = self.calculate_deployed_contract_address(&sender_address, nonce)?;

        Ok(ContractDeploymentTransaction::Create(
            ContractDeploymentTransactionStatus::NotYetDeployed(TransactionDeploymentRequestData {
                tx,
                tx_cost,
                expected_address,
            }),
        ))
    }

    pub fn calculate_deployed_contract_address(
        &self,
        sender_address: &Address,
        nonce: u64,
    ) -> Result<Address, String> {
        generate_create_address(&sender_address, nonce)
    }
}

#[derive(Clone, Debug)]
pub struct Create2DeploymentOpts {
    pub salt: String,
    pub raw_salt: Vec<u8>,
    pub factory: Create2Factory,
    pub init_code: Vec<u8>,
}
impl Create2DeploymentOpts {
    pub fn default(init_code: &Vec<u8>) -> Self {
        let salt = DEFAULT_CREATE2_SALT.to_string();
        let raw_salt = salt_str_to_hex(&salt).unwrap();
        Self { salt, raw_salt, factory: Create2Factory::Default, init_code: init_code.clone() }
    }

    pub fn default_proxied() -> Self {
        let salt = DEFAULT_CREATE2_SALT.to_string();
        let raw_salt = salt_str_to_hex(&salt).unwrap();
        Self { salt, raw_salt, factory: Create2Factory::Proxied, init_code: vec![] }
    }

    pub fn new_proxied(
        values: &Box<Vec<Value>>,
        default_values: &ValueMap,
    ) -> Result<Self, String> {
        if values.len() != 1 {
            return Err(format!("'create2' field can only be specified once"));
        }
        let values =
            values.first().unwrap().as_object().ok_or("'create2' field must be an object")?;

        let values = ValueStore::tmp()
            .with_inputs(&ValueMap::new().with_store(values))
            .with_defaults(default_values);

        let salt = values
            .get_value(SALT)
            .map(|s| s.to_string())
            .unwrap_or(DEFAULT_CREATE2_SALT.to_string());

        let raw_salt = salt_str_to_hex(&salt)?;
        if values.get_value(FACTORY_ADDRESS).is_some() {
            return Err(format!(
                "invalid 'create2' field: 'factory_address' field is not allowed for proxied contracts"
            ));
        } else {
            Ok(Self { salt, raw_salt, factory: Create2Factory::Proxied, init_code: vec![] })
        }
    }

    pub fn new(
        values: &Box<Vec<Value>>,
        default_values: &ValueMap,
        init_code: &Vec<u8>,
    ) -> Result<Self, String> {
        if values.len() != 1 {
            return Err(format!("Create2 options must contain exactly one entry"));
        }
        let values = values
            .first()
            .unwrap()
            .as_object()
            .ok_or(format!("Create2 contract options must be an object"))?;

        let values = ValueStore::tmp()
            .with_inputs(&ValueMap::new().with_store(values))
            .with_defaults(default_values);

        let salt = values
            .get_value(SALT)
            .map(|s| s.to_string())
            .unwrap_or(DEFAULT_CREATE2_SALT.to_string());

        let raw_salt = salt_str_to_hex(&salt)?;
        if let Some(custom_create2_factory_address) =
            values.get_value(FACTORY_ADDRESS).and_then(|v| Some(v.clone()))
        {
            let custom_create2_factory_address =
                get_expected_address(&custom_create2_factory_address)?;
            let create2_factory_abi = values.get_string(FACTORY_ABI).map(|v| v.to_string());
            let create2_factory_function_name = values
                .get_expected_string(FACTORY_FUNCTION_NAME)
                .map_err(|e| e.message)?
                .to_string();

            let function_args: Vec<DynSolValue> = vec![
                DynSolValue::FixedBytes(Word::from_slice(&raw_salt), 32),
                DynSolValue::Bytes(init_code.clone()),
            ];

            Ok(Self {
                salt,
                raw_salt,
                factory: Create2Factory::Custom {
                    address: custom_create2_factory_address,
                    abi: create2_factory_abi,
                    function_name: create2_factory_function_name,
                    function_args,
                },
                init_code: init_code.clone(),
            })
        } else {
            Ok(Self {
                salt: salt.to_string(),
                raw_salt,
                factory: Create2Factory::Default,
                init_code: init_code.clone(),
            })
        }
    }

    pub async fn get_deployment_transaction(
        &self,
        rpc: &EvmRpc,
        sender_address: &Value,
        nonce: u64,
        chain_id: u64,
        amount: u64,
        gas_limit: Option<u64>,
        tx_type: &TransactionType,
        values: &ValueStore,
    ) -> Result<ContractDeploymentTransaction, String> {
        let calculated_deployed_address = self.calculate_deployed_contract_address()?;
        let code_at_address =
            rpc.get_code(&calculated_deployed_address).await.map_err(|e| e.to_string())?;

        if !code_at_address.is_empty() {
            return Ok(ContractDeploymentTransaction::Create2(
                ContractDeploymentTransactionStatus::AlreadyDeployed(calculated_deployed_address),
            ));
        }

        let common = CommonTransactionFields {
            to: Some(Value::string(self.get_factory_address())),
            from: sender_address.clone(),
            nonce: Some(nonce),
            chain_id,
            amount,
            gas_limit,
            tx_type: tx_type.clone(),
            input: Some(self.get_deployment_transaction_input()?),
            deploy_code: None,
        };

        let (tx, tx_cost, _) = build_unsigned_transaction(rpc.clone(), values, common).await?;
        let expected_address = self.calculate_deployed_contract_address()?;

        Ok(ContractDeploymentTransaction::Create2(
            ContractDeploymentTransactionStatus::NotYetDeployed(TransactionDeploymentRequestData {
                tx,
                tx_cost,
                expected_address,
            }),
        ))
    }

    fn get_deployment_transaction_input(&self) -> Result<Vec<u8>, String> {
        match &self.factory {
            Create2Factory::Default => {
                let mut input = Vec::with_capacity(self.raw_salt.len() + self.init_code.len());
                input.extend_from_slice(&self.raw_salt[..]);
                input.extend_from_slice(&self.init_code[..]);
                Ok(input)
            }
            Create2Factory::Custom { abi, function_name, function_args, .. } => match abi {
                Some(abi) => {
                    encode_contract_call_inputs_from_abi_str(abi, function_name, function_args)
                }
                None => encode_contract_call_inputs_from_selector(function_name, function_args),
            },
            Create2Factory::Proxied => unreachable!(),
        }
    }

    pub fn get_factory_address(&self) -> String {
        match &self.factory {
            Create2Factory::Default => DEFAULT_CREATE2_FACTORY_ADDRESS.to_string(),
            Create2Factory::Custom { address, .. } => address.to_string(),
            Create2Factory::Proxied => PROXY_FACTORY_ADDRESS.to_string(),
        }
    }

    pub fn calculate_deployed_contract_address(&self) -> Result<Address, String> {
        let create2_factory_address = self.get_factory_address();
        generate_create2_address(
            &Value::string(create2_factory_address),
            &self.salt,
            &self.init_code,
        )
    }

    pub async fn validate_create2_factory_address(&self, rpc: &EvmRpc) -> Result<(), String> {
        let address = self.get_factory_address();

        validate_create2_factory_address(rpc, &Value::string(address)).await
    }
}

#[derive(Clone, Debug)]
pub enum Create2Factory {
    Default,
    Custom {
        address: Address,
        abi: Option<String>,
        function_name: String,
        function_args: Vec<DynSolValue>,
    },
    Proxied,
}

pub fn salt_str_to_hex(salt: &str) -> Result<Vec<u8>, String> {
    let salt = hex::decode(salt).map_err(|e| format!("failed to decode salt: {e}"))?;
    if salt.len() != 32 {
        return Err("salt must be a 32-byte string".into());
    }
    Ok(salt)
}

pub fn generate_create2_address(
    factory_address: &Value,
    salt: &str,
    init_code: &Vec<u8>,
) -> Result<Address, String> {
    let Some(factory_address_bytes) = factory_address.try_get_buffer_bytes_result()? else {
        return Err("failed to generate create2 address: invalid create2 factory address".into());
    };
    let salt_bytes =
        salt_str_to_hex(salt).map_err(|e| format!("failed to generate create2 address: {e}"))?;

    let init_code_hash = keccak256(&init_code);
    let mut hasher = Keccak256::new();
    hasher.update(&[0xff]);
    hasher.update(factory_address_bytes);
    hasher.update(&salt_bytes);
    hasher.update(&init_code_hash);

    let result = hasher.finalize();
    let address_bytes = &result[12..32];
    Ok(Address::from_slice(&address_bytes))
}

pub struct CreateAddress {
    pub sender_address: Address,
    pub nonce: u64,
}

impl Encodable for CreateAddress {
    fn encode(&self, out: &mut dyn BufMut) {
        let enc: [&dyn Encodable; 2] = [&self.sender_address, &self.nonce];
        encode_list::<&dyn Encodable, dyn Encodable>(&enc, out);
    }
}

pub fn generate_create_address(sender_address: &Address, nonce: u64) -> Result<Address, String> {
    let create_address = CreateAddress { sender_address: sender_address.clone(), nonce };
    let mut out = Vec::new();
    create_address.encode(&mut out);
    let mut hasher = Keccak256::new();
    hasher.update(out);

    let result = hasher.finalize();
    let address_bytes = &result[12..32];
    Ok(Address::from_slice(&address_bytes))
}

async fn validate_create2_factory_address(
    rpc: &EvmRpc,
    create2_factory_address: &Value,
) -> Result<(), String> {
    let Some(create2_factory_address) = create2_factory_address.try_get_buffer_bytes_result()?
    else {
        return Err(format!(
            "invalid create2 factory address: {}",
            create2_factory_address.to_string()
        ));
    };
    let create2_factory_address = Address::from_slice(&create2_factory_address[..]);
    let factory_code = rpc.get_code(&create2_factory_address).await.map_err(|e| {
        format!("failed to validate create2 contract factory address: {}", e.to_string())
    })?;
    if factory_code.is_empty() {
        return Err(format!(
            "invalid create2 contract factory: address {} is not a contract",
            create2_factory_address.to_string()
        ));
    }
    Ok(())
}
