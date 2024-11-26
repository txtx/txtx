use alloy::dyn_abi::DynSolValue;
use alloy::primitives::Address;
use alloy::rpc::types::TransactionRequest;
use std::collections::HashMap;
use txtx_addon_kit::indexmap::IndexMap;
use txtx_addon_kit::types::commands::{
    CommandExecutionFutureResult, CommandExecutionResult, CommandImplementation,
    PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::signers::{
    SignerActionsFutureResult, SignerInstance, SignerSignFutureResult,
};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::{
    commands::CommandSpecification,
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{
    signers::SignersState, types::RunbookSupervisionContext, ConstructDid,
};
use txtx_addon_kit::uuid::Uuid;

use crate::codec::contract_deployment::create_opts::ContractCreationOpts;
use crate::codec::contract_deployment::proxy_opts::ProxyContractOpts;
use crate::codec::contract_deployment::{get_contract_init_code, ContractDeploymentTransaction};
use crate::codec::{
    get_typed_transaction_bytes, string_to_address, value_to_sol_value, TransactionType,
};

use crate::constants::{
    ALREADY_DEPLOYED, ARTIFACTS, CONTRACT, CONTRACT_ADDRESS, DO_VERIFY_CONTRACT,
    EXPECTED_CONTRACT_ADDRESS, RPC_API_URL, TRANSACTION_TYPE,
};
use crate::rpc::EvmRpc;
use crate::signers::common::get_signer_nonce;
use crate::typing::{
    EvmValue, CONTRACT_METADATA, CREATE2_OPTS, PROXIED_CONTRACT_INITIALIZER, PROXY_CONTRACT_OPTS,
};

use super::check_confirmations::CheckEvmConfirmations;
use super::sign_transaction::SignEvmTransaction;
use super::verify_contract::VerifyEvmContract;
use super::{get_common_tx_params_from_args, get_expected_address, get_signer_did};
use txtx_addon_kit::constants::TX_HASH;

lazy_static! {
    pub static ref PROXY_DEPLOY_CONTRACT: PreCommandSpecification = {
        let mut command = define_command! {
            ProxyDeployContract => {
                name: "Coming soon",
                matcher: "deploy_contract",
                documentation: indoc!{r#"
                    The `evm::deploy_contract` is coming soon.
                "#},
                implements_signing_capability: true,
                implements_background_task_capability: true,
                inputs: [
                    description: {
                        documentation: "A description of the transaction",
                        typing: Type::string(),
                        optional: true,
                        tainting: false,
                        internal: false
                    },
                    rpc_api_url: {
                        documentation: "The URL of the EVM API used to broadcast the transaction.",
                        typing: Type::string(),
                        optional: true,
                        tainting: false,
                        internal: false
                    },
                    chain_id: {
                        documentation: "The chain id.",
                        typing: Type::string(),
                        optional: true,
                        tainting: true,
                        internal: false
                    },
                    signer: {
                        documentation: "A reference to a signer construct, which will be used to sign the transaction.",
                        typing: Type::string(),
                        optional: false,
                        tainting: true,
                        internal: false
                    },
                    contract: {
                        documentation: indoc!{r#"
                            The contract to deploy. At a minimum, this should be an object with a key `bytecode` and the contract bytecode.
                            The abi field can also be provided to add type checking for the constructor arguments.
                            The `evm::get_contract_from_foundry_project` and `evm::get_contract_from_hardhat_project` functions can be used to retrieve the contract object.
                        "#},
                        typing: CONTRACT_METADATA.clone(),
                        optional: false,
                        tainting: true,
                        internal: false
                    },
                    initializers: {
                        documentation: "An optional array of initializer functions + arguments to call on the contract that is deployed to the proxy contract.",
                        typing: PROXIED_CONTRACT_INITIALIZER.clone(),
                        optional: true,
                        tainting: true,
                        internal: false
                    },
                    constructor_args: {
                        documentation: "The optional constructor arguments for the deployed contract.",
                        typing: Type::array(Type::string()),
                        optional: true,
                        tainting: true,
                        internal: false
                    },
                    // create2 opts
                    create_opcode: {
                        documentation: "The create opcode to use for deployment. Options are 'create' and 'create2'. The default is 'create2'.",
                        typing: Type::string(),
                        optional: true,
                        tainting: true,
                        internal: false
                    },
                    create2: {
                        documentation: "Options for deploying the contract with the CREATE2 opcode, overwriting txtx default options.",
                        typing: CREATE2_OPTS.clone(),
                        optional: true,
                        tainting: true,
                        internal: false
                    },
                    // proxy opts
                    proxied: {
                        documentation: "Deploys the contract via a proxy contract. The default is false.",
                        typing: Type::bool(),
                        optional: true,
                        tainting: true,
                        internal: false
                    },
                    proxy: {
                        documentation: "Options for deploying the contract via a proxy contract, overwriting txtx default options.",
                        typing: PROXY_CONTRACT_OPTS.clone(),
                        optional: true,
                        tainting: true,
                        internal: false
                    },
                    // standard transaction opts
                    amount: {
                        documentation: "The amount, in WEI, to send with the deployment.",
                        typing: Type::integer(),
                        optional: true,
                        tainting: true,
                        internal: false
                    },
                    type: {
                        documentation: "The transaction type. Options are 'Legacy', 'EIP2930', 'EIP1559', 'EIP4844'. The default is 'EIP1559'.",
                        typing: Type::string(),
                        optional: true,
                        tainting: false,
                        internal: false
                    },
                    max_fee_per_gas: {
                        documentation: "Sets the max fee per gas of an EIP1559 transaction. This value will be retrieved from the network if omitted.",
                        typing: Type::integer(),
                        optional: true,
                        tainting: false,
                        internal: false
                    },
                    max_priority_fee_per_gas: {
                        documentation: "Sets the max priority fee per gas of an EIP1559 transaction. This value will be retrieved from the network if omitted.",
                        typing: Type::integer(),
                        optional: true,
                        tainting: false,
                        internal: false
                    },
                    nonce: {
                        documentation: "The account nonce of the signer. This value will be retrieved from the network if omitted.",
                        typing: Type::integer(),
                        optional: true,
                        tainting: false,
                        internal: false
                    },
                    gas_limit: {
                        documentation: "Sets the maximum amount of gas that should be used to execute this transaction. This value will be retrieved from the network if omitted.",
                        typing: Type::integer(),
                        optional: true,
                        tainting: false,
                        internal: false
                    },
                    gas_price: {
                        documentation: "Sets the gas price for Legacy transactions. This value will be retrieved from the network if omitted.",
                        typing: Type::integer(),
                        optional: true,
                        tainting: false,
                        internal: false
                    },
                    expected_contract_address: {
                        documentation: "The contract address that the deployment should yield. If the deployment does not yield this address, the action will fail. If this field is omitted, the any deployed address will be accepted.",
                        typing: Type::string(),
                        optional: true,
                        tainting: true,
                        internal: false
                    },
                    confirmations: {
                        documentation: "Once the transaction is included on a block, the number of blocks to await before the transaction is considered successful and Runbook execution continues. The default is 1.",
                        typing: Type::integer(),
                        optional: true,
                        tainting: false,
                        internal: false
                    },
                    verify: {
                        documentation: "Coming soon.",
                        typing: Type::bool(),
                        optional: true,
                        tainting: true,
                        internal: false
                    },
                    block_explorer_api_key: {
                        documentation: "Coming soon.",
                        typing: Type::string(),
                        optional: true,
                        tainting: false,
                        internal: false
                    }
                ],
                outputs: [
                    tx_hash: {
                        documentation: "The hash of the transaction.",
                        typing: Type::string()
                    },
                    abi: {
                        documentation: "The deployed contract ABI, if it was provided as a contract input.",
                        typing: Type::string()
                    },
                    contract_address: {
                        documentation: "The address of the deployed transaction.",
                        typing: Type::string()
                    }
                ],
                example: txtx_addon_kit::indoc! {r#"
                    action "my_contract" "evm::deploy_contract_create2" {
                        contract = evm::get_contract_from_foundry_project("MyContract")
                        salt = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
                        signer = signer.deployer
                    }
                "#},
            }
        };

        if let PreCommandSpecification::Atomic(ref mut spec) = command {
            spec.create_critical_output = Some("contract_address".to_string());
        }
        command
    };
}

pub struct ProxyDeployContract;
impl CommandImplementation for ProxyDeployContract {
    fn check_instantiability(
        _ctx: &CommandSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    #[cfg(not(feature = "wasm"))]
    fn check_signed_executability(
        construct_did: &ConstructDid,
        instance_name: &str,
        spec: &CommandSpecification,
        values: &ValueStore,
        supervision_context: &RunbookSupervisionContext,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        mut signers: SignersState,
    ) -> SignerActionsFutureResult {
        use txtx_addon_kit::helpers::build_diag_context_fn;

        use crate::{
            codec::contract_deployment::{
                ContractDeploymentTransactionStatus, TransactionDeploymentRequestData,
            },
            constants::{CHAIN_ID, TRANSACTION_COST, TRANSACTION_PAYLOAD_BYTES},
            typing::EvmValue,
        };

        let signer_did = get_signer_did(values).unwrap();

        let construct_did = construct_did.clone();
        let instance_name = instance_name.to_string();
        let spec = spec.clone();
        let values = values.clone();
        let supervision_context = supervision_context.clone();
        let signers_instances = signers_instances.clone();

        let to_diag_with_ctx =
            build_diag_context_fn(instance_name.to_string(), "evm::deploy_contract".to_string());

        let future = async move {
            let mut actions = Actions::none();
            let mut signer_state = signers.pop_signer_state(&signer_did).unwrap();
            if let Some(_) =
                signer_state.get_scoped_value(&construct_did.value().to_string(), TX_HASH)
            {
                return Ok((signers, signer_state, Actions::none()));
            }

            let from = signer_state
                .get_expected_value("signer_address")
                .map_err(|diag| (signers.clone(), signer_state.clone(), diag))?;

            let rpc_api_url = values
                .get_expected_string(RPC_API_URL)
                .map_err(|diag| (signers.clone(), signer_state.clone(), diag))?;
            let chain_id = values
                .get_expected_uint(CHAIN_ID)
                .map_err(|diag| (signers.clone(), signer_state.clone(), diag))?;

            let deployer = ContractDeploymentTransactionRequestBuilder::new(
                &rpc_api_url,
                chain_id,
                from,
                &signer_state,
                &values,
            )
            .await
            .map_err(|e| (signers.clone(), signer_state.clone(), to_diag_with_ctx(e)))?;

            let impl_deploy_tx = deployer
                .get_implementation_deployment_transaction(&values)
                .await
                .map_err(|e| (signers.clone(), signer_state.clone(), to_diag_with_ctx(e)))?;

            let proxy_deploy_tx = deployer
                .get_proxy_deployment_transaction(&values)
                .await
                .map_err(|e| (signers.clone(), signer_state.clone(), to_diag_with_ctx(e)))?;

            let payload = match impl_deploy_tx {
                ContractDeploymentTransaction::Create(status)
                | ContractDeploymentTransaction::Create2(status) => match status {
                    ContractDeploymentTransactionStatus::AlreadyDeployed(contract_address) => {
                        signer_state.insert_scoped_value(
                            &construct_did.to_string(),
                            ALREADY_DEPLOYED,
                            Value::bool(true),
                        );
                        signer_state.insert_scoped_value(
                            &construct_did.to_string(),
                            CONTRACT_ADDRESS,
                            EvmValue::address(&contract_address),
                        );
                        Value::null()
                    }
                    ContractDeploymentTransactionStatus::NotYetDeployed(
                        TransactionDeploymentRequestData { tx, tx_cost, expected_address },
                    ) => {
                        signer_state.insert_scoped_value(
                            &construct_did.to_string(),
                            CONTRACT_ADDRESS,
                            EvmValue::address(&expected_address),
                        );
                        signer_state.insert_scoped_value(
                            &construct_did.to_string(),
                            TRANSACTION_COST,
                            Value::integer(tx_cost),
                        );
                        let bytes = get_typed_transaction_bytes(&tx).map_err(|e| {
                            (signers.clone(), signer_state.clone(), to_diag_with_ctx(e))
                        })?;
                        let payload = EvmValue::transaction(bytes);
                        payload
                    }
                },
            };

            let mut values = values.clone();
            values.insert(TRANSACTION_PAYLOAD_BYTES, payload);
            signers.push_signer_state(signer_state);

            let future_result = SignEvmTransaction::check_signed_executability(
                &construct_did,
                &instance_name,
                &spec,
                &values,
                &supervision_context,
                &signers_instances,
                signers,
            );
            let (signers, signer_state, mut signing_actions) = match future_result {
                Ok(future) => match future.await {
                    Ok(res) => res,
                    Err(err) => return Err(err),
                },
                Err(err) => return Err(err),
            };

            actions.append(&mut signing_actions);
            Ok((signers, signer_state, actions))
        };
        Ok(Box::pin(future))
    }

    fn run_signed_execution(
        construct_did: &ConstructDid,
        spec: &CommandSpecification,
        values: &ValueStore,
        progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        signers: SignersState,
    ) -> SignerSignFutureResult {
        let mut values = values.clone();
        let signers_instances = signers_instances.clone();
        let construct_did = construct_did.clone();
        let spec = spec.clone();
        let progress_tx = progress_tx.clone();
        let mut signers = signers.clone();

        let mut result: CommandExecutionResult = CommandExecutionResult::new();
        let signer_did = get_signer_did(&values).unwrap();
        let signer_state = signers.clone().pop_signer_state(&signer_did).unwrap();
        // insert pre-calculated contract address into outputs to be used by verify contract bg task
        let contract_address =
            signer_state.get_scoped_value(&construct_did.to_string(), CONTRACT_ADDRESS).unwrap();
        result.outputs.insert(CONTRACT_ADDRESS.to_string(), contract_address.clone());

        let contract = values.get_expected_object("contract").unwrap();
        if let Some(abi) = contract.get("abi") {
            result.outputs.insert("abi".to_string(), abi.clone());
        }

        let already_deployed = signer_state
            .get_scoped_bool(&construct_did.to_string(), ALREADY_DEPLOYED)
            .unwrap_or(false);
        result.outputs.insert(ALREADY_DEPLOYED.into(), Value::bool(already_deployed));
        let future = async move {
            // if this contract has already been deployed, we'll skip signing and confirming
            let (signers, signer_state) = if !already_deployed {
                signers.push_signer_state(signer_state);
                let run_signing_future = SignEvmTransaction::run_signed_execution(
                    &construct_did,
                    &spec,
                    &values,
                    &progress_tx,
                    &signers_instances,
                    signers,
                );
                let (signers, signer_state, mut res_signing) = match run_signing_future {
                    Ok(future) => match future.await {
                        Ok(res) => res,
                        Err(err) => return Err(err),
                    },
                    Err(err) => return Err(err),
                };

                result.append(&mut res_signing);
                values.insert(TX_HASH, result.outputs.get(TX_HASH).unwrap().clone());

                (signers, signer_state)
            } else {
                (signers.clone(), signer_state.clone())
            };

            values.insert(ALREADY_DEPLOYED, Value::bool(already_deployed)); // todo: delte?
            let mut res = match CheckEvmConfirmations::run_execution(
                &construct_did,
                &spec,
                &values,
                &progress_tx,
            ) {
                Ok(future) => match future.await {
                    Ok(res) => res,
                    Err(diag) => return Err((signers, signer_state, diag)),
                },
                Err(data) => return Err((signers, signer_state, data)),
            };
            result.append(&mut res);

            let do_verify = values.get_bool(DO_VERIFY_CONTRACT).unwrap_or(false);
            if do_verify {
                let mut res = match VerifyEvmContract::run_execution(
                    &construct_did,
                    &spec,
                    &values,
                    &progress_tx,
                ) {
                    Ok(future) => match future.await {
                        Ok(res) => res,
                        Err(diag) => return Err((signers, signer_state, diag)),
                    },
                    Err(data) => return Err((signers, signer_state, data)),
                };

                result.append(&mut res);
            }

            Ok((signers, signer_state, result))
        };

        Ok(Box::pin(future))
    }

    fn build_background_task(
        construct_did: &ConstructDid,
        spec: &CommandSpecification,
        inputs: &ValueStore,
        outputs: &ValueStore,
        progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
        background_tasks_uuid: &Uuid,
        supervision_context: &RunbookSupervisionContext,
    ) -> CommandExecutionFutureResult {
        let construct_did = construct_did.clone();
        let spec = spec.clone();
        let mut inputs = inputs.clone();
        let outputs = outputs.clone();
        let progress_tx = progress_tx.clone();
        let background_tasks_uuid = background_tasks_uuid.clone();
        let supervision_context = supervision_context.clone();

        let future = async move {
            let mut result = CommandExecutionResult::new();

            let contract = inputs.get_expected_object("contract").unwrap();
            if let Some(abi) = contract.get("abi") {
                result.outputs.insert("abi".to_string(), abi.clone());
            }
            let mut res = CheckEvmConfirmations::build_background_task(
                &construct_did,
                &spec,
                &inputs,
                &outputs,
                &progress_tx,
                &background_tasks_uuid,
                &supervision_context,
            )?
            .await?;

            result.append(&mut res);

            let do_verify = inputs.get_bool(DO_VERIFY_CONTRACT).unwrap_or(false);
            if do_verify {
                let contract_artifacts = inputs.get_expected_value(CONTRACT)?;
                inputs.insert(ARTIFACTS, contract_artifacts.clone());

                // insert pre-calculated contract address into outputs to be used by verify contract bg task
                if let Some(contract_address) = result.outputs.get(CONTRACT_ADDRESS) {
                    inputs.insert(CONTRACT_ADDRESS, contract_address.clone());
                }

                let mut res = VerifyEvmContract::build_background_task(
                    &construct_did,
                    &spec,
                    &inputs,
                    &outputs,
                    &progress_tx,
                    &background_tasks_uuid,
                    &supervision_context,
                )?
                .await?;
                result.append(&mut res);
            }
            Ok(result)
        };
        Ok(Box::pin(future))
    }
}

enum Create2DeploymentResult {
    AlreadyDeployed(Address),
    NotDeployed((TransactionRequest, i128, Address)),
}

fn validate_expected_address_match(
    expected_contract_address: Option<&str>,
    calculated_deployed_contract_address: &Address,
) -> Result<(), String> {
    if let Some(expected_contract_address) = expected_contract_address {
        let expected = string_to_address(expected_contract_address.to_string())
            .map_err(|e| format!("invalid expected contract address: {e}"))?;
        if !calculated_deployed_contract_address.eq(&expected) {
            return Err(format!(
                "contract deployment does not yield expected address: actual ({}), expected ({})",
                calculated_deployed_contract_address, expected
            ));
        }
    }
    Ok(())
}
pub struct ContractDeploymentTransactionRequestBuilder {
    rpc: EvmRpc,
    chain_id: u64,
    from_address: Address,
    init_code: Vec<u8>,
    amount: u64,
    gas_limit: Option<u64>,
    signer_starting_nonce: u64,
    tx_type: TransactionType,
    contract_creation_opts: ContractCreationOpts,
    proxy_opts: Option<ProxyContractOpts>,
    expected_contract_address: Option<String>,
}

impl ContractDeploymentTransactionRequestBuilder {
    pub async fn new(
        rpc_api_url: &str,
        chain_id: u64,
        from_address: &Value,
        signer_state: &ValueStore,
        values: &ValueStore,
    ) -> Result<Self, String> {
        let rpc = EvmRpc::new(&rpc_api_url)?;
        let from_address = get_expected_address(from_address)?;

        let proxy_opts = ProxyContractOpts::from_value_store(values)?;

        let init_code = get_contract_init_code(values, proxy_opts.is_some())?;

        let (amount, gas_limit, nonce) = get_common_tx_params_from_args(values)?;
        let signer_starting_nonce = match nonce {
            Some(user_set_nonce) => user_set_nonce,
            None => {
                if let Some(signer_nonce) = get_signer_nonce(signer_state, chain_id)? {
                    signer_nonce + 1
                } else {
                    let signer_nonce = rpc
                        .get_nonce(&from_address)
                        .await
                        .map_err(|e| format!("failed to get nonce: {e}"))?;
                    signer_nonce
                }
            }
        };

        let tx_type = TransactionType::from_some_value(values.get_string(TRANSACTION_TYPE))
            .map_err(|diag| diag.message)?;

        let contract_creation_opts = ContractCreationOpts::new(values, &init_code)?;

        let expected_contract_address =
            values.get_string(EXPECTED_CONTRACT_ADDRESS).map(|v| v.to_string());

        contract_creation_opts.validate(&rpc).await?;
        if let Some(proxy_opts) = &proxy_opts {
            proxy_opts.validate(&rpc).await?;
        }

        Ok(Self {
            rpc,
            chain_id,
            from_address: from_address.clone(),
            init_code,
            amount,
            gas_limit,
            signer_starting_nonce,
            tx_type,
            contract_creation_opts,
            proxy_opts,
            expected_contract_address,
        })
    }

    pub async fn get_proxy_deployment_transaction(
        &self,
        values: &ValueStore,
    ) -> Result<Option<ContractDeploymentTransaction>, String> {
        match &self.proxy_opts {
            Some(proxy_opts) => Some(
                proxy_opts
                    .get_unsigned_proxy_deployment_transaction(
                        &self.rpc,
                        &EvmValue::address(&self.from_address),
                        self.get_proxy_deployment_nonce(),
                        self.chain_id,
                        self.amount,
                        self.gas_limit,
                        &self.tx_type,
                        values,
                    )
                    .await,
            )
            .transpose(),
            None => Ok(None),
        }
    }

    async fn get_implementation_deployment_transaction(
        &self,
        values: &ValueStore,
    ) -> Result<ContractDeploymentTransaction, String> {
        self.contract_creation_opts
            .get_deployment_transaction(
                &self.rpc,
                &EvmValue::address(&self.from_address),
                self.get_implementation_deployment_nonce(),
                self.chain_id,
                self.amount,
                self.gas_limit,
                &self.tx_type,
                values,
            )
            .await
    }

    fn calculate_deployed_contract_address(&self) -> Result<Address, String> {
        match &self.proxy_opts {
            Some(proxy_opts) => {
                proxy_opts.contract_creation_opts.calculate_deployed_contract_address(
                    &self.from_address,
                    self.get_proxy_deployment_nonce(),
                )
            }
            None => self.contract_creation_opts.calculate_deployed_contract_address(
                &self.from_address,
                self.get_implementation_deployment_nonce(),
            ),
        }
    }

    /// Gets the nonce for the proxy deployment transaction, which is the starting nonce plus 1.
    pub fn get_proxy_deployment_nonce(&self) -> u64 {
        self.signer_starting_nonce + 1
    }

    /// Gets the nonce for the implementation deployment transaction.
    pub fn get_implementation_deployment_nonce(&self) -> u64 {
        self.signer_starting_nonce
    }
}

pub struct ProxiedContractInitializer {
    function_name: String,
    function_args: Option<Vec<DynSolValue>>,
}
impl ProxiedContractInitializer {
    pub fn new(initializers: &Value) -> Result<Vec<Self>, String> {
        let initializers = initializers
            .as_map()
            .map(|values| {
                values
                    .iter()
                    .map(|v| {
                        v.as_object()
                            .ok_or(format!("proxied contract initializer must be a map type"))
                    })
                    .collect::<Result<Vec<&IndexMap<String, Value>>, String>>()
            })
            .transpose()?
            .ok_or(format!("proxied contract initializer must be a map type"))?;

        let mut res = vec![];
        for initializer in initializers {
            let function_name = initializer
                .get("function_name")
                .and_then(|v| v.as_string())
                .ok_or(format!("initializer must contain a 'function_name'"))?;
            let function_args = if let Some(function_args) = initializer.get("function_args") {
                let sol_args = function_args
                    .expect_array()
                    .iter()
                    .map(|v| value_to_sol_value(&v))
                    .collect::<Result<Vec<DynSolValue>, String>>()?;
                Some(sol_args)
            } else {
                None
            };
            res.push(Self { function_name: function_name.to_string(), function_args });
        }
        Ok(res)
    }
}
