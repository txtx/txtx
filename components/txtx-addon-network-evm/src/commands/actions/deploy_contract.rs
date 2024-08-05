use alloy::dyn_abi::DynSolValue;
use alloy::json_abi::JsonAbi;
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

use crate::typing::DEPLOYMENT_ARTIFACTS_TYPE;
use crate::{codec::get_typed_transaction_bytes, typing::ETH_TRANSACTION};

use crate::codec::{value_to_sol_value, CommonTransactionFields};
use crate::constants::{
    CONTRACT_ADDRESS, CONTRACT_CONSTRUCTOR_ARGS, RPC_API_URL, SIGNED_TRANSACTION_BYTES, TX_HASH,
};
use crate::rpc::EVMRpc;

use super::check_confirmations::CheckEVMConfirmations;
use super::get_signing_construct_did;
use super::sign_transaction::SignEVMTransaction;
use super::verify_contract::VerifyEVMContract;

lazy_static! {
    pub static ref EVM_DEPLOY_CONTRACT: PreCommandSpecification = define_command! {
      EVMDeployContract => {
          name: "Sign EVM Contract Deployment Transaction",
          matcher: "deploy_contract",
          documentation: "The `evm::deploy_contract` action encodes a contract deployment transaction, signs it with the provided wallet data, and broadcasts it to the network.",
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

pub struct EVMDeployContract;
impl CommandImplementation for EVMDeployContract {
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

            let transaction =
                build_unsigned_contract_deploy(&mut signing_command_state, &spec, &args, &defaults)
                    .await
                    .map_err(|diag| (wallets.clone(), signing_command_state.clone(), diag))?;

            let bytes = get_typed_transaction_bytes(&transaction).map_err(|e| {
                (
                    wallets.clone(),
                    signing_command_state.clone(),
                    diagnosed_error!("command 'evm::deploy_contract': {e}"),
                )
            })?;

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
async fn build_unsigned_contract_deploy(
    wallet_state: &mut ValueStore,
    _spec: &CommandSpecification,
    args: &ValueStore,
    defaults: &AddonDefaults,
) -> Result<TransactionRequest, Diagnostic> {
    use crate::{
        codec::{build_unsigned_transaction, TransactionType},
        constants::{CHAIN_ID, GAS_LIMIT, NONCE, TRANSACTION_AMOUNT, TRANSACTION_TYPE},
    };

    let from = wallet_state.get_expected_value("signer_address")?.clone();

    let rpc_api_url = args.get_defaulting_string(RPC_API_URL, &defaults)?;
    let chain_id = args.get_defaulting_uint(CHAIN_ID, &defaults)?;
    let init_code = get_contract_init_code(args)
        .map_err(|e| diagnosed_error!("command 'evm::deploy_contract': {}", e))?;

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

    let rpc = EVMRpc::new(&rpc_api_url)
        .map_err(|e| diagnosed_error!("command 'evm::deploy_contract': {}", e))?;

    let common = CommonTransactionFields {
        to: None,
        from: from.clone(),
        nonce,
        chain_id,
        amount: amount,
        gas_limit,
        tx_type,
        input: None,
        deploy_code: Some(init_code),
    };
    let tx = build_unsigned_transaction(rpc, args, common)
        .await
        .map_err(|e| diagnosed_error!("command: 'evm::deploy_contract': {e}"))?;
    Ok(tx)
}

pub fn get_contract_init_code(args: &ValueStore) -> Result<Vec<u8>, String> {
    let contract = args
        .get_expected_object("contract")
        .map_err(|e| e.to_string())?;
    let constructor_args = if let Some(function_args) = args.get_value(CONTRACT_CONSTRUCTOR_ARGS) {
        let sol_args = function_args
            .expect_array()
            .iter()
            .map(|v| value_to_sol_value(&v))
            .collect::<Result<Vec<DynSolValue>, String>>()?;
        Some(sol_args)
    } else {
        None
    };

    let Some(bytecode) = contract
        .get("bytecode")
        .and_then(|code| Some(code.expect_string().to_string()))
    else {
        return Err(format!("contract missing required bytecode"));
    };
    println!("initial bytecode: {}", bytecode);
    let mut init_code = alloy::hex::decode(bytecode).map_err(|e| e.to_string())?;

    // if we have an abi available in the contract, parse it out
    let json_abi: Option<JsonAbi> = match contract.get("abi") {
        Some(abi_string) => {
            let abi = serde_json::from_str(&abi_string.expect_string())
                .map_err(|e| format!("failed to decode contract abi: {e}"))?;
            Some(abi)
        }
        None => None,
    };

    if let Some(constructor_args) = constructor_args {
        // if we have an abi, use it to validate the constructor arguments
        if let Some(json_abi) = json_abi {
            if json_abi.constructor.is_none() {
                return Err(format!(
                    "invalid arguments: constructor arguments provided, but abi has no constructor"
                ));
            }
        }
        let mut abi_encoded_args = constructor_args
            .iter()
            .flat_map(|s| s.abi_encode())
            .collect::<Vec<u8>>();
        init_code.append(&mut abi_encoded_args);
    } else {
        // if we have an abi, use it to validate whether constructor arguments are needed
        if let Some(json_abi) = json_abi {
            if json_abi.constructor.is_some() {
                return Err(format!(
                    "invalid arguments: no constructor arguments provided, but abi has constructor"
                ));
            }
        }
    };
    Ok(init_code)
}
