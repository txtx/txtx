use alloy::dyn_abi::DynSolValue;
use alloy::json_abi::JsonAbi;
use alloy::primitives::Address;
use std::collections::HashMap;
use txtx_addon_kit::indexmap::IndexMap;
use txtx_addon_kit::types::cloud_interface::CloudServiceContext;
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

use crate::codec::contract_deployment::compiled_artifacts::CompiledContractArtifacts;
use crate::codec::contract_deployment::create_opts::ContractCreationOpts;
use crate::codec::contract_deployment::{
    create_init_code, AddressAbiMap, ContractDeploymentTransaction,
    ContractDeploymentTransactionStatus,
};
use crate::codec::verify::verify_contracts;
use crate::codec::{
    get_typed_transaction_bytes, value_to_abi_constructor_args, value_to_sol_value, TransactionType,
};

use crate::constants::{
    ADDRESS_ABI_MAP, ALREADY_DEPLOYED, CONTRACT_ADDRESS, CONTRACT_CONSTRUCTOR_ARGS,
    CONTRACT_VERIFICATION_OPTS, DO_VERIFY_CONTRACT, IMPL_CONTRACT_ADDRESS, IS_PROXIED,
    PROXY_CONTRACT_ADDRESS, RPC_API_URL, TRANSACTION_TYPE, VERIFICATION_RESULTS,
};
use crate::rpc::EvmRpc;
use crate::signers::common::get_signer_nonce;
use crate::typing::{
    EvmValue, CONTRACT_METADATA, CONTRACT_VERIFICATION_OPTS_TYPE, CREATE2_OPTS, DECODED_LOG_OUTPUT,
    LINKED_LIBRARIES_TYPE, PROXIED_CONTRACT_INITIALIZER, PROXY_CONTRACT_OPTS, RAW_LOG_OUTPUT,
    VERIFICATION_RESULT_TYPE,
};

use super::call_contract::{
    encode_contract_call_inputs_from_abi, encode_contract_call_inputs_from_selector,
};
use super::check_confirmations::CheckEvmConfirmations;
use super::sign_transaction::SignEvmTransaction;

use super::{get_common_tx_params_from_args, get_expected_address, get_signer_did};
use txtx_addon_kit::constants::SignerKey;

