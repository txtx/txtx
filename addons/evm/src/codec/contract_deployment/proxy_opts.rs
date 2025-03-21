use alloy::dyn_abi::{DynSolValue, Word};
use alloy::json_abi::JsonAbi;
use alloy::primitives::Address;
use txtx_addon_kit::types::stores::{ValueMap, ValueStore};
use txtx_addon_kit::types::types::Value;

use crate::codec::{build_unsigned_transaction, CommonTransactionFields, TransactionType};
use crate::commands::actions::deploy_contract::ProxiedContractInitializer;

use super::create_opts::{generate_create2_address, Create2DeploymentOpts, Create2Factory};
use super::{create_init_code, ContractDeploymentTransaction};
use crate::constants::{
    ERC1967_PROXY_ABI, ERC1967_PROXY_BYTECODE, PROXY_FACTORY_ABI_INTERFACE, PROXY_FACTORY_ADDRESS,
};
use crate::rpc::EvmRpc;

#[derive(Clone, Debug)]
pub struct ProxiedCreationOpts {
    pub impl_create2_opts: Create2DeploymentOpts,
    pub proxy_create2_opts: Create2DeploymentOpts,
    pub implementation_initializers: Vec<ProxiedContractInitializer>,
}

impl ProxiedCreationOpts {
    pub fn from_value_store(
        values: &ValueStore,
        impl_create2_opts: &Create2DeploymentOpts,
    ) -> Result<Option<Self>, String> {
        // if we're proxying, the overwrite the create2 factory to be the proxy factory
        let mut impl_create2_opts = impl_create2_opts.clone();
        impl_create2_opts.factory = Create2Factory::Proxied;

        let proxy_opts = values.get_map("proxy");
        let do_proxy = values.get_bool("proxied").unwrap_or(false) || proxy_opts.is_some();
        let proxy_opts = if do_proxy {
            let initializers = if let Some(initializers) = values.get_value("initializer") {
                ProxiedContractInitializer::new(initializers)?
            } else {
                vec![]
            };
            if let Some(proxy_opts) = proxy_opts {
                let proxy_opts = ProxiedCreationOpts::new(
                    proxy_opts,
                    &values.defaults,
                    &impl_create2_opts,
                    initializers,
                )?;
                Some(proxy_opts)
            } else {
                Some(ProxiedCreationOpts::default(&impl_create2_opts, initializers))
            }
        } else {
            None
        };
        Ok(proxy_opts)
    }

    pub fn default(
        impl_create2_opts: &Create2DeploymentOpts,
        initializers: Vec<ProxiedContractInitializer>,
    ) -> Self {
        Self {
            impl_create2_opts: impl_create2_opts.clone(),
            proxy_create2_opts: Create2DeploymentOpts::default_proxied(),
            implementation_initializers: initializers,
        }
    }

    pub fn new(
        values: &Box<Vec<Value>>,
        default_values: &ValueMap,
        impl_create2_opts: &Create2DeploymentOpts,
        initializers: Vec<ProxiedContractInitializer>,
    ) -> Result<Self, String> {
        if values.len() != 1 {
            return Err(format!("'proxy' field can only be specified once",));
        }
        let values =
            values.first().unwrap().as_object().ok_or("'proxy' field must be an object")?;

        let values = ValueStore::tmp()
            .with_inputs(&ValueMap::new().with_store(values))
            .with_defaults(default_values);

        let proxy_create2_opts = match values.get_map("create2") {
            Some(create2_opts) => {
                Create2DeploymentOpts::new_proxied(&create2_opts, &default_values)
                    .map_err(|e| format!("invalid 'proxy' field: {e}"))?
            }
            None => Create2DeploymentOpts::default_proxied(),
        };

        Ok(Self {
            impl_create2_opts: impl_create2_opts.clone(),
            proxy_create2_opts,
            implementation_initializers: initializers,
        })
    }

    pub async fn validate(&self, rpc: &EvmRpc) -> Result<(), String> {
        self.impl_create2_opts.validate_create2_factory_address(rpc).await?;
        Ok(())
    }

