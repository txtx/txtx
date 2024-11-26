use alloy::dyn_abi::DynSolValue;
use alloy::json_abi::JsonAbi;
use txtx_addon_kit::types::stores::{ValueMap, ValueStore};
use txtx_addon_kit::types::types::Value;

use crate::codec::contract_deployment::create_init_code;
use crate::codec::contract_deployment::create_opts::ContractCreationOpts;
use crate::codec::{value_to_sol_value, TransactionType};
use crate::commands::actions::proxy_deploy_contract::ProxiedContractInitializer;

use super::ContractDeploymentTransaction;
use crate::constants::DEFAULT_PROXY_CONTRACT;
use crate::rpc::EvmRpc;

pub struct ProxyContractOpts {
    pub contract_creation_opts: ContractCreationOpts,
    init_code: Vec<u8>,
    implementation_initializers: Vec<ProxiedContractInitializer>,
    values: ValueStore,
}

impl ProxyContractOpts {
    pub fn from_value_store(values: &ValueStore) -> Result<Option<Self>, String> {
        let proxy_opts = values.get_map("proxy");
        let do_proxy = values.get_bool("proxied").unwrap_or(false) || proxy_opts.is_some();
        let proxy_opts = if do_proxy {
            if let Some(proxy_opts) = proxy_opts {
                let proxy_opts = ProxyContractOpts::new(proxy_opts, &values.defaults)?;
                Some(proxy_opts)
            } else {
                Some(ProxyContractOpts::default())
            }
        } else {
            None
        };
        Ok(proxy_opts)
    }

    pub fn default() -> Self {
        let proxy_contract = &DEFAULT_PROXY_CONTRACT;

        let bytecode =
            proxy_contract.get("bytecode").map(|code| code.expect_string().to_string()).unwrap();

        let json_abi: JsonAbi = proxy_contract
            .get("abi")
            .map(|abi_string| serde_json::from_str(&abi_string.expect_string()).unwrap())
            .unwrap();

        let init_code = create_init_code(bytecode, None, Some(json_abi)).unwrap();

        Self {
            contract_creation_opts: ContractCreationOpts::default(&init_code),
            init_code,
            implementation_initializers: vec![],
            values: ValueStore::tmp(),
        }
    }
    pub fn new(values: &Box<Vec<Value>>, default_values: &ValueMap) -> Result<Self, String> {
        if values.len() != 1 {
            return Err(format!("Proxy contract options must contain exactly one entry"));
        }
        let values = values
            .first()
            .unwrap()
            .as_object()
            .ok_or(format!("Proxy contract options must be an object"))?;

        let values = ValueStore::tmp()
            .with_inputs(&ValueMap::new().with_store(values))
            .with_defaults(default_values);

        let proxy_contract = match values.get_value("contract") {
            Some(proxy_contract) => {
                proxy_contract.as_object().ok_or(format!("contract option must be an object"))?
            }
            None => &DEFAULT_PROXY_CONTRACT, // todo, set proper default
        };

        let constructor_args = if let Some(constructor_args) = values.get_value("constructor_args")
        {
            let sol_args = constructor_args
                .expect_array()
                .iter()
                .map(|v| value_to_sol_value(&v))
                .collect::<Result<Vec<DynSolValue>, String>>()?;
            Some(sol_args)
        } else {
            None
        };

        let bytecode = proxy_contract
            .get("bytecode")
            .map(|code| code.expect_string().to_string())
            .ok_or(format!("contract missing required bytecode"))?;

        let json_abi: Option<JsonAbi> = match proxy_contract.get("abi") {
            Some(abi_string) => {
                let abi = serde_json::from_str(&abi_string.expect_string())
                    .map_err(|e| format!("failed to decode contract abi: {e}"))?;
                Some(abi)
            }
            None => None,
        };

        let init_code = create_init_code(bytecode, constructor_args, json_abi)?;

        let initializers = if let Some(initializers) = values.get_value("initializer") {
            ProxiedContractInitializer::new(initializers)?
        } else {
            vec![]
        };

        let contract_creation_opts = ContractCreationOpts::new(&values, &init_code)?;

        Ok(Self {
            contract_creation_opts,
            init_code,
            implementation_initializers: initializers,
            values,
        })
    }

    pub async fn validate(&self, rpc: &EvmRpc) -> Result<(), String> {
        self.contract_creation_opts.validate(rpc).await?;
        Ok(())
    }

    pub async fn get_unsigned_proxy_deployment_transaction(
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
        self.contract_creation_opts
            .get_deployment_transaction(
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
}