lazy_static! {
    pub static ref DEPLOY_CONTRACT: PreCommandSpecification = {
        let mut command = define_command! {
            DeployContract => {
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
                    initializer: {
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
                        documentation: "Indicates whether the contract should be verified after deployment. The default is `true`. Set this value to `false` to prevent verification event when `verifier` args are provided.",
                        typing: Type::bool(),
                        optional: true,
                        tainting: true,
                        internal: false
                    },
                    verifier: {
                        documentation: "Specifies the verifier options for contract verifications.",
                        typing: CONTRACT_VERIFICATION_OPTS_TYPE.clone(),
                        optional: true,
                        tainting: false,
                        internal: false
                    },
                    linked_libraries: {
                        documentation: "A map of contract name to contract address to specify the linked libraries for the deployed contract.",
                        typing: LINKED_LIBRARIES_TYPE.clone(),
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
                    },
                    logs: {
                        documentation: "The logs of the transaction, decoded via any ABI provided by the contract call.",
                        typing: DECODED_LOG_OUTPUT.clone()
                    },
                    raw_logs: {
                          documentation: "The raw logs of the transaction.",
                          typing: RAW_LOG_OUTPUT.clone()
                    },
                    verification_results: {
                        documentation: "The contract verification results, if the action was configured to verify the contract.",
                        typing: Type::array(VERIFICATION_RESULT_TYPE.clone())
                    }
                ],
                example: txtx_addon_kit::indoc! {r#"
                    action "my_contract" "evm::deploy_contract" {
                        contract = evm::get_contract_from_foundry_project("MyContract")
                        signer = signer.deployer
                        create2 {
                            salt = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
                        }
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

pub struct DeployContract;
impl CommandImplementation for DeployContract {
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
        auth_context: &txtx_addon_kit::types::AuthorizationContext,
    ) -> SignerActionsFutureResult {
        use crate::{
            codec::contract_deployment::{
                ContractDeploymentTransactionStatus, ProxiedDeploymentTransaction,
                TransactionDeploymentRequestData,
            },
            constants::{
                CHAIN_ID, IMPL_CONTRACT_ADDRESS, PROXY_CONTRACT_ADDRESS, TRANSACTION_COST,
                TRANSACTION_PAYLOAD_BYTES,
            },
            typing::EvmValue,
        };

        let signer_did = get_signer_did(values).unwrap();
        let construct_did = construct_did.clone();
        let instance_name = instance_name.to_string();
        let spec = spec.clone();
        let values = values.clone();
        let supervision_context = supervision_context.clone();
        let signers_instances = signers_instances.clone();
        let auth_context = auth_context.clone();

        let future = async move {
            use crate::commands::actions::get_meta_description;

            let mut actions = Actions::none();
            let mut signer_state = signers.pop_signer_state(&signer_did).unwrap();
            if let Some(_) =
                signer_state.get_scoped_value(&construct_did.value().to_string(), SignerKey::TxHash)
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
            .map_err(|e| (signers.clone(), signer_state.clone(), diagnosed_error!("{}", e)))?;

            let impl_deploy_tx = deployer
                .get_implementation_deployment_transaction(&values)
                .await
                .map_err(|e| (signers.clone(), signer_state.clone(), diagnosed_error!("{}", e)))?;

            let meta_description = deployer
                .description(&impl_deploy_tx)
                .map(|d| get_meta_description(d, &signer_did, &signers_instances));

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
                            (signers.clone(), signer_state.clone(), diagnosed_error!("{}", e))
                        })?;
                        let payload = EvmValue::transaction(bytes);
                        payload
                    }
                },
                ContractDeploymentTransaction::Proxied(ProxiedDeploymentTransaction {
                    tx,
                    tx_cost,
                    expected_impl_address,
                    expected_proxy_address,
                }) => {
                    signer_state.insert_scoped_value(
                        &construct_did.to_string(),
                        IS_PROXIED,
                        Value::bool(true),
                    );
                    signer_state.insert_scoped_value(
                        &construct_did.to_string(),
                        CONTRACT_ADDRESS,
                        EvmValue::address(&expected_proxy_address),
                    );
                    signer_state.insert_scoped_value(
                        &construct_did.to_string(),
                        IMPL_CONTRACT_ADDRESS,
                        EvmValue::address(&expected_impl_address),
                    );
                    signer_state.insert_scoped_value(
                        &construct_did.to_string(),
                        PROXY_CONTRACT_ADDRESS,
                        EvmValue::address(&expected_proxy_address),
                    );
                    signer_state.insert_scoped_value(
                        &construct_did.to_string(),
                        TRANSACTION_COST,
                        Value::integer(tx_cost),
                    );
                    let bytes = get_typed_transaction_bytes(&tx).map_err(|e| {
                        (signers.clone(), signer_state.clone(), diagnosed_error!("{}", e))
                    })?;
                    let payload = EvmValue::transaction(bytes);
                    payload
                }
            };

            let mut values = values.clone();
            values.insert(TRANSACTION_PAYLOAD_BYTES, payload);
            if let Some(meta_description) = meta_description {
                use txtx_addon_kit::constants::DocumentationKey;
                values.insert(DocumentationKey::MetaDescription, Value::string(meta_description));
            }
            signers.push_signer_state(signer_state);

            let future_result = SignEvmTransaction::check_signed_executability(
                &construct_did,
                &instance_name,
                &spec,
                &values,
                &supervision_context,
                &signers_instances,
                signers,
                &auth_context,
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
        auth_context: &txtx_addon_kit::types::AuthorizationContext,
    ) -> SignerSignFutureResult {
        let mut values = values.clone();
        let signers_instances = signers_instances.clone();
        let construct_did = construct_did.clone();
        let spec = spec.clone();
        let progress_tx = progress_tx.clone();
        let mut signers = signers.clone();
        let auth_context = auth_context.clone();

        let mut result: CommandExecutionResult = CommandExecutionResult::new();
        let signer_did = get_signer_did(&values).unwrap();
        let signer_state = signers.clone().pop_signer_state(&signer_did).unwrap();

        let already_deployed = signer_state
            .get_scoped_bool(&construct_did.to_string(), ALREADY_DEPLOYED)
            .unwrap_or(false);

        let is_proxied =
            signer_state.get_scoped_bool(&construct_did.to_string(), IS_PROXIED).unwrap_or(false);
        // insert pre-calculated contract address into outputs to be used by verify contract bg task
        let contract_address =
            signer_state.get_scoped_value(&construct_did.to_string(), CONTRACT_ADDRESS).unwrap();
        result.outputs.insert(CONTRACT_ADDRESS.to_string(), contract_address.clone());
        let contract = values.get_expected_object("contract").unwrap();
        let contract_abi = contract.get("abi");

        let mut address_abi_map = AddressAbiMap::new();

        // the check confirmations function can decode the receipt logs if it has the available contract abi's,
        // so we can index the contract addresses with their abi's here
        if is_proxied {
            let impl_contract_value = signer_state
                .get_scoped_value(&construct_did.to_string(), IMPL_CONTRACT_ADDRESS)
                .unwrap();
            let proxy_contract_value = signer_state
                .get_scoped_value(&construct_did.to_string(), PROXY_CONTRACT_ADDRESS)
                .unwrap();
            let impl_contract_address = get_expected_address(impl_contract_value).unwrap();
            let proxy_contract_address = get_expected_address(proxy_contract_value).unwrap();

            result.outputs.insert(IMPL_CONTRACT_ADDRESS.to_string(), impl_contract_value.clone());

            result.outputs.insert(PROXY_CONTRACT_ADDRESS.to_string(), proxy_contract_value.clone());

            address_abi_map.insert_opt(&impl_contract_address, &contract_abi);
            address_abi_map.insert_proxy_abis(&proxy_contract_address, &contract_abi);
            address_abi_map.insert_proxy_factory_abi();
        } else {
            address_abi_map
                .insert_opt(&get_expected_address(contract_address).unwrap(), &contract_abi);
        }

        result.outputs.insert(ADDRESS_ABI_MAP.to_string(), address_abi_map.to_value());

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
                    &auth_context,
                );
                let (signers, signer_state, mut res_signing) = match run_signing_future {
                    Ok(future) => match future.await {
                        Ok(res) => res,
                        Err(err) => return Err(err),
                    },
                    Err(err) => return Err(err),
                };

                result.append(&mut res_signing);
                values.insert(SignerKey::TxHash, result.outputs.get(SignerKey::TxHash.as_ref()).unwrap().clone());

                (signers, signer_state)
            } else {
                (signers.clone(), signer_state.clone())
            };

            values.insert(ALREADY_DEPLOYED, Value::bool(already_deployed)); // todo: delete?

            let mut res = match CheckEvmConfirmations::run_execution(
                &construct_did,
                &spec,
                &values,
                &progress_tx,
                &auth_context,
            ) {
                Ok(future) => match future.await {
                    Ok(res) => res,
                    Err(diag) => return Err((signers, signer_state, diag)),
                },
                Err(data) => return Err((signers, signer_state, data)),
            };
            result.append(&mut res);

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
        cloud_service_context: &Option<CloudServiceContext>,
    ) -> CommandExecutionFutureResult {
        let construct_did = construct_did.clone();
        let spec = spec.clone();
        let mut inputs = inputs.clone();
        let outputs = outputs.clone();
        let progress_tx = progress_tx.clone();
        let background_tasks_uuid = background_tasks_uuid.clone();
        let supervision_context = supervision_context.clone();
        let cloud_service_context = cloud_service_context.clone();

        let future = async move {
            let mut result = CommandExecutionResult::new();

            // If the deployment is done through create2, it could have already been deployed,
            // which means we don't have a tx_hash
            if let Some(tx_hash) = inputs.get_value(SignerKey::TxHash) {
                result.insert(SignerKey::TxHash.as_ref(), tx_hash.clone());
            };

            if let Some(impl_contract_address) = outputs.get_value(IMPL_CONTRACT_ADDRESS) {
                result.insert(IMPL_CONTRACT_ADDRESS, impl_contract_address.clone());
            }
            if let Some(proxy_contract_address) = outputs.get_value(PROXY_CONTRACT_ADDRESS) {
                result.insert(PROXY_CONTRACT_ADDRESS, proxy_contract_address.clone());
            }
            let mut res = CheckEvmConfirmations::build_background_task(
                &construct_did,
                &spec,
                &inputs,
                &outputs,
                &progress_tx,
                &background_tasks_uuid,
                &supervision_context,
                &cloud_service_context,
            )?
            .await?;

            result.append(&mut res);

            let do_verify = inputs.get_bool(DO_VERIFY_CONTRACT).unwrap_or(true);
            let has_opts = inputs.get_value(CONTRACT_VERIFICATION_OPTS).is_some();
            if do_verify && has_opts {
                // insert pre-calculated contract address into outputs to be used by verify contract bg task
                if let Some(contract_address) = result.outputs.get(CONTRACT_ADDRESS) {
                    inputs.insert(CONTRACT_ADDRESS, contract_address.clone());
                }

                let verification_result =
                    verify_contracts(&construct_did, &inputs, &progress_tx, &background_tasks_uuid)
                        .await
                        .map_err(|e| {
                            diagnosed_error!("failed to perform all contract verifications: {}", e)
                        })?;
                result.insert(VERIFICATION_RESULTS, verification_result);
            } else {
                result.insert(VERIFICATION_RESULTS, Value::null());
            }
            Ok(result)
        };
        Ok(Box::pin(future))
    }
}

