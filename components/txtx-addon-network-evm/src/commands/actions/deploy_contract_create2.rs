use alloy::hex;
use alloy::rpc::types::TransactionRequest;
use std::collections::HashMap;
use txtx_addon_kit::types::commands::{
    CommandExecutionFutureResult, CommandExecutionResult, CommandImplementation,
    PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::wallets::{
    WalletActionsFutureResult, WalletInstance, WalletSignFutureResult,
};
use txtx_addon_kit::types::ValueStore;
use txtx_addon_kit::types::{
    commands::CommandSpecification,
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{
    types::RunbookSupervisionContext, wallets::SigningCommandsState, ConstructDid,
};
use txtx_addon_kit::uuid::Uuid;
use txtx_addon_kit::AddonDefaults;

use crate::typing::{DEPLOYMENT_ARTIFACTS_TYPE, ETH_ADDRESS};
use crate::{codec::get_typed_transaction_bytes, typing::ETH_TRANSACTION};

use crate::codec::CommonTransactionFields;
use crate::constants::{CONTRACT_ADDRESS, RPC_API_URL, SIGNED_TRANSACTION_BYTES, TX_HASH};
use crate::rpc::EVMRpc;

use super::check_confirmations::CheckEVMConfirmations;
use super::get_signing_construct_did;
use super::sign_transaction::SignEVMTransaction;
use super::verify_contract::VerifyEVMContract;

lazy_static! {
    pub static ref EVM_DEPLOY_CONTRACT_CREATE2: PreCommandSpecification = define_command! {
      EVMDeployContractCreate2 => {
          name: "Deploy an EVM Contract Using a Create2 Proxy Contract",
          matcher: "deploy_contract_create2",
          documentation: "Coming soon",
          implements_signing_capability: true,
          implements_background_task_capability: true,
          inputs: [
            description: {
                documentation: "A description of the transaction",
                typing: Type::string(),
                optional: true,
                interpolable: true
            },
            rpc_api_url: {
              documentation: "The URL of the EVM API used to broadcast the transaction.",
              typing: Type::string(),
              optional: false,
              interpolable: true
            },
            from: {
                documentation: "A reference to a wallet construct, which will be used to sign the transaction.",
                typing: Type::string(),
                optional: false,
                interpolable: true
            },
            create2_factory_address: {
                documentation: "Coming soon",
                typing: Type::addon(ETH_ADDRESS.clone()),
                optional: true,
                interpolable: true
            },
            create2_function_name: {
                documentation: "Coming soon",
                typing: Type::string(),
                optional: true,
                interpolable: true
            },
            amount: {
                documentation: "The amount, in WEI, to send with the deployment.",
                typing: Type::uint(),
                optional: true,
                interpolable: true
            },
            type: {
                documentation: "The transaction type. Options are 'Legacy', 'EIP2930', 'EIP1559', 'EIP4844'. The default is 'EIP1559'.",
                typing: Type::string(),
                optional: true,
                interpolable: true
            },
            max_fee_per_gas: {
                documentation: "Sets the max fee per gas of an EIP1559 transaction.",
                typing: Type::uint(),
                optional: true,
                interpolable: true
            },
            max_priority_fee_per_gas: {
                documentation: "Sets the max priority fee per gas of an EIP1559 transaction.",
                typing: Type::uint(),
                optional: true,
                interpolable: true
            },
            chain_id: {
                documentation: "The chain id.",
                typing: Type::string(),
                optional: true,
                interpolable: true
            },
            nonce: {
                documentation: "The account nonce of the signer. This value will be retrieved from the network if omitted.",
                typing: Type::uint(),
                optional: true,
                interpolable: true
            },
            gas_limit: {
                documentation: "Sets the maximum amount of gas that should be used to execute this transaction.",
                typing: Type::uint(),
                optional: true,
                interpolable: true
            },
            gas_price: {
                documentation: "Sets the gas price for Legacy transactions.",
                typing: Type::uint(),
                optional: true,
                interpolable: true
            },
            contract: {
                documentation: "Coming soon",
                typing: DEPLOYMENT_ARTIFACTS_TYPE.clone(),
                optional: false,
                interpolable: true
            },
            constructor_args: {
                documentation: "Coming soon",
                typing: Type::array(Type::string()),
                optional: true,
                interpolable: true
            },
            expected_contract_address: {
              documentation: "Coming soon",
              typing: Type::string(),
              optional: true,
              interpolable: true
            },
            salt: {
              documentation: "Coming soon",
              typing: Type::string(),
              optional: true,
              interpolable: true
            },
            confirmations: {
                documentation: "Once the transaction is included on a block, the number of blocks to await before the transaction is considered successful and Runbook execution continues.",
                typing: Type::uint(),
                optional: true,
                interpolable: true
            },
            verify: {
                documentation: "",
                typing: Type::bool(),
                optional: true,
                interpolable: true
            },
            block_explorer_api_key: {
              documentation: "The URL of the block explorer used to verify the contract.",
              typing: Type::string(),
              optional: true,
              interpolable: true
            },
            depends_on: {
              documentation: "References another command's outputs, preventing this command from executing until the referenced command is successful.",
              typing: Type::string(),
              optional: true,
              interpolable: true
            }
          ],
          outputs: [
              tx_hash: {
                  documentation: "The hash of the transaction.",
                  typing: Type::string()
              }
          ],
          example: txtx_addon_kit::indoc! {r#"
          // Coming soon
      "#},
      }
    };
}

pub struct EVMDeployContractCreate2;
impl CommandImplementation for EVMDeployContractCreate2 {
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
        args: &ValueStore,
        defaults: &AddonDefaults,
        supervision_context: &RunbookSupervisionContext,
        wallets_instances: &HashMap<ConstructDid, WalletInstance>,
        mut wallets: SigningCommandsState,
    ) -> WalletActionsFutureResult {
        use crate::constants::TRANSACTION_PAYLOAD_BYTES;

        let signing_construct_did = get_signing_construct_did(args).unwrap();

        let construct_did = construct_did.clone();
        let instance_name = instance_name.to_string();
        let spec = spec.clone();
        let args = args.clone();
        let defaults = defaults.clone();
        let supervision_context = supervision_context.clone();
        let wallets_instances = wallets_instances.clone();

        let future = async move {
            let mut actions = Actions::none();
            let mut signing_command_state = wallets
                .pop_signing_command_state(&signing_construct_did)
                .unwrap();
            if let Some(_) = signing_command_state
                .get_scoped_value(&construct_did.value().to_string(), SIGNED_TRANSACTION_BYTES)
            {
                return Ok((wallets, signing_command_state, Actions::none()));
            }

            let (transaction, contract_address) = build_unsigned_create2_deployment(
                &mut signing_command_state,
                &spec,
                &args,
                &defaults,
            )
            .await
            .map_err(|diag| (wallets.clone(), signing_command_state.clone(), diag))?;

            // for the create2 contract deployment, the tx receipt won't have the contract address, so we need to store it
            signing_command_state.insert_scoped_value(
                &construct_did.to_string(),
                CONTRACT_ADDRESS,
                Value::string(contract_address),
            );

            let bytes = get_typed_transaction_bytes(&transaction).map_err(|e| {
                (
                    wallets.clone(),
                    signing_command_state.clone(),
                    diagnosed_error!("command 'evm::deploy_contract_create2': {e}"),
                )
            })?;
            println!("paylod for signer: {:?}", bytes);
            let payload = Value::buffer(bytes, ETH_TRANSACTION.clone());
            let mut args = args.clone();
            args.insert(TRANSACTION_PAYLOAD_BYTES, payload);
            wallets.push_signing_command_state(signing_command_state);

            let future_result = SignEVMTransaction::check_signed_executability(
                &construct_did,
                &instance_name,
                &spec,
                &args,
                &defaults,
                &supervision_context,
                &wallets_instances,
                wallets,
            );
            let (wallets, signing_command_state, mut signing_actions) = match future_result {
                Ok(future) => match future.await {
                    Ok(res) => res,
                    Err(err) => return Err(err),
                },
                Err(err) => return Err(err),
            };

            actions.append(&mut signing_actions);
            Ok((wallets, signing_command_state, actions))
        };
        Ok(Box::pin(future))
    }

    fn run_signed_execution(
        construct_did: &ConstructDid,
        spec: &CommandSpecification,
        args: &ValueStore,
        defaults: &AddonDefaults,
        progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
        wallets_instances: &HashMap<ConstructDid, WalletInstance>,
        wallets: SigningCommandsState,
    ) -> WalletSignFutureResult {
        let mut args = args.clone();
        let wallets_instances = wallets_instances.clone();
        let defaults = defaults.clone();
        let construct_did = construct_did.clone();
        let spec = spec.clone();
        let progress_tx = progress_tx.clone();

        let future = async move {
            let run_signing_future = SignEVMTransaction::run_signed_execution(
                &construct_did,
                &spec,
                &args,
                &defaults,
                &progress_tx,
                &wallets_instances,
                wallets,
            );
            let (wallets, signing_command_state, mut res_signing) = match run_signing_future {
                Ok(future) => match future.await {
                    Ok(res) => res,
                    Err(err) => return Err(err),
                },
                Err(err) => return Err(err),
            };

            args.insert(TX_HASH, res_signing.outputs.get(TX_HASH).unwrap().clone());
            let mut res = match CheckEVMConfirmations::run_execution(
                &construct_did,
                &spec,
                &args,
                &defaults,
                &progress_tx,
            ) {
                Ok(future) => match future.await {
                    Ok(res) => res,
                    Err(diag) => return Err((wallets, signing_command_state, diag)),
                },
                Err(data) => return Err((wallets, signing_command_state, data)),
            };

            res_signing.append(&mut res);

            let do_verify = args.get_bool("verify").unwrap_or(false);
            if do_verify {
                let mut res = match VerifyEVMContract::run_execution(
                    &construct_did,
                    &spec,
                    &args,
                    &defaults,
                    &progress_tx,
                ) {
                    Ok(future) => match future.await {
                        Ok(res) => res,
                        Err(diag) => return Err((wallets, signing_command_state, diag)),
                    },
                    Err(data) => return Err((wallets, signing_command_state, data)),
                };

                // insert pre-calculated contract address into outputs to be used by verify contract bg task
                let contract_address = signing_command_state
                    .get_scoped_value(&construct_did.to_string(), CONTRACT_ADDRESS)
                    .unwrap();
                res_signing
                    .outputs
                    .insert(CONTRACT_ADDRESS.to_string(), contract_address.clone());

                res_signing.append(&mut res);
            }

            Ok((wallets, signing_command_state, res_signing))
        };

        Ok(Box::pin(future))
    }

    fn build_background_task(
        construct_did: &ConstructDid,
        spec: &CommandSpecification,
        inputs: &ValueStore,
        outputs: &ValueStore,
        defaults: &AddonDefaults,
        progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
        background_tasks_uuid: &Uuid,
        supervision_context: &RunbookSupervisionContext,
    ) -> CommandExecutionFutureResult {
        let construct_did = construct_did.clone();
        let spec = spec.clone();
        let mut inputs = inputs.clone();
        let outputs = outputs.clone();
        let defaults = defaults.clone();
        let progress_tx = progress_tx.clone();
        let background_tasks_uuid = background_tasks_uuid.clone();
        let supervision_context = supervision_context.clone();

        let future = async move {
            let mut result = CommandExecutionResult::new();
            let mut res = CheckEVMConfirmations::build_background_task(
                &construct_did,
                &spec,
                &inputs,
                &outputs,
                &defaults,
                &progress_tx,
                &background_tasks_uuid,
                &supervision_context,
            )?
            .await?;

            result.append(&mut res);

            let do_verify = inputs.get_bool("verify").unwrap_or(false);
            if do_verify {
                let contract_artifacts = inputs.get_expected_value("contract")?;
                inputs.insert("artifacts", contract_artifacts.clone());

                if let Some(contract_address) = result.outputs.get(CONTRACT_ADDRESS) {
                    inputs.insert(CONTRACT_ADDRESS, contract_address.clone());
                }

                let mut res = VerifyEVMContract::build_background_task(
                    &construct_did,
                    &spec,
                    &inputs,
                    &outputs,
                    &defaults,
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

#[cfg(not(feature = "wasm"))]
async fn build_unsigned_create2_deployment(
    wallet_state: &ValueStore,
    _spec: &CommandSpecification,
    args: &ValueStore,
    defaults: &AddonDefaults,
) -> Result<(TransactionRequest, String), Diagnostic> {
    use alloy::dyn_abi::DynSolValue;

    use crate::{
        codec::{build_unsigned_transaction, value_to_sol_value, TransactionType},
        commands::actions::{
            deploy_contract::get_contract_init_code,
            sign_contract_call::{
                encode_contract_call_inputs_from_abi, encode_contract_call_inputs_from_selector,
            },
        },
        constants::{
            CHAIN_ID, CONTRACT_FUNCTION_ARGS, CREATE2_FACTORY_ABI, CREATE2_FACTORY_ADDRESS,
            CREATE2_FUNCTION_NAME, DEFAULT_CREATE2_FACTORY_ADDRESS, EXPECTED_CONTRACT_ADDRESS,
            GAS_LIMIT, NONCE, TRANSACTION_AMOUNT, TRANSACTION_TYPE,
        },
    };

    let from = wallet_state.get_expected_value("signer_address")?;

    // let network_id = args.get_defaulting_string(NETWORK_ID, defaults)?;
    let rpc_api_url = args.get_defaulting_string(RPC_API_URL, &defaults)?;
    let chain_id = args.get_defaulting_uint(CHAIN_ID, &defaults)?;

    let contract_address = args
        .get_value(CREATE2_FACTORY_ADDRESS)
        .and_then(|v| Some(v.clone()));
    let init_code = get_contract_init_code(args)
        .map_err(|e| diagnosed_error!("command 'evm::deploy_contract_create2': {}", e))?;
    let expected_contract_address = args.get_string(EXPECTED_CONTRACT_ADDRESS);

    let amount = args
        .get_value(TRANSACTION_AMOUNT)
        .map(|v| v.expect_uint())
        .unwrap_or(0);
    let gas_limit = args.get_value(GAS_LIMIT).map(|v| v.expect_uint());
    let mut nonce = args.get_value(NONCE).map(|v| v.expect_uint());
    if nonce.is_none() {
        if let Some(wallet_nonce) = wallet_state.get_value(NONCE).map(|v| v.expect_uint()) {
            nonce = Some(wallet_nonce + 1);
        }
    }
    let tx_type = TransactionType::from_some_value(args.get_string(TRANSACTION_TYPE))?;

    // if the user provided a contract address, they aren't using the default create2 factory
    let input = if contract_address.is_some() {
        let contract_abi = args.get_string(CREATE2_FACTORY_ABI);
        let function_name = args.get_expected_string(CREATE2_FUNCTION_NAME)?;
        let function_args: Vec<DynSolValue> = args
            .get_expected_array(CONTRACT_FUNCTION_ARGS)
            .map(|v| {
                v.iter()
                    .map(|v| {
                        value_to_sol_value(&v).map_err(|e| {
                            diagnosed_error!("command 'evm::sign_contract_call': {}", e)
                        })
                    })
                    .collect::<Result<Vec<DynSolValue>, Diagnostic>>()
            })
            .unwrap_or(Ok(vec![]))?;

        if let Some(abi_str) = contract_abi {
            encode_contract_call_inputs_from_abi(abi_str, function_name, &function_args)
                .map_err(|e| diagnosed_error!("command 'deploy_contract_create2': {e}"))?
        } else {
            encode_contract_call_inputs_from_selector(function_name, &function_args)
                .map_err(|e| diagnosed_error!("command 'deploy_contract_create2': {e}"))?
        }
    } else {
        let salt = args.get_expected_string("salt")?;
        encode_default_create2_proxy_args(salt, &init_code)
            .map_err(|e| diagnosed_error!("command 'deploy_contract_create2': {e}"))?
    };

    let rpc = EVMRpc::new(&rpc_api_url)
        .map_err(|e| diagnosed_error!("command 'evm::deploy_contract_create2': {}", e))?;

    let contract_address = contract_address
        .and_then(|a| Some(a.clone()))
        .unwrap_or(Value::string(DEFAULT_CREATE2_FACTORY_ADDRESS.to_string()));
    let common = CommonTransactionFields {
        to: Some(contract_address),
        from: from.clone(),
        nonce,
        chain_id,
        amount: amount,
        gas_limit,
        tx_type,
        input: Some(input),
        deploy_code: None,
    };

    let tx = build_unsigned_transaction(rpc.clone(), args, common)
        .await
        .map_err(|e| diagnosed_error!("command: 'evm::deploy_contract_create2': {e}"))?;

    let actual_contract_address = rpc.call(&tx).await.map_err(|e| {
        diagnosed_error!(
            "command 'evm::deploy_contract_create2': failed to simulate deployment: {}",
            e
        )
    })?;
    if let Some(expected_contract_address) = expected_contract_address {
        if !actual_contract_address.eq(expected_contract_address) {
            return Err(diagnosed_error!("command 'evm::deploy_contract_create2': contract deployment does not yield expected address: actual ({}), expected ({})", actual_contract_address, expected_contract_address));
        }
    }
    Ok((tx, actual_contract_address))
}

fn encode_default_create2_proxy_args(salt: &str, init_code: &Vec<u8>) -> Result<Vec<u8>, String> {
    let salt = hex::decode(salt).map_err(|e| format!("failed to decode salt: {e}"))?;
    if salt.len() != 32 {
        return Err("salt must be a 32-byte string".into());
    }
    let mut data = Vec::with_capacity(salt.len() + init_code.len());
    data.extend_from_slice(&salt[..]);
    data.extend_from_slice(&init_code[..]);
    Ok(data)
}
