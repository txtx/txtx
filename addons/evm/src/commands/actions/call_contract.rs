use alloy::contract::Interface;
use alloy::dyn_abi::DynSolValue;
use alloy::hex;
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
use txtx_addon_kit::types::{
    commands::CommandSpecification,
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{
    signers::SignersState, types::RunbookSupervisionContext, ConstructDid,
};
use txtx_addon_kit::uuid::Uuid;

use crate::codec::CommonTransactionFields;
use crate::commands::actions::check_confirmations::CheckEvmConfirmations;
use crate::commands::actions::sign_transaction::SignEvmTransaction;
use crate::constants::{CONTRACT_ABI, RPC_API_URL};
use crate::rpc::EvmRpc;
use crate::typing::EVM_ADDRESS;
use txtx_addon_kit::constants::TX_HASH;

use super::get_signer_did;

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
            }
          ],
          outputs: [
              tx_hash: {
                  documentation: "The hash of the transaction.",
                  typing: Type::string()
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
    ) -> SignerActionsFutureResult {
        use txtx_addon_kit::helpers::build_diag_context_fn;

        use crate::{
            codec::get_typed_transaction_bytes,
            commands::actions::sign_transaction::SignEvmTransaction,
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
            build_diag_context_fn(instance_name.to_string(), "evm::call_contract".to_string());

        let future = async move {
            let mut actions = Actions::none();
            let mut signer_state = signers.pop_signer_state(&signer_did).unwrap();
            if let Some(_) =
                signer_state.get_scoped_value(&construct_did.value().to_string(), TX_HASH)
            {
                return Ok((signers, signer_state, Actions::none()));
            }
            let (transaction, transaction_cost) =
                build_unsigned_contract_call(&signer_state, &spec, &values, &to_diag_with_ctx)
                    .await
                    .map_err(|diag| (signers.clone(), signer_state.clone(), diag))?;

            let bytes = get_typed_transaction_bytes(&transaction)
                .map_err(|e| (signers.clone(), signer_state.clone(), to_diag_with_ctx(e)))?;

            let payload = EvmValue::transaction(bytes);

            let mut values = values.clone();
            values.insert(TRANSACTION_PAYLOAD_BYTES, payload.clone());

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
        let mut signers = signers.clone();

        let mut result: CommandExecutionResult = CommandExecutionResult::new();
        let signer_did = get_signer_did(&values).unwrap();
        let signer_state = signers.clone().pop_signer_state(&signer_did).unwrap();
        let future = async move {
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
        let inputs = inputs.clone();
        let outputs = outputs.clone();
        let progress_tx = progress_tx.clone();
        let background_tasks_uuid = background_tasks_uuid.clone();
        let supervision_context = supervision_context.clone();

        let future = async move {
            let mut result = CommandExecutionResult::new();
            result.outputs.insert(TX_HASH.to_string(), inputs.get_value(TX_HASH).unwrap().clone());
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
    to_diag_with_ctx: &impl Fn(std::string::String) -> Diagnostic,
) -> Result<(TransactionRequest, i128), Diagnostic> {
    use crate::{
        codec::{build_unsigned_transaction, value_to_sol_value, TransactionType},
        commands::actions::get_common_tx_params_from_args,
        constants::{
            CHAIN_ID, CONTRACT_ABI, CONTRACT_ADDRESS, CONTRACT_FUNCTION_ARGS,
            CONTRACT_FUNCTION_NAME, TRANSACTION_TYPE,
        },
        signers::common::get_signer_nonce,
    };

    let from = signer_state.get_expected_value("signer_address")?;

    let rpc_api_url = values.get_expected_string(RPC_API_URL)?;
    let chain_id = values.get_expected_uint(CHAIN_ID)?;

    let contract_address = values.get_expected_value(CONTRACT_ADDRESS)?;
    let contract_abi = values.get_string(CONTRACT_ABI);
    let function_name = values.get_expected_string(CONTRACT_FUNCTION_NAME)?;
    let function_args: Vec<DynSolValue> = values
        .get_value(CONTRACT_FUNCTION_ARGS)
        .map(|v| {
            v.expect_array()
                .iter()
                .map(|v| value_to_sol_value(&v).map_err(to_diag_with_ctx))
                .collect::<Result<Vec<DynSolValue>, Diagnostic>>()
        })
        .unwrap_or(Ok(vec![]))?;

    let (amount, gas_limit, mut nonce) =
        get_common_tx_params_from_args(values).map_err(to_diag_with_ctx)?;
    if nonce.is_none() {
        if let Some(signer_nonce) =
            get_signer_nonce(signer_state, chain_id).map_err(to_diag_with_ctx)?
        {
            nonce = Some(signer_nonce + 1);
        }
    }

    let tx_type = TransactionType::from_some_value(values.get_string(TRANSACTION_TYPE))?;

    let rpc = EvmRpc::new(&rpc_api_url).map_err(to_diag_with_ctx)?;

    let input = if let Some(abi_str) = contract_abi {
        encode_contract_call_inputs_from_abi_str(abi_str, function_name, &function_args)
            .map_err(to_diag_with_ctx)?
    } else {
        encode_contract_call_inputs_from_selector(function_name, &function_args)
            .map_err(to_diag_with_ctx)?
    };

    let common = CommonTransactionFields {
        to: Some(contract_address.clone()),
        from: from.clone(),
        nonce,
        chain_id,
        amount,
        gas_limit,
        tx_type,
        input: Some(input),
        deploy_code: None,
    };

    let res = build_unsigned_transaction(rpc, values, common).await.map_err(to_diag_with_ctx)?;
    Ok(res)
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