pub struct ContractDeploymentTransactionRequestBuilder {
    rpc: EvmRpc,
    chain_id: u64,
    from_address: Address,
    amount: u64,
    gas_limit: Option<u64>,
    signer_starting_nonce: u64,
    tx_type: TransactionType,
    contract_creation_opts: ContractCreationOpts,
    abi: Option<JsonAbi>,
    contract_name: Option<String>,
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

        let is_proxy_contract =
            values.get_bool("proxied").unwrap_or(false) || values.get_value("proxy").is_some();

        let compiled_contract_artifacts = CompiledContractArtifacts::from_map(
            &values.get_expected_object("contract").map_err(|e| e.to_string())?,
        )
        .map_err(|d| d.to_string())?;

        let contract_name = compiled_contract_artifacts.contract_name.clone();

        let constructor_args = if let Some(function_args) =
            values.get_value(CONTRACT_CONSTRUCTOR_ARGS)
        {
            if is_proxy_contract {
                return Err(format!(
                    "invalid arguments: constructor arguments provided, but contract is a proxy contract"
                ));
            }
            if let Some(abi) = &compiled_contract_artifacts.abi {
                if let Some(constructor) = &abi.constructor {
                    Some(
                        value_to_abi_constructor_args(&function_args, &constructor)
                            .map_err(|e| e.message)?,
                    )
                } else {
                    return Err(format!(
                        "constructor args provided, but no constructor found in abi"
                    ));
                }
            } else {
                let sol_args = function_args
                    .expect_array()
                    .iter()
                    .map(|v| value_to_sol_value(&v))
                    .collect::<Result<Vec<DynSolValue>, String>>()?;
                Some(sol_args)
            }
        } else {
            None
        };

