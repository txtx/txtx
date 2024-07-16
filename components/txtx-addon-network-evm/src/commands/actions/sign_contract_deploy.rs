use alloy::rpc::types::TransactionRequest;
use alloy::{consensus::Transaction, network::TransactionBuilder};
use std::collections::HashMap;
use txtx_addon_kit::types::commands::{
    CommandExecutionResult, CommandImplementation, PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::{
    ActionItemRequest, ActionItemRequestType, ActionItemStatus, Actions, BlockEvent,
    ReviewInputRequest,
};
use txtx_addon_kit::types::wallets::{
    return_synchronous_ok, WalletActionsFutureResult, WalletInstance, WalletSignFutureResult,
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
use txtx_addon_kit::AddonDefaults;

use crate::{
    codec::get_typed_transaction_bytes,
    constants::{ACTION_ITEM_CHECK_FEE, ACTION_ITEM_CHECK_NONCE},
    typing::ETH_TRANSACTION,
};

use crate::codec::CommonTransactionFields;
use crate::constants::{
    NETWORK_ID, RPC_API_URL, SIGNED_TRANSACTION_BYTES, UNSIGNED_TRANSACTION_BYTES,
};
use crate::rpc::EVMRpc;
use crate::typing::DEPLOYMENT_ARTIFACTS_TYPE;

use super::get_signing_construct_did;

lazy_static! {
    pub static ref SIGN_EVM_CONTRACT_DEPLOY: PreCommandSpecification = define_command! {
      SignEVMContractDeploy => {
          name: "Sign EVM Contract Deployment Transaction",
          matcher: "sign_contract_deploy",
          documentation: "The `evm::sign_contract_deploy` action signs an unsigned transaction with the supplied wallet data.",
          implements_signing_capability: true,
          implements_background_task_capability: false,
          inputs: [
            description: {
                documentation: "A description of the transaction",
                typing: Type::string(),
                optional: true,
                interpolable: true
            },
            from: {
                documentation: "A reference to a wallet construct, which will be used to sign the transaction.",
                typing: Type::string(),
                optional: false,
                interpolable: true
            },
            artifacts: {
                documentation: indoc!{ r#"An object containing the deployment artifacts. Schema:
                ```json
                    {
                        "abi": String,
                        "bytecode": String,
                        "source": String,
                        "compiler_version": String
                    }
                ```
                "# },
                typing: DEPLOYMENT_ARTIFACTS_TYPE.clone(),
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
            network_id: {
                documentation: "The network id.",
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
            depends_on: {
              documentation: "References another command's outputs, preventing this command from executing until the referenced command is successful.",
              typing: Type::string(),
              optional: true,
              interpolable: true
            }
          ],
          outputs: [
              signed_transaction_bytes: {
                  documentation: "The signed transaction bytes.",
                  typing: Type::string()
              },
              network_id: {
                  documentation: "Network id of the signed transaction.",
                  typing: Type::string()
              }
          ],
          example: txtx_addon_kit::indoc! {r#"
          // Coming soon
      "#},
      }
    };
}

pub struct SignEVMContractDeploy;
impl CommandImplementation for SignEVMContractDeploy {
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
        let signing_construct_did = get_signing_construct_did(args).unwrap();

        let wallet = wallets_instances
            .get(&signing_construct_did)
            .unwrap()
            .clone();
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
                build_unsigned_contract_deploy(&signing_command_state, &spec, &args, &defaults)
                    .await
                    .map_err(|diag| (wallets.clone(), signing_command_state.clone(), diag))?;

            let bytes = get_typed_transaction_bytes(&transaction).map_err(|e| {
                (
                    wallets.clone(),
                    signing_command_state.clone(),
                    diagnosed_error!("command 'evm::sign_transfer': {e}"),
                )
            })?;
            let transaction = transaction.build_unsigned().unwrap();

            let payload = Value::buffer(bytes, ETH_TRANSACTION.clone());

            signing_command_state.insert_scoped_value(
                &construct_did.value().to_string(),
                UNSIGNED_TRANSACTION_BYTES,
                payload.clone(),
            );
            wallets.push_signing_command_state(signing_command_state);

            if supervision_context.review_input_values {
                actions.push_panel("Transaction Signing", "");
                actions.push_sub_group(vec![
                    ActionItemRequest::new(
                        &Some(construct_did.clone()),
                        "".into(),
                        Some(format!("Check account nonce")),
                        ActionItemStatus::Todo,
                        ActionItemRequestType::ReviewInput(ReviewInputRequest {
                            input_name: "".into(),
                            value: Value::uint(transaction.nonce()),
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
                            value: Value::uint(transaction.gas_limit().try_into().unwrap()), // todo
                        }),
                        ACTION_ITEM_CHECK_FEE,
                    ),
                ])
            }

            let signing_command_state = wallets
                .pop_signing_command_state(&signing_construct_did)
                .unwrap();
            let description = args
                .get_expected_string("description")
                .ok()
                .and_then(|d| Some(d.to_string()));
            let (wallets, wallet_state, mut wallet_actions) =
                (wallet.specification.check_signability)(
                    &construct_did,
                    &instance_name,
                    &description,
                    &payload,
                    &wallet.specification,
                    &args,
                    signing_command_state,
                    wallets,
                    &wallets_instances,
                    &defaults,
                    &supervision_context,
                )?;
            actions.append(&mut wallet_actions);
            Ok((wallets, wallet_state, actions))
        };
        Ok(Box::pin(future))
    }

    fn run_signed_execution(
        construct_did: &ConstructDid,
        _spec: &CommandSpecification,
        args: &ValueStore,
        defaults: &AddonDefaults,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
        wallets_instances: &HashMap<ConstructDid, WalletInstance>,
        mut wallets: SigningCommandsState,
    ) -> WalletSignFutureResult {
        let signing_construct_did = get_signing_construct_did(args).unwrap();
        let signing_command_state = wallets
            .pop_signing_command_state(&signing_construct_did)
            .unwrap();

        if let Ok(signed_transaction_bytes) = args.get_expected_value(SIGNED_TRANSACTION_BYTES) {
            let mut result = CommandExecutionResult::new();
            result.outputs.insert(
                SIGNED_TRANSACTION_BYTES.into(),
                signed_transaction_bytes.clone(),
            );
            return return_synchronous_ok(wallets, signing_command_state, result);
        }

        let wallet = wallets_instances.get(&signing_construct_did).unwrap();
        let payload = signing_command_state
            .get_scoped_value(
                &construct_did.value().to_string(),
                UNSIGNED_TRANSACTION_BYTES,
            )
            .unwrap()
            .clone();

        let title = args
            .get_expected_string("description")
            .unwrap_or("New Transaction".into());

        let res = (wallet.specification.sign)(
            construct_did,
            title,
            &payload,
            &wallet.specification,
            &args,
            signing_command_state,
            wallets,
            wallets_instances,
            &defaults,
        );
        res
    }
}

#[cfg(not(feature = "wasm"))]
async fn build_unsigned_contract_deploy(
    wallet_state: &ValueStore,
    _spec: &CommandSpecification,
    args: &ValueStore,
    defaults: &AddonDefaults,
) -> Result<TransactionRequest, Diagnostic> {
    use txtx_addon_kit::types::types::PrimitiveValue;

    use crate::{
        codec::{build_unsigned_transaction, TransactionType},
        constants::{ARTIFACTS, CHAIN_ID, GAS_LIMIT, NONCE, TRANSACTION_AMOUNT, TRANSACTION_TYPE},
    };

    let from = wallet_state.get_expected_value("signer_address")?;

    let network_id = args.get_defaulting_string(NETWORK_ID, defaults)?;
    let rpc_api_url = args.get_defaulting_string(RPC_API_URL, &defaults)?;
    let chain_id = args.get_defaulting_uint(CHAIN_ID, &defaults)?;

    let artifacts = args.get_expected_object(ARTIFACTS)?;

    let Some(Value::Primitive(PrimitiveValue::String(bytecode))) = artifacts.get("bytecode") else {
        return Err(diagnosed_error!(
            "command: 'evm::sign_contract_deploy': contract deployment artifacts missing bytecode"
        ));
    };
    let bytecode = alloy::hex::decode(bytecode)
        .map_err(|e| diagnosed_error!("command 'evm::sign_contract_deploy': {}", e))?;

    let amount = args
        .get_value(TRANSACTION_AMOUNT)
        .map(|v| v.expect_uint())
        .unwrap_or(0);
    let gas_limit = args.get_value(GAS_LIMIT).map(|v| v.expect_uint());
    let nonce = args.get_value(NONCE).map(|v| v.expect_uint());
    let tx_type = TransactionType::from_some_value(args.get_string(TRANSACTION_TYPE))?;

    let rpc = EVMRpc::new(&rpc_api_url)
        .map_err(|e| diagnosed_error!("command 'evm::sign_contract_deploy': {}", e))?;

    let common = CommonTransactionFields {
        to: None,
        from: from.clone(),
        nonce,
        chain_id,
        amount: amount,
        gas_limit,
        tx_type,
        input: None,
        deploy_code: Some(bytecode),
    };
    let tx = build_unsigned_transaction(rpc, args, common)
        .await
        .map_err(|e| diagnosed_error!("command: 'evm::sign_contract_deploy': {e}"))?;
    Ok(tx)
}
