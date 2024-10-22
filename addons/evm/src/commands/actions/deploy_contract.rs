use alloy::dyn_abi::{DynSolValue, JsonAbiExt};
use alloy::json_abi::JsonAbi;
use alloy::rpc::types::TransactionRequest;
use std::collections::HashMap;
use txtx_addon_kit::types::commands::{
    CommandExecutionFutureResult, CommandExecutionResult, CommandImplementation,
    PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::signers::{
    SignerActionsFutureResult, SignerInstance, SignerSignFutureResult,
};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::{commands::CommandSpecification, diagnostics::Diagnostic, types::Type};
use txtx_addon_kit::types::{
    signers::SignersState, types::RunbookSupervisionContext, ConstructDid,
};
use txtx_addon_kit::uuid::Uuid;

use crate::codec::get_typed_transaction_bytes;

use crate::codec::{value_to_sol_value, CommonTransactionFields};
use crate::constants::{
    ARTIFACTS, CONTRACT_ADDRESS, CONTRACT_CONSTRUCTOR_ARGS, DO_VERIFY_CONTRACT, RPC_API_URL,
};
use crate::rpc::EvmRpc;
use crate::typing::CONTRACT_METADATA;
use txtx_addon_kit::constants::TX_HASH;

use super::check_confirmations::CheckEvmConfirmations;
use super::get_signer_did;
use super::sign_transaction::SignEvmTransaction;
use super::verify_contract::VerifyEvmContract;

lazy_static! {
    pub static ref EVM_DEPLOY_CONTRACT: PreCommandSpecification = define_command! {
      EvmDeployContract => {
          name: "Sign EVM Contract Deployment Transaction",
          matcher: "deploy_contract",
          documentation: "The `evm::deploy_contract` action encodes a contract deployment transaction, signs it with the provided signer data, and broadcasts it to the network.",
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
            signer: {
                documentation: "A reference to a signer construct, which will be used to sign the transaction.",
                typing: Type::string(),
                optional: false,
                tainting: true,
                internal: false
            },
            amount: {
                documentation: "The amount, in WEI, to send with the deployment.",
                typing: Type::integer(),
                optional: true,
                tainting: true,
                internal: false
            },
            type: {
                documentation: "The transaction type. Options are 'Legacy', 'EIP2930', 'EIP1559', 'EIP4844'. The default is 'EIP1559'. This value will be retrieved from the network if omitted.",
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
            chain_id: {
                documentation: "The chain id.",
                typing: Type::string(),
                optional: true,
                tainting: true,
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
            constructor_args: {
                documentation: "The optional constructor arguments for the deployed contract.",
                typing: Type::array(Type::string()),
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
          action "my_contract" "evm::deploy_contract" {
              contract = evm::get_contract_from_foundry_project("MyContract")
              signer = signer.deployer
          }
      "#},
      }
    };
}

pub struct EvmDeployContract;
impl CommandImplementation for EvmDeployContract {
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
        use txtx_addon_kit::{helpers::build_diag_context_fn, types::types::Value};

        use crate::{
            constants::{TRANSACTION_COST, TRANSACTION_PAYLOAD_BYTES},
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

            let (transaction, transaction_cost) = build_unsigned_contract_deploy(
                &mut signer_state,
                &spec,
                &values,
                &to_diag_with_ctx,
            )
            .await
            .map_err(|diag| (signers.clone(), signer_state.clone(), diag))?;

            let bytes = get_typed_transaction_bytes(&transaction).map_err(|e| {
                (
                    signers.clone(),
                    signer_state.clone(),
                    diagnosed_error!("command 'evm::deploy_contract': {e}"),
                )
            })?;

            let payload = EvmValue::transaction(bytes);
            let mut values = values.clone();
            values.insert(TRANSACTION_PAYLOAD_BYTES, payload);

            signer_state.insert_scoped_value(
                &construct_did.to_string(),
                TRANSACTION_COST,
                Value::integer(transaction_cost),
            );
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

        let mut result: CommandExecutionResult = CommandExecutionResult::new();

        let contract = values.get_expected_object("contract").unwrap();
        if let Some(abi) = contract.get("abi") {
            result.outputs.insert("abi".to_string(), abi.clone());
        }

        let future = async move {
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

            values.insert(TX_HASH, res_signing.outputs.get(TX_HASH).unwrap().clone());
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
                let contract_artifacts = inputs.get_expected_value("contract")?;
                inputs.insert(ARTIFACTS, contract_artifacts.clone());
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

#[cfg(not(feature = "wasm"))]
async fn build_unsigned_contract_deploy(
    signer_state: &mut ValueStore,
    _spec: &CommandSpecification,
    values: &ValueStore,
    to_diag_with_ctx: &impl Fn(std::string::String) -> Diagnostic,
) -> Result<(TransactionRequest, i128), Diagnostic> {
    use crate::{
        codec::{build_unsigned_transaction, TransactionType},
        commands::actions::get_common_tx_params_from_args,
        constants::{CHAIN_ID, NONCE, TRANSACTION_TYPE},
    };

    let from = signer_state.get_expected_value("signer_address")?.clone();

    let rpc_api_url = values.get_expected_string(RPC_API_URL)?;
    let chain_id = values.get_expected_uint(CHAIN_ID)?;
    let init_code = get_contract_init_code(values)
        .map_err(|e| diagnosed_error!("command 'evm::deploy_contract': {}", e))?;

    let (amount, gas_limit, mut nonce) =
        get_common_tx_params_from_args(values).map_err(to_diag_with_ctx)?;
    if nonce.is_none() {
        if let Some(signer_nonce) = signer_state
            .get_value(NONCE)
            .map(|v| v.expect_uint())
            .transpose()
            .map_err(to_diag_with_ctx)?
        {
            nonce = Some(signer_nonce + 1);
        }
    }
    let tx_type = TransactionType::from_some_value(values.get_string(TRANSACTION_TYPE))?;

    let rpc = EvmRpc::new(&rpc_api_url)
        .map_err(|e| diagnosed_error!("command 'evm::deploy_contract': {}", e))?;

    let common = CommonTransactionFields {
        to: None,
        from: from.clone(),
        nonce,
        chain_id,
        amount,
        gas_limit,
        tx_type,
        input: None,
        deploy_code: Some(init_code),
    };
    let res = build_unsigned_transaction(rpc, values, common)
        .await
        .map_err(|e| diagnosed_error!("command 'evm::deploy_contract': {e}"))?;
    Ok(res)
}

pub fn get_contract_init_code(args: &ValueStore) -> Result<Vec<u8>, String> {
    let contract = args.get_expected_object("contract").map_err(|e| e.to_string())?;
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

    let Some(bytecode) =
        contract.get("bytecode").and_then(|code| Some(code.expect_string().to_string()))
    else {
        return Err(format!("contract missing required bytecode"));
    };

    // if we have an abi available in the contract, parse it out
    let json_abi: Option<JsonAbi> = match contract.get("abi") {
        Some(abi_string) => {
            let abi = serde_json::from_str(&abi_string.expect_string())
                .map_err(|e| format!("failed to decode contract abi: {e}"))?;
            Some(abi)
        }
        None => None,
    };
    create_init_code(bytecode, constructor_args, json_abi)
}

pub fn create_init_code(
    bytecode: String,
    constructor_args: Option<Vec<DynSolValue>>,
    json_abi: Option<JsonAbi>,
) -> Result<Vec<u8>, String> {
    let mut init_code = alloy::hex::decode(bytecode).map_err(|e| e.to_string())?;
    if let Some(constructor_args) = constructor_args {
        // if we have an abi, use it to validate the constructor arguments
        let mut abi_encoded_args = if let Some(json_abi) = json_abi {
            if let Some(constructor) = json_abi.constructor {
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
            if json_abi.constructor.is_some() {
                return Err(format!(
                    "invalid arguments: no constructor arguments provided, but abi has constructor"
                ));
            }
        }
    };
    Ok(init_code)
}
