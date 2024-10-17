use alloy::consensus::Transaction;
use std::collections::HashMap;
use txtx_addon_kit::types::commands::{
    CommandExecutionResult, CommandImplementation, PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::{
    ActionItemRequest, ActionItemStatus, Actions, BlockEvent, ReviewInputRequest,
};
use txtx_addon_kit::types::signers::{
    return_synchronous_ok, SignerActionsFutureResult, SignerInstance, SignerSignFutureResult,
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

use crate::constants::{ACTION_ITEM_CHECK_FEE, ACTION_ITEM_CHECK_NONCE, ALREADY_DEPLOYED};

use crate::constants::SECRET_KEY_WALLET_UNSIGNED_TRANSACTION_BYTES;
use crate::typing::EvmValue;
use txtx_addon_kit::constants::TX_HASH;

use super::get_signer_did;

lazy_static! {
    pub static ref SIGN_TRANSACTION: PreCommandSpecification = define_command! {
      SignEvmTransaction => {
          name: "Sign EVM Transaction",
          matcher: "sign_transaction",
          documentation: "The `evm::sign_transaction` command signs an EVM transaction.",
          implements_signing_capability: true,
          implements_background_task_capability: false,
          inputs: [
            description: {
                documentation: "A description of the transaction",
                typing: Type::string(),
                optional: true,
                tainting: false,
                internal: false
            },
            transaction_payload_bytes: {
                documentation: "The unsigned transaction payload bytes.",
                typing: Type::string(),
                optional: false,
                tainting: true,
                internal: false
            },
            signer: {
                documentation: "A reference to a signer construct, which will be used to sign the transaction payload.",
                typing: Type::string(),
                optional: false,
                tainting: true,
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
          action "signed_tx" "evm::sign_transaction" {
              description = "Deploy a new contract"
              transaction_payload_bytes = "0x1234567890abcdef"
              signer = signer.operator
          }
      "#},
      }
    };
}

pub struct SignEvmTransaction;
impl CommandImplementation for SignEvmTransaction {
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
        values: &ValueStore,
        supervision_context: &RunbookSupervisionContext,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        mut signers: SignersState,
    ) -> SignerActionsFutureResult {
        use alloy::{
            network::TransactionBuilder, primitives::TxKind, rpc::types::TransactionRequest,
        };

        use crate::{
            codec::{
                format_transaction_cost, format_transaction_for_display, typed_transaction_bytes,
            },
            constants::{
                ALREADY_DEPLOYED, FORMATTED_TRANSACTION, TRANSACTION_COST,
                TRANSACTION_PAYLOAD_BYTES, WEB_WALLET_UNSIGNED_TRANSACTION_BYTES,
            },
        };

        let signer_did = get_signer_did(values).unwrap();

        let signer = signers_instances.get(&signer_did).unwrap().clone();
        let construct_did = construct_did.clone();
        let instance_name = instance_name.to_string();
        let values = values.clone();
        let supervision_context = supervision_context.clone();
        let signers_instances = signers_instances.clone();

        let future = async move {
            let mut actions = Actions::none();
            let mut signer_state = signers.pop_signer_state(&signer_did).unwrap();
            if let Some(_) =
                signer_state.get_scoped_value(&construct_did.value().to_string(), TX_HASH)
            {
                return Ok((signers, signer_state, Actions::none()));
            }

            let description =
                values.get_expected_string("description").ok().and_then(|d| Some(d.to_string()));

            let already_deployed = signer_state
                .get_scoped_bool(&construct_did.to_string(), ALREADY_DEPLOYED)
                .unwrap_or(false);
            // if this transaction is a contract deployment that's already been done,
            // the signer may still want to have some action items, but we won't build a transaction
            if already_deployed {
                if supervision_context.review_input_values {
                    actions.push_panel("Transaction Execution", "");
                    actions.push_sub_group(description.clone(), vec![]);
                }
            } else {
                let transaction_request_bytes = values
                    .get_expected_buffer_bytes(TRANSACTION_PAYLOAD_BYTES)
                    .map_err(|diag| (signers.clone(), signer_state.clone(), diag))?;

                let mut transaction: TransactionRequest =
                    serde_json::from_slice(&transaction_request_bytes[..]).map_err(|e| {
                        (
                            signers.clone(),
                            signer_state.clone(),
                            diagnosed_error!("error deserializing transaction: {e}"),
                        )
                    })?;

                // The transaction kind isn't serialized as part of the tx, so we need to ensure that the tx kind
                // is Create if there is no to address. maybe we should consider some additional checks here to
                // ensure we aren't errantly setting it to create
                if None == transaction.to {
                    transaction = transaction.with_kind(TxKind::Create);
                }
                let transaction = transaction.build_unsigned().map_err(|e| {
                    (
                        signers.clone(),
                        signer_state.clone(),
                        diagnosed_error!("error building unsigned transaction: {e}"),
                    )
                })?;

                let web_wallet_payload_bytes = typed_transaction_bytes(&transaction);
                let web_wallet_payload = Value::buffer(web_wallet_payload_bytes);

                // the secret key wallet and web wallet need the transaction in slightly different formats,
                // so we'll store them in separate keys and allow the signer to choose which one it needs
                signer_state.insert_scoped_value(
                    &construct_did.to_string(),
                    SECRET_KEY_WALLET_UNSIGNED_TRANSACTION_BYTES,
                    Value::buffer(transaction_request_bytes),
                );
                signer_state.insert_scoped_value(
                    &construct_did.value().to_string(),
                    WEB_WALLET_UNSIGNED_TRANSACTION_BYTES,
                    web_wallet_payload.clone(),
                );
                let display_payload = format_transaction_for_display(&transaction);
                signer_state.insert_scoped_value(
                    &construct_did.to_string(),
                    FORMATTED_TRANSACTION,
                    Value::string(display_payload),
                );

                let mut action_items = vec![ActionItemRequest::new(
                    &Some(construct_did.clone()),
                    "".into(),
                    Some(format!("Check account nonce")),
                    ActionItemStatus::Todo,
                    ReviewInputRequest::new("", &Value::integer(transaction.nonce().into()))
                        .to_action_type(),
                    ACTION_ITEM_CHECK_NONCE,
                )];

                if let Some(tx_cost) =
                    signer_state.get_scoped_integer(&construct_did.to_string(), TRANSACTION_COST)
                {
                    let formatted_cost = format_transaction_cost(tx_cost).map_err(|e| {
                        (
                            signers.clone(),
                            signer_state.clone(),
                            diagnosed_error!("failed to format transaction cost: {e}"),
                        )
                    })?;
                    action_items.push(ActionItemRequest::new(
                        &Some(construct_did.clone()),
                        "Wei".into(),
                        Some(format!("Check transaction cost")),
                        ActionItemStatus::Todo,
                        ReviewInputRequest::new("", &Value::string(formatted_cost))
                            .to_action_type(),
                        ACTION_ITEM_CHECK_FEE,
                    ));
                }
                if supervision_context.review_input_values {
                    actions.push_panel("Transaction Execution", "");
                    actions.push_sub_group(description.clone(), action_items)
                }
            }

            let (signers, signer_state, mut signer_actions) =
                (signer.specification.check_signability)(
                    &construct_did,
                    &instance_name,
                    &description,
                    &Value::null(), // null payload because we want to signer to pull the appropriate one from the state
                    &signer.specification,
                    &values,
                    signer_state,
                    signers,
                    &signers_instances,
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
        values: &ValueStore,
        _progress_tx: &txtx_addon_kit::channel::Sender<BlockEvent>,
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        mut signers: SignersState,
    ) -> SignerSignFutureResult {
        let signer_did = get_signer_did(values).unwrap();
        let signer_state = signers.pop_signer_state(&signer_did).unwrap();

        let already_deployed = signer_state
            .get_scoped_bool(&construct_did.to_string(), ALREADY_DEPLOYED)
            .unwrap_or(false);
        if let Some(tx_hash) =
            signer_state.get_scoped_value(&construct_did.value().to_string(), TX_HASH)
        {
            let mut result = CommandExecutionResult::new();
            if !already_deployed {
                let tx_hash_bytes = tx_hash.expect_buffer_bytes_result().map_err(|e| {
                    (
                        signers.clone(),
                        signer_state.clone(),
                        diagnosed_error!("failed to get tx hash bytes: {e}"),
                    )
                })?;
                let tx_hash = EvmValue::tx_hash(tx_hash_bytes);
                result.outputs.insert(TX_HASH.into(), tx_hash.clone());
                result.outputs.insert(TX_HASH.into(), tx_hash.clone());
            }
            return return_synchronous_ok(signers, signer_state, result);
        }

        let signer = signers_instances.get(&signer_did).unwrap();

        let title = values.get_expected_string("description").unwrap_or("New Transaction".into());

        let res = (signer.specification.sign)(
            construct_did,
            title,
            &Value::null(),
            &signer.specification,
            &values,
            signer_state,
            signers,
            signers_instances,
        );
        res
    }
}
