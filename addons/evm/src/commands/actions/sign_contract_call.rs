use alloy::contract::Interface;
use alloy::dyn_abi::DynSolValue;
use alloy::hex;
use alloy::json_abi::JsonAbi;
use alloy::rpc::types::TransactionRequest;
use std::collections::HashMap;
use txtx_addon_kit::types::commands::{
    CommandExecutionResult, CommandImplementation, PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::{
    ActionItemRequest, ActionItemRequestType, ActionItemStatus, Actions, BlockEvent,
    ReviewInputRequest,
};
use txtx_addon_kit::types::signers::{
    return_synchronous_ok, SignerActionsFutureResult, SignerInstance, SignerSignFutureResult,
};
use txtx_addon_kit::types::ValueStore;
use txtx_addon_kit::types::{
    commands::CommandSpecification,
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{
    signers::SignersState, types::RunbookSupervisionContext, ConstructDid,
};
use txtx_addon_kit::AddonDefaults;

use crate::codec::CommonTransactionFields;
use crate::constants::{RPC_API_URL, SIGNED_TRANSACTION_BYTES, UNSIGNED_TRANSACTION_BYTES};
use crate::rpc::EVMRpc;
use crate::typing::EVM_ADDRESS;

use super::get_signer_did;

lazy_static! {
    pub static ref SIGN_EVM_CONTRACT_CALL: PreCommandSpecification = define_command! {
      SignEVMContractCall => {
          name: "Sign EVM Contract Call Transaction",
          matcher: "sign_contract_call",
          documentation: "The `evm::sign_contract_call` action encodes a contract call transaction, signs it with the provided signer data, and broadcasts it to the network.",
          implements_signing_capability: true,
          implements_background_task_capability: false,
          inputs: [
            description: {
                documentation: "A description of the transaction.",
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
            signer: {
                documentation: "A reference to a signer construct, which will be used to sign the transaction.",
                typing: Type::string(),
                optional: false,
                interpolable: true
            },
            contract_address: {
                documentation: "The address of the contract being called.",
                typing: Type::addon(EVM_ADDRESS),
                optional: false,
                interpolable: true
            },
            contract_abi: {
                documentation: "The contract ABI, optionally used to check input arguments before sending the transaction to the chain.",
                typing: Type::addon(EVM_ADDRESS),
                optional: true,
                interpolable: true
            },
            function_name: {
                documentation: "The contract function to invoke.",
                typing: Type::string(),
                optional: false,
                interpolable: true
            },
            function_args: {
                documentation: "The contract function arguments",
                typing: Type::array(Type::buffer()),
                optional: true,
                interpolable: true
            },
            amount: {
                documentation: "The amount, in WEI, to transfer.",
                typing: Type::integer(),
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
                typing: Type::integer(),
                optional: true,
                interpolable: true
            },
            max_priority_fee_per_gas: {
                documentation: "Sets the max priority fee per gas of an EIP1559 transaction.",
                typing: Type::integer(),
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
                typing: Type::integer(),
                optional: true,
                interpolable: true
            },
            gas_limit: {
                documentation: "Sets the maximum amount of gas that should be used to execute this transaction.",
                typing: Type::integer(),
                optional: true,
                interpolable: true
            },
            gas_price: {
                documentation: "Sets the gas price for Legacy transactions.",
                typing: Type::integer(),
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
            action "call_some_contract" "evm::sign_contract_call" {
                contract_address = evm::address(env.MY_CONTRACT_ADDRESS)
                function_name = "myFunction"
                function_args = [evm::bytes("0x1234")]
                from = evm::address(env.MY_ADDRESS)
            }
      "#},
      }
    };
}

pub struct SignEVMContractCall;
impl CommandImplementation for SignEVMContractCall {
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
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        mut signers: SignersState,
    ) -> SignerActionsFutureResult {
        use alloy::{consensus::Transaction, network::TransactionBuilder};

        use crate::{
            codec::get_typed_transaction_bytes,
            constants::{ACTION_ITEM_CHECK_FEE, ACTION_ITEM_CHECK_NONCE},
            typing::EvmValue,
        };

        let signer_did = get_signer_did(args).unwrap();

        let signer = signers_instances.get(&signer_did).unwrap().clone();
        let construct_did = construct_did.clone();
        let instance_name = instance_name.to_string();
        let spec = spec.clone();
        let args = args.clone();
        let defaults = defaults.clone();
        let supervision_context = supervision_context.clone();
        let signers_instances = signers_instances.clone();

        let future = async move {
            let mut actions = Actions::none();
            let mut signer_state = signers.pop_signer_state(&signer_did).unwrap();
            if let Some(_) = signer_state
                .get_scoped_value(&construct_did.value().to_string(), SIGNED_TRANSACTION_BYTES)
            {
                return Ok((signers, signer_state, Actions::none()));
            }

            let transaction = build_unsigned_contract_call(&signer_state, &spec, &args, &defaults)
                .await
                .map_err(|diag| (signers.clone(), signer_state.clone(), diag))?;

            let bytes = get_typed_transaction_bytes(&transaction).map_err(|e| {
                (
                    signers.clone(),
                    signer_state.clone(),
                    diagnosed_error!("command 'evm::sign_transfer': {e}"),
                )
            })?;
            let transaction = transaction.build_unsigned().unwrap();

            let payload = EvmValue::transaction(bytes);

            signer_state.insert_scoped_value(
                &construct_did.value().to_string(),
                UNSIGNED_TRANSACTION_BYTES,
                payload.clone(),
            );
            signers.push_signer_state(signer_state);
            let description = args
                .get_expected_string("description")
                .ok()
                .and_then(|d| Some(d.to_string()));

            if supervision_context.review_input_values {
                actions.push_panel("Transaction Signing", "");
                actions.push_sub_group(
                    description.clone(),
                    vec![
                        ActionItemRequest::new(
                            &Some(construct_did.clone()),
                            "".into(),
                            Some(format!("Check account nonce")),
                            ActionItemStatus::Todo,
                            ActionItemRequestType::ReviewInput(ReviewInputRequest {
                                input_name: "".into(),
                                value: Value::integer(transaction.nonce().into()),
                            }),
                            ACTION_ITEM_CHECK_NONCE,
                        ),
                        ActionItemRequest::new(
                            &Some(construct_did.clone()),
                            "µSTX".into(),
                            Some(format!("Check transaction fee")),
                            ActionItemStatus::Todo,
                            ActionItemRequestType::ReviewInput(ReviewInputRequest {
                                input_name: "".into(),
                                value: Value::integer(transaction.gas_limit().try_into().unwrap()), // todo
                            }),
                            ACTION_ITEM_CHECK_FEE,
                        ),
                    ],
                )
            }

            let signer_state = signers.pop_signer_state(&signer_did).unwrap();
            let (signers, signer_state, mut signer_actions) =
                (signer.specification.check_signability)(
                    &construct_did,
                    &instance_name,
                    &description,
                    &payload,
                    &signer.specification,
                    &args,
                    signer_state,
                    signers,
                    &signers_instances,
                    &defaults,
                    &supervision_context,
                )?;
            actions.append(&mut signer_actions);
            Ok((signers, signer_state, actions))
        };
        Ok(Box::pin(future))
    }

    fn run_signed_execution(
        construct_did: &ConstructDid,
        _spec: &CommandSpecification,
        args: &ValueStore,
        defaults: &AddonDefaults,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        mut signers: SignersState,
    ) -> SignerSignFutureResult {
        let signer_did = get_signer_did(args).unwrap();
        let signer_state = signers.pop_signer_state(&signer_did).unwrap();

        if let Ok(signed_transaction_bytes) = args.get_expected_value(SIGNED_TRANSACTION_BYTES) {
            let mut result = CommandExecutionResult::new();
            result.outputs.insert(
                SIGNED_TRANSACTION_BYTES.into(),
                signed_transaction_bytes.clone(),
            );
            return return_synchronous_ok(signers, signer_state, result);
        }

        let signer = signers_instances.get(&signer_did).unwrap();

        let payload = signer_state
            .get_scoped_value(&construct_did.to_string(), UNSIGNED_TRANSACTION_BYTES)
            .unwrap()
            .clone();

        let title = args
            .get_expected_string("description")
            .unwrap_or("New Transaction".into());

        let res = (signer.specification.sign)(
            construct_did,
            title,
            &payload,
            &signer.specification,
            &args,
            signer_state,
            signers,
            signers_instances,
            &defaults,
        );
        res
    }
}

#[cfg(not(feature = "wasm"))]
async fn build_unsigned_contract_call(
    signer_state: &ValueStore,
    _spec: &CommandSpecification,
    args: &ValueStore,
    defaults: &AddonDefaults,
) -> Result<TransactionRequest, Diagnostic> {
    use crate::{
        codec::{build_unsigned_transaction, value_to_sol_value, TransactionType},
        commands::actions::get_common_tx_params_from_args,
        constants::{
            CHAIN_ID, CONTRACT_ABI, CONTRACT_ADDRESS, CONTRACT_FUNCTION_ARGS,
            CONTRACT_FUNCTION_NAME, TRANSACTION_TYPE,
        },
    };

    let from = signer_state.get_expected_value("signer_address")?;

    // let network_id = args.get_defaulting_string(NETWORK_ID, defaults)?;
    let rpc_api_url = args.get_defaulting_string(RPC_API_URL, &defaults)?;
    let chain_id = args.get_defaulting_uint(CHAIN_ID, &defaults)?;

    let contract_address = args.get_expected_value(CONTRACT_ADDRESS)?;
    let contract_abi = args.get_string(CONTRACT_ABI);
    let function_name = args.get_expected_string(CONTRACT_FUNCTION_NAME)?;
    let function_args: Vec<DynSolValue> = args
        .get_value(CONTRACT_FUNCTION_ARGS)
        .map(|v| {
            v.expect_array()
                .iter()
                .map(|v| {
                    value_to_sol_value(&v)
                        .map_err(|e| diagnosed_error!("command 'evm::sign_contract_call': {}", e))
                })
                .collect::<Result<Vec<DynSolValue>, Diagnostic>>()
        })
        .unwrap_or(Ok(vec![]))?;

    let (amount, gas_limit, nonce) = get_common_tx_params_from_args(args)
        .map_err(|e| diagnosed_error!("command 'evm::sign_contract_call': {}", e))?;

    let tx_type = TransactionType::from_some_value(args.get_string(TRANSACTION_TYPE))?;

    let rpc = EVMRpc::new(&rpc_api_url)
        .map_err(|e| diagnosed_error!("command 'evm::sign_contract_call': {}", e))?;

    let input = if let Some(abi_str) = contract_abi {
        encode_contract_call_inputs_from_abi(abi_str, function_name, &function_args)
            .map_err(|e| diagnosed_error!("command 'sign_contract_call': {e}"))?
    } else {
        encode_contract_call_inputs_from_selector(function_name, &function_args)
            .map_err(|e| diagnosed_error!("command 'sign_contract_call': {e}"))?
    };

    let common = CommonTransactionFields {
        to: Some(contract_address.clone()),
        from: from.clone(),
        nonce,
        chain_id,
        amount: amount,
        gas_limit,
        tx_type,
        input: Some(input),
        deploy_code: None,
    };

    let tx = build_unsigned_transaction(rpc, args, common)
        .await
        .map_err(|e| diagnosed_error!("command 'evm::sign_contract_call': {e}"))?;
    Ok(tx)
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
    let encoded_args = function_args
        .iter()
        .flat_map(|v| v.abi_encode_params())
        .collect::<Vec<u8>>();
    let mut data = Vec::with_capacity(encoded_args.len() + 4);
    data.extend_from_slice(&selector[..]);
    data.extend_from_slice(&encoded_args[..]);
    Ok(data)
}

pub fn encode_contract_call_inputs_from_abi(
    abi_str: &str,
    function_name: &str,
    function_args: &Vec<DynSolValue>,
) -> Result<Vec<u8>, String> {
    let abi: JsonAbi =
        serde_json::from_str(&abi_str).map_err(|e| format!("invalid contract abi: {}", e))?;

    let interface = Interface::new(abi);
    interface
        .encode_input(function_name, &function_args)
        .map_err(|e| format!("failed to encode contract inputs: {e}"))
}
