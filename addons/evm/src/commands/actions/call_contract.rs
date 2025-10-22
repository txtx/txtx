use alloy::contract::Interface;
use alloy::dyn_abi::DynSolValue;
use alloy::hex;
use alloy::json_abi::JsonAbi;
use alloy::rpc::types::TransactionRequest;
use std::collections::HashMap;
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

use crate::codec::contract_deployment::AddressAbiMap;
use crate::codec::CommonTransactionFields;
use crate::commands::actions::check_confirmations::CheckEvmConfirmations;
use crate::commands::actions::sign_transaction::SignEvmTransaction;
use crate::constants::{
    ABI_ENCODED_RESULT, ADDRESS_ABI_MAP, CONTRACT_ABI, CONTRACT_ADDRESS, RESULT, RPC_API_URL,
    TX_HASH,
};
use crate::rpc::EvmRpc;
use crate::typing::{DECODED_LOG_OUTPUT, EVM_ADDRESS, EVM_SIM_RESULT, RAW_LOG_OUTPUT};
use txtx_addon_kit::constants::SignerKey;

use super::{get_expected_address, get_signer_did};

lazy_static! {
    pub static ref SIGN_EVM_CONTRACT_CALL: PreCommandSpecification = define_command! {
      SignEvmContractCall => {
          name: "Sign EVM Contract Call Transaction",
          matcher: "call_contract",
          documentation: "The `evm::call_contract` action encodes a contract call transaction, signs it with the provided signer data, and broadcasts it to the network.",
          implements_signing_capability: true,
          implements_background_task_capability: true,
          inputs: [
            description: {
                documentation: "A description of the transaction.",
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
            contract_address: {
                documentation: "The address of the contract being called.",
                typing: Type::addon(EVM_ADDRESS),
                optional: false,
                tainting: true,
                internal: false
            },
            contract_abi: {
                documentation: "The contract ABI, optionally used to check input arguments before sending the transaction to the chain.",
                typing: Type::addon(EVM_ADDRESS),
                optional: true,
                tainting: false,
                internal: false
            },
            function_name: {
                documentation: "The contract function to invoke.",
                typing: Type::string(),
                optional: false,
                tainting: true,
                internal: false
            },
            function_args: {
                documentation: "The contract function arguments",
                typing: Type::array(Type::buffer()),
                optional: true,
                tainting: true,
                internal: false
            },
            amount: {
                documentation: "The amount, in WEI, to transfer.",
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
            confirmations: {
                documentation: "Once the transaction is included on a block, the number of blocks to await before the transaction is considered successful and Runbook execution continues. The default is 1.",
                typing: Type::integer(),
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
              logs: {
                  documentation: "The logs of the transaction, decoded via any ABI provided by the contract call.",
                  typing: DECODED_LOG_OUTPUT.clone()
              },
              raw_logs: {
                    documentation: "The raw logs of the transaction.",
                    typing: RAW_LOG_OUTPUT.clone()
              },
              result: {
                    documentation: "The result of simulating the execution of the transaction directly before its execution.",
                    typing: Type::string()
              },
              abi_encoded_result: {
                    documentation: "The simulation result with ABI context for using in other function calls.",
                    typing: Type::addon(EVM_SIM_RESULT)
              }
          ],
          example: txtx_addon_kit::indoc! {r#"
            action "call_some_contract" "evm::call_contract" {
                contract_address = input.contract_address
                function_name = "myFunction"
                function_args = [evm::bytes("0x1234")]
                signer = signer.operator
            }
      "#},
      }
    };
}

pub struct SignEvmContractCall;
impl CommandImplementation for SignEvmContractCall {
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
            codec::get_typed_transaction_bytes,
            commands::actions::sign_transaction::SignEvmTransaction,
            constants::{ABI_ENCODED_RESULT, RESULT, TRANSACTION_COST, TRANSACTION_PAYLOAD_BYTES},
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
            use txtx_addon_kit::constants::DocumentationKey;

            use crate::commands::actions::get_meta_description;

            let mut actions = Actions::none();
            let mut signer_state = signers.pop_signer_state(&signer_did).unwrap();
            if let Some(_) =
                signer_state.get_scoped_value(&construct_did.value().to_string(), SignerKey::TxHash)
            {
                return Ok((signers, signer_state, Actions::none()));
            }
            let (
                transaction,
                transaction_cost,
                sim_result_raw,
                sim_result_with_encoding,
                meta_description,
            ) = build_unsigned_contract_call(&signer_state, &spec, &values)
                .await
                .map_err(|diag| (signers.clone(), signer_state.clone(), diag))?;

            let meta_description =
                get_meta_description(meta_description, &signer_did, &signers_instances);

            signer_state.insert_scoped_value(
                &construct_did.to_string(),
                RESULT,
                Value::String(sim_result_raw),
            );
            signer_state.insert_scoped_value(
                &construct_did.to_string(),
                ABI_ENCODED_RESULT,
                sim_result_with_encoding,
            );

            let bytes = get_typed_transaction_bytes(&transaction)
                .map_err(|e| (signers.clone(), signer_state.clone(), diagnosed_error!("{}", e)))?;

            let payload = EvmValue::transaction(bytes);

            let mut values = values.clone();
            values.insert(TRANSACTION_PAYLOAD_BYTES, payload.clone());
            values.insert(DocumentationKey::MetaDescription, Value::string(meta_description));

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
        let signers = signers.clone();
        let auth_context = auth_context.clone();

        let mut result: CommandExecutionResult = CommandExecutionResult::new();

        let future = async move {
            let contract_address =
                get_expected_address(values.get_value(CONTRACT_ADDRESS).unwrap()).unwrap();

            let signer_did = get_signer_did(&values).unwrap();
            let signer_state = signers.get_signer_state(&signer_did).unwrap();
            let call_result = signer_state
                .get_expected_scoped_value(&construct_did.to_string(), RESULT)
                .map_err(|e| (signers.clone(), signer_state.clone(), e))?;
            result.outputs.insert(RESULT.to_string(), call_result.clone());

            let encoded_result = signer_state
                .get_expected_scoped_value(&construct_did.to_string(), ABI_ENCODED_RESULT)
                .map_err(|e| (signers.clone(), signer_state.clone(), e))?;
            result.outputs.insert(ABI_ENCODED_RESULT.to_string(), encoded_result.clone());

            let contract_abi = values.get_value(CONTRACT_ABI);
            let mut address_abi_map = AddressAbiMap::new();
            address_abi_map.insert_opt(&contract_address, &contract_abi);

            result.outputs.insert(ADDRESS_ABI_MAP.to_string(), address_abi_map.to_value());

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
        let inputs = inputs.clone();
        let outputs = outputs.clone();
        let progress_tx = progress_tx.clone();
        let background_tasks_uuid = background_tasks_uuid.clone();
        let supervision_context = supervision_context.clone();
        let cloud_service_context = cloud_service_context.clone();

        let future = async move {
            let mut result = CommandExecutionResult::new();
            result.outputs.insert(SignerKey::TxHash.to_string(), inputs.get_value(SignerKey::TxHash).unwrap().clone());

            let call_result = outputs.get_expected_value(RESULT)?;
            result.outputs.insert(RESULT.to_string(), call_result.clone());

            let encoded_result = outputs.get_expected_value(ABI_ENCODED_RESULT)?;
            result.outputs.insert(ABI_ENCODED_RESULT.to_string(), encoded_result.clone());

            if let Some(contract_abi) = outputs.get_value(CONTRACT_ABI) {
                result.outputs.insert(CONTRACT_ABI.to_string(), contract_abi.clone());
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

            Ok(result)
        };
        Ok(Box::pin(future))
    }
}

#[cfg(not(feature = "wasm"))]
async fn build_unsigned_contract_call(
    signer_state: &ValueStore,
    _spec: &CommandSpecification,
    values: &ValueStore,
) -> Result<(TransactionRequest, i128, String, Value, String), Diagnostic> {
    use crate::{
        codec::{
            build_unsigned_transaction, value_to_abi_function_args, value_to_sol_value,
            TransactionType,
        },
        commands::actions::get_common_tx_params_from_args,
        constants::{
            CHAIN_ID, CONTRACT_ABI, CONTRACT_ADDRESS, CONTRACT_FUNCTION_ARGS,
            CONTRACT_FUNCTION_NAME, TRANSACTION_TYPE,
        },
        signers::common::get_signer_nonce,
        typing::EvmValue,
    };

    let from = signer_state.get_expected_value("signer_address")?;

    let rpc_api_url = values.get_expected_string(RPC_API_URL)?;
    let chain_id = values.get_expected_uint(CHAIN_ID)?;

    let contract_address_value = values.get_expected_value(CONTRACT_ADDRESS)?;
    let contract_abi = values.get_string(CONTRACT_ABI);
    let function_name = values.get_expected_string(CONTRACT_FUNCTION_NAME)?;

    let contract_address = EvmValue::to_address(contract_address_value)?;

    let function_args = if let Some(abi_str) = contract_abi {
        values
            .get_value(CONTRACT_FUNCTION_ARGS)
            .map(|v| {
                let abi: JsonAbi = serde_json::from_str(&abi_str)
                    .map_err(|e| diagnosed_error!("invalid contract abi: {}", e))?;
                value_to_abi_function_args(&function_name, &v, &abi)
            })
            .unwrap_or(Ok(vec![]))?
    } else {
        values
            .get_value(CONTRACT_FUNCTION_ARGS)
            .map(|v| {
                v.expect_array()
                    .iter()
                    .map(|v| value_to_sol_value(&v).map_err(|e| diagnosed_error!("{}", e)))
                    .collect::<Result<Vec<DynSolValue>, Diagnostic>>()
            })
            .unwrap_or(Ok(vec![]))?
    };

    let (amount, gas_limit, mut nonce) =
        get_common_tx_params_from_args(values).map_err(|e| diagnosed_error!("{}", e))?;
    if nonce.is_none() {
        if let Some(signer_nonce) =
            get_signer_nonce(signer_state, chain_id).map_err(|e| diagnosed_error!("{}", e))?
        {
            nonce = Some(signer_nonce + 1);
        }
    }

    let tx_type = TransactionType::from_some_value(values.get_string(TRANSACTION_TYPE))?;

    let rpc = EvmRpc::new(&rpc_api_url).map_err(|e| diagnosed_error!("{}", e))?;

    let input = if let Some(abi_str) = contract_abi {
        encode_contract_call_inputs_from_abi_str(abi_str, function_name, &function_args)
            .map_err(|e| diagnosed_error!("{}", e))?
    } else {
        encode_contract_call_inputs_from_selector(function_name, &function_args)
            .map_err(|e| diagnosed_error!("{}", e))?
    };

    let common = CommonTransactionFields {
        to: Some(contract_address_value.clone()),
        from: from.clone(),
        nonce,
        chain_id,
        amount,
        gas_limit,
        tx_type,
        input: Some(input),
        deploy_code: None,
    };

    let function_spec = if let Some(abi_str) = contract_abi {
        let abi: JsonAbi = serde_json::from_str(&abi_str)
            .map_err(|e| diagnosed_error!("invalid contract abi: {}", e))?;

        if let Some(function) = abi.function(&function_name).and_then(|f| f.first()) {
            if let Ok(out) = serde_json::to_vec(&function) {
                Some(out)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let (tx, cost, sim_result_raw) = build_unsigned_transaction(rpc, values, common)
        .await
        .map_err(|e| diagnosed_error!("{}", e))?;

    let sim_result_bytes = hex::decode(&sim_result_raw)
        .map_err(|e| diagnosed_error!("invalid simulation result: {}", e))?;

    let sim_result = EvmValue::sim_result(sim_result_bytes, function_spec);

    Ok((tx, cost, sim_result_raw, sim_result, format!("The transaction will call the `{}` function on the contract at address `{}` with the provided arguments.", function_name, contract_address)))
}

pub fn encode_contract_call_inputs_from_selector(
    function_name: &str,
    function_args: &Vec<DynSolValue>,
) -> Result<Vec<u8>, String> {
    let selector =
        hex::decode(function_name).map_err(|e| format!("failed to decode function_name: {e}"))?;
    if selector.len() != 4 {
        return Err(
            "function_name must be a valid 4-byte function selector if no contract abi is provided"
                .into(),
        );
    }
    let encoded_args =
        function_args.iter().flat_map(|v| v.abi_encode_params()).collect::<Vec<u8>>();
    let mut data = Vec::with_capacity(encoded_args.len() + 4);
    data.extend_from_slice(&selector[..]);
    data.extend_from_slice(&encoded_args[..]);
    Ok(data)
}

pub fn encode_contract_call_inputs_from_abi_str(
    abi_str: &str,
    function_name: &str,
    function_args: &Vec<DynSolValue>,
) -> Result<Vec<u8>, String> {
    let abi: JsonAbi =
        serde_json::from_str(&abi_str).map_err(|e| format!("invalid contract abi: {}", e))?;

    encode_contract_call_inputs_from_abi(&abi, function_name, function_args)
}

pub fn encode_contract_call_inputs_from_abi(
    abi: &JsonAbi,
    function_name: &str,
    function_args: &Vec<DynSolValue>,
) -> Result<Vec<u8>, String> {
    let interface = Interface::new(abi.clone());
    interface
        .encode_input(function_name, &function_args)
        .map_err(|e| format!("failed to encode contract inputs: {e}"))
}
