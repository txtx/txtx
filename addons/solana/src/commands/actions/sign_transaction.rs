use std::collections::HashMap;
use txtx_addon_kit::constants::SIGNED_TRANSACTION_BYTES;
use txtx_addon_kit::types::commands::{
    CommandExecutionResult, CommandImplementation, PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::{
    ActionItemRequest, ActionItemRequestType, ActionItemStatus, Actions, BlockEvent,
    ReviewInputRequest,
};
use txtx_addon_kit::types::signers::{
    return_synchronous_ok, SignerActionsFutureResult, SignerInstance, SignerSignFutureResult,
    SignersState,
};
use txtx_addon_kit::types::types::RunbookSupervisionContext;
use txtx_addon_kit::types::{
    commands::CommandSpecification,
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{ConstructDid, ValueStore};
use txtx_addon_kit::AddonDefaults;

use crate::constants::{TRANSACTION_PAYLOAD_BYTES, UNSIGNED_TRANSACTION_BYTES};

use super::get_signer_did;

lazy_static! {
    pub static ref SIGN_SOLANA_TRANSACTION: PreCommandSpecification = define_command! {
      SignSolanaTransaction => {
          name: "Sign Solana Transaction",
          matcher: "sign_transaction",
          documentation: "The `solana::sign_transaction` is coming soon.",
          implements_signing_capability: true,
          implements_background_task_capability: false,
          inputs: [
            description: {
                documentation: "Description of the transaction",
                typing: Type::string(),
                optional: true,
                interpolable: true
            },
            transaction_payload_bytes: {
                documentation: "The transaction payload bytes, encoded as a clarity buffer.",
                typing: Type::string(),
                optional: false,
                interpolable: true
            },
            chain_id: {
                documentation: indoc!{r#"Coming soon"#},
                typing: Type::string(),
                optional: true,
                interpolable: true
            },
            signer: {
                documentation: "A reference to a signer construct, which will be used to sign the transaction payload.",
                typing: Type::string(),
                optional: false,
                interpolable: true
            },
            nonce: {
                documentation: "The account nonce of the signer. This value will be retrieved from the network if omitted.",
                typing: Type::integer(),
                optional: true,
                interpolable: true
            }
          ],
          outputs: [
              signed_transaction_bytes: {
                  documentation: "The signed transaction bytes.",
                  typing: Type::string()
              },
              chain_id: {
                  documentation: "Coming soon.",
                  typing: Type::string()
              }
          ],
          example: txtx_addon_kit::indoc! {r#"
          // Coming soon
      "#},
      }
    };
}

pub struct SignSolanaTransaction;
impl CommandImplementation for SignSolanaTransaction {
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
        use txtx_addon_kit::constants::SIGNATURE_APPROVED;

        use crate::{constants::ACTION_ITEM_CHECK_NONCE, typing::SolanaValue};

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
            if signer_state
                .get_scoped_value(&construct_did.to_string(), SIGNED_TRANSACTION_BYTES)
                .is_some()
                || signer_state
                    .get_scoped_value(&construct_did.to_string(), SIGNATURE_APPROVED)
                    .is_some()
            {
                return Ok((signers, signer_state, Actions::none()));
            }

            let nonce = args.get_value("nonce").map(|v| v.expect_uint().unwrap());

            let transaction_payload_bytes =
                match args.get_expected_buffer_bytes(TRANSACTION_PAYLOAD_BYTES) {
                    Ok(bytes) => bytes,
                    Err(e) => return Err((signers, signer_state, diagnosed_error!("{e}"))),
                };

            let payload = SolanaValue::transaction(transaction_payload_bytes);

            signer_state.insert_scoped_value(
                &construct_did.to_string(),
                UNSIGNED_TRANSACTION_BYTES,
                payload.clone(),
            );
            let description =
                args.get_expected_string("description").ok().and_then(|d| Some(d.to_string()));

            if supervision_context.review_input_values {
                actions.push_group(
                    &description
                        .clone()
                        .unwrap_or("Review and sign the transactions from the list below".into()),
                    vec![ActionItemRequest::new(
                        &Some(construct_did.clone()),
                        "".into(),
                        Some(format!("Check account nonce")),
                        ActionItemStatus::Todo,
                        ActionItemRequestType::ReviewInput(ReviewInputRequest {
                            input_name: "".into(),
                            value: Value::integer(nonce.unwrap() as i128),
                        }),
                        ACTION_ITEM_CHECK_NONCE,
                    )],
                )
            }

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
