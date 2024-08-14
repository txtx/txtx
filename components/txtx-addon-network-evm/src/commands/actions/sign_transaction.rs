use alloy::consensus::Transaction;
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

use crate::constants::{ACTION_ITEM_CHECK_FEE, ACTION_ITEM_CHECK_NONCE};

use crate::constants::{SIGNED_TRANSACTION_BYTES, UNSIGNED_TRANSACTION_BYTES};

use super::get_signing_construct_did;

lazy_static! {
    pub static ref SIGN_TRANSACTION: PreCommandSpecification = define_command! {
      SignEVMTransaction => {
          name: "Sign EVM Transaction",
          matcher: "sign_transaction",
          documentation: "Coming soon",
          implements_signing_capability: true,
          implements_background_task_capability: false,
          inputs: [
            description: {
                documentation: "A description of the transaction",
                typing: Type::string(),
                optional: true,
                interpolable: true
            },
            transaction_payload_bytes: {
                documentation: "The unsigned transaction payload bytes.",
                typing: Type::string(),
                optional: false,
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

pub struct SignEVMTransaction;
impl CommandImplementation for SignEVMTransaction {
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
        _spec: &CommandSpecification,
        args: &ValueStore,
        defaults: &AddonDefaults,
        supervision_context: &RunbookSupervisionContext,
        wallets_instances: &HashMap<ConstructDid, WalletInstance>,
        mut wallets: SigningCommandsState,
    ) -> WalletActionsFutureResult {
        use alloy::{
            network::TransactionBuilder, primitives::TxKind, rpc::types::TransactionRequest,
        };

        use crate::constants::TRANSACTION_PAYLOAD_BYTES;

        let signing_construct_did = get_signing_construct_did(args).unwrap();

        let wallet = wallets_instances
            .get(&signing_construct_did)
            .unwrap()
            .clone();
        let construct_did = construct_did.clone();
        let instance_name = instance_name.to_string();
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

            let payload = args
                .get_expected_value(TRANSACTION_PAYLOAD_BYTES)
                .map_err(|diag| (wallets.clone(), signing_command_state.clone(), diag))?;
            let transaction_bytes = payload.expect_buffer_bytes();

            let mut transaction: TransactionRequest =
                serde_json::from_slice(&transaction_bytes[..]).map_err(|e| {
                    (
                        wallets.clone(),
                        signing_command_state.clone(),
                        diagnosed_error!("error deserializing transaction: {e}"),
                    )
                })?;

            // The transaction kind isn't serialized as part of the tx, so we need to ensure that the tx kind
            // is Create if there is no to address. maybe we should consider some additional checks here to
            // ensure we aren't errantly setting it to create
            if None == transaction.to {
                transaction = transaction.with_kind(TxKind::Create);
            }
            let transaction = transaction.build_unsigned().unwrap();

            signing_command_state.insert_scoped_value(
                &construct_did.value().to_string(),
                UNSIGNED_TRANSACTION_BYTES,
                payload.clone(),
            );
            wallets.push_signing_command_state(signing_command_state);
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

            let signing_command_state = wallets
                .pop_signing_command_state(&signing_construct_did)
                .unwrap();

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
            .get_scoped_value(&construct_did.to_string(), UNSIGNED_TRANSACTION_BYTES)
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