        let linked_libraries = EvmValue::parse_linked_libraries(values)?;

        let init_code = create_init_code(
            compiled_contract_artifacts.bytecode,
            constructor_args,
            &compiled_contract_artifacts.abi,
            linked_libraries,
        )?;

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

        contract_creation_opts.validate(&rpc).await?;

        Ok(Self {
            rpc,
            chain_id,
            from_address: from_address.clone(),
            amount,
            gas_limit,
            signer_starting_nonce,
            tx_type,
            contract_creation_opts,
            abi: compiled_contract_artifacts.abi.clone(),
            contract_name,
        })
    }

    fn description(&self, deployment_tx: &ContractDeploymentTransaction) -> Option<String> {
        match deployment_tx {
            ContractDeploymentTransaction::Create2(status) => match status {
                ContractDeploymentTransactionStatus::AlreadyDeployed(_) => None,
                ContractDeploymentTransactionStatus::NotYetDeployed(data) => Some(format!(
                    "The transaction will deploy the{} contract via Create2 to the address {} .",
                    self.contract_name
                        .as_deref()
                        .map(|name| format!(" '{name}'"))
                        .unwrap_or("".into()),
                    data.expected_address
                )),
            },
            ContractDeploymentTransaction::Create(status) => match status {
                ContractDeploymentTransactionStatus::AlreadyDeployed(_) => None,
                ContractDeploymentTransactionStatus::NotYetDeployed(data) => Some(format!(
                    "The transaction will deploy the{} contract to the address {}.",
                    self.contract_name
                        .as_deref()
                        .map(|name| format!(" '{name}'"))
                        .unwrap_or("".into()),
                    data.expected_address
                )),
            },
            ContractDeploymentTransaction::Proxied(data) => {
                Some(format!(
                    "The transaction will deploy a proxy and implementation contract{}. The proxy contract will be deployed to the address {} and the implementation contract will be deployed to the address {}.",
                    self.contract_name
                        .as_deref()
                        .map(|name| format!("for the '{name}' contract"))
                        .unwrap_or("".into()),
                    data.expected_proxy_address,
                    data.expected_impl_address
                ))
            },
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
                &self.abi,
            )
            .await
    }

    /// Gets the nonce for the implementation deployment transaction.
    pub fn get_implementation_deployment_nonce(&self) -> u64 {
        self.signer_starting_nonce
    }
}

#[derive(Clone, Debug)]
pub struct ProxiedContractInitializer {
    function_name: String,
    function_args: Vec<DynSolValue>,
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
                sol_args
            } else {
                vec![]
            };
            res.push(Self { function_name: function_name.to_string(), function_args });
        }
        Ok(res)
    }

    pub fn get_fn_input_bytes(
        &self,
        initialized_contract_abi: &Option<JsonAbi>,
    ) -> Result<Vec<u8>, String> {
        if let Some(abi) = &initialized_contract_abi {
            encode_contract_call_inputs_from_abi(&abi, &self.function_name, &self.function_args)
                .map_err(|e| format!("failed to encode initializer function: {e}"))
        } else {
            encode_contract_call_inputs_from_selector(&self.function_name, &self.function_args)
                .map_err(|e| format!("failed to encode initializer function: {e}"))
        }
    }
}
