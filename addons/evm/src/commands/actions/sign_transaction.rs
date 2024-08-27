use alloy::consensus::Transaction;
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

use crate::constants::{ACTION_ITEM_CHECK_FEE, ACTION_ITEM_CHECK_NONCE};

use crate::constants::{SIGNED_TRANSACTION_BYTES, UNSIGNED_TRANSACTION_BYTES};

use super::get_signer_did;

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
        signers_instances: &HashMap<ConstructDid, SignerInstance>,
        mut signers: SignersState,
    ) -> SignerActionsFutureResult {
        use alloy::{
            network::TransactionBuilder, primitives::TxKind, rpc::types::TransactionRequest,
        };

        use crate::constants::TRANSACTION_PAYLOAD_BYTES;

        let signer_did = get_signer_did(args).unwrap();

        let signer = signers_instances.get(&signer_did).unwrap().clone();
        let construct_did = construct_did.clone();
        let instance_name = instance_name.to_string();
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

            let payload = args
                .get_expected_value(TRANSACTION_PAYLOAD_BYTES)
                .map_err(|diag| (signers.clone(), signer_state.clone(), diag))?;
            let transaction_bytes = payload.expect_buffer_bytes();

            let mut transaction: TransactionRequest =
                serde_json::from_slice(&transaction_bytes[..]).map_err(|e| {
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
            let transaction = transaction.build_unsigned().unwrap();

            signer_state.insert_scoped_value(
                &construct_did.value().to_string(),
                UNSIGNED_TRANSACTION_BYTES,
                payload.clone(),
            );
            signers.push_signer_state(signer_state);
            let description =
                args.get_expected_string("description").ok().and_then(|d| Some(d.to_string()));

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
            result
                .outputs
                .insert(SIGNED_TRANSACTION_BYTES.into(), signed_transaction_bytes.clone());
            return return_synchronous_ok(signers, signer_state, result);
        }

        let signer = signers_instances.get(&signer_did).unwrap();
        let payload = signer_state
            .get_scoped_value(&construct_did.to_string(), UNSIGNED_TRANSACTION_BYTES)
            .unwrap()
            .clone();

        let title = args.get_expected_string("description").unwrap_or("New Transaction".into());

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