    pub async fn get_deployment_via_factory_transaction(
        &self,
        rpc: &EvmRpc,
        sender_address: &Value,
        nonce: u64,
        chain_id: u64,
        amount: u64,
        gas_limit: Option<u64>,
        tx_type: &TransactionType,
        values: &ValueStore,
        impl_abi: &Option<JsonAbi>,
    ) -> Result<ContractDeploymentTransaction, String> {
        self.validate_proxy_factory(rpc).await?;
        let impl_init_code = self.impl_create2_opts.init_code.clone();
        let impl_salt = self.impl_create2_opts.raw_salt.clone();
        let proxy_salt = self.proxy_create2_opts.raw_salt.clone();
        let initializers = self.get_initializer_transaction_bytes(impl_abi)?;
        let function_name = "deploy";
        let function_args = [
            DynSolValue::Bytes(impl_init_code),
            DynSolValue::FixedBytes(Word::from_slice(&impl_salt), 32),
            DynSolValue::FixedBytes(Word::from_slice(&proxy_salt), 32),
            DynSolValue::Array(
                initializers
                    .into_iter()
                    .map(|b| DynSolValue::Bytes(b))
                    .collect::<Vec<DynSolValue>>(),
            ),
        ];
        let input =
            PROXY_FACTORY_ABI_INTERFACE.encode_input(&function_name, &function_args).unwrap();

        let common = CommonTransactionFields {
            to: Some(Value::string(self.get_factory_address())),
            from: sender_address.clone(),
            nonce: Some(nonce),
            chain_id,
            amount,
            gas_limit,
            tx_type: tx_type.clone(),
            input: Some(input),
            deploy_code: None,
        };

        let (tx, tx_cost, _) = build_unsigned_transaction(rpc.clone(), values, common).await?;
        let expected_proxy_address = self.calculate_deployed_proxy_contract_address()?;
        let expected_impl_address = self.calculate_deployed_impl_contract_address()?;

        Ok(ContractDeploymentTransaction::Proxied(super::ProxiedDeploymentTransaction {
            tx,
            tx_cost,
            expected_impl_address,
            expected_proxy_address,
        }))
    }

    fn get_factory_address(&self) -> String {
        self.impl_create2_opts.get_factory_address()
    }

    pub fn calculate_deployed_proxy_contract_address(&self) -> Result<Address, String> {
        let create2_factory_address = PROXY_FACTORY_ADDRESS.to_string();
        let init_code = create_init_code(
            ERC1967_PROXY_BYTECODE.clone(),
            Some(vec![
                DynSolValue::Address(self.calculate_deployed_impl_contract_address()?),
                DynSolValue::Bytes(vec![]),
            ]),
            &Some(ERC1967_PROXY_ABI.clone()),
        )
        .map_err(|e| format!("failed to calculate deployed proxy contract address: {e}"))?;
        generate_create2_address(
            &Value::string(create2_factory_address),
            &self.proxy_create2_opts.salt,
            &init_code,
        )
    }

    pub fn calculate_deployed_impl_contract_address(&self) -> Result<Address, String> {
        self.impl_create2_opts.calculate_deployed_contract_address()
    }

    async fn validate_proxy_factory(&self, rpc: &EvmRpc) -> Result<(), String> {
        let proxy_factory_code = rpc
            .get_code(&PROXY_FACTORY_ADDRESS)
            .await
            .map_err(|e| format!("failed to validate proxy factory address: {}", e.to_string()))?;
        if proxy_factory_code.is_empty() {
            return Err(format!(
                "cannot deploy contract through proxy: no code at proxy factory address ({})",
                PROXY_FACTORY_ADDRESS.to_string()
            ));
        }
        Ok(())
    }
    pub fn get_initializer_transaction_bytes(
        &self,
        impl_abi: &Option<JsonAbi>,
    ) -> Result<Vec<Vec<u8>>, String> {
        let mut tx_bytes = vec![];
        for (i, initializer) in self.implementation_initializers.iter().enumerate() {
            let initializer_bytes = initializer.get_fn_input_bytes(impl_abi).map_err(|e| {
                format!("failed to encode initializer transaction {}: {}", i + 1, e)
            })?;
            tx_bytes.push(initializer_bytes);
        }
        Ok(tx_bytes)
    }
}
