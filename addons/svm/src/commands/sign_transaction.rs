use crate::commands::get_signers_did;
use crate::typing::{SvmValue, SVM_TRANSACTION};
use solana_sdk::signature::Signature;
use std::collections::HashMap;
use txtx_addon_kit::constants::SIGNED_TRANSACTION_BYTES;
use txtx_addon_kit::types::commands::{
    CommandExecutionResult, CommandImplementation, PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::signers::{
    SignerActionsFutureResult, SignerInstance, SignerSignFutureResult, SignersState,
};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::RunbookSupervisionContext;
use txtx_addon_kit::types::{commands::CommandSpecification, diagnostics::Diagnostic, types::Type};
use txtx_addon_kit::types::{ConstructDid, Did};

use crate::constants::{
    AUTHORITY, IS_DEPLOYMENT, PARTIALLY_SIGNED_TRANSACTION_BYTES, TRANSACTION_BYTES,
};

use super::deploy_program::get_deployment_signers;
use super::get_signer_did;

lazy_static! {
    pub static ref SIGN_TRANSACTION: PreCommandSpecification = define_command! {
      SignTransaction => {
          name: "Sign SVM Transaction",
          matcher: "sign_transaction",
          documentation: "The `svm::send_transaction` is used to sign a transaction and broadcast it to the specified SVM-compatible network.",
          implements_signing_capability: true,
          implements_background_task_capability: false,
          inputs: [
            description: {
                documentation: "A description of the transaction.",
                typing: Type::string(),
                optional: true,
                tainting: false,
                internal: false,
                sensitive: false
            },
            transaction_bytes: {
                documentation: "The transaction bytes to sign.",
                typing: Type::addon(SVM_TRANSACTION),
                optional: false,
                tainting: true,
                internal: false,
                sensitive: false
            },
            signers: {
                documentation: "A set of references to a signer construct, which will be used to sign the transaction.",
                typing: Type::array(Type::string()),
                optional: true,
                tainting: true,
                internal: false,
                sensitive: false
            },
            signer: {
                documentation: "A reference to a signer construct, which will be used to sign the transaction.",
                typing: Type::array(Type::string()),
                optional: true,
                tainting: true,
                internal: false
            }
          ],
          outputs: [
              signed_transaction_bytes: {
                  documentation: "The signed transaction bytes.",
                  typing: Type::addon(SVM_TRANSACTION)
              }
          ],
          example: txtx_addon_kit::indoc! {r#"
          // Coming soon
      "#},
      }
    };
}

pub struct SignTransaction;
impl CommandImplementation for SignTransaction {
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
        use txtx_addon_kit::{constants::SIGNATURE_APPROVED, types::types::Value};

        use crate::{
            codec::UpgradeableProgramDeployer, commands::get_custom_signer_did,
            constants::FORMATTED_TRANSACTION,
        };

        let construct_did = construct_did.clone();
        let instance_name = instance_name.to_string();
        let values = values.clone();
        let supervision_context = supervision_context.clone();
        let signers_instances = signers_instances.clone();

        let future = async move {
            let mut actions = Actions::none();
            let is_deployment = values.get_bool(IS_DEPLOYMENT).unwrap_or(false);

            let description =
                values.get_expected_string("description").ok().and_then(|d| Some(d.to_string()));

            if is_deployment {
                let payload =
                    values.get_value(TRANSACTION_BYTES).unwrap().as_array().unwrap().clone();
                let tx_count = payload.len();
                let mut cursor = 0;

                let authority_signer_did = get_custom_signer_did(&values, AUTHORITY).unwrap();
                let authority_signer_state =
                    signers.get_signer_state(&authority_signer_did).unwrap().clone();

                for tx_value in payload.iter() {
                    cursor += 1;
                    let (transaction_bytes, signer_did) =
                        UpgradeableProgramDeployer::get_signer_did_from_transaction_value(
                            &tx_value, &values,
                        )
                        .map_err(|e| (signers.clone(), authority_signer_state.clone(), e))?;

                    let Some(signer_did) = signer_did else {
                        if cursor == tx_count - 1 {
                            let authority_signer_state =
                                signers.get_signer_state(&authority_signer_did).unwrap().clone();
                            return Ok((signers, authority_signer_state.clone(), actions));
                        } else {
                            continue;
                        }
                    };

                    let signer_instance = signers_instances.get(&signer_did).unwrap();
                    let mut signer_state = signers.get_signer_state(&signer_did).unwrap().clone();

                    signer_state.insert_scoped_value(
                        &construct_did.to_string(),
                        FORMATTED_TRANSACTION,
                        Value::string(format!("Transaction signed by '{}'", signer_instance.name)),
                    );

                    let payload = SvmValue::transaction_from_bytes(transaction_bytes);

                    let (new_signers, new_signer_state, mut signer_actions) =
                        (signer_instance.specification.check_signability)(
                            &construct_did,
                            &instance_name,
                            &description,
                            &payload,
                            &signer_instance.specification,
                            &values,
                            signer_state.clone(),
                            signers,
                            &signers_instances,
                            &supervision_context,
                        )?;
                    signers = new_signers;
                    signers.push_signer_state(new_signer_state.clone());
                    signer_state = new_signer_state.clone();
                    actions.append(&mut signer_actions);

                    if cursor == tx_count - 1 {
                        return Ok((signers, signer_state.clone(), actions));
                    }
                }
                unreachable!()
            } else {
                let signers_dids_with_instances =
                    get_signers_and_instance(&values, &signers_instances).unwrap();
                let signers_count = signers_dids_with_instances.len();
                let mut cursor = 0;

                for (signer_did, signer_instance) in signers_dids_with_instances {
                    let mut signer_state = signers.get_signer_state(&signer_did).unwrap().clone();

                    let signer_already_signed = signer_state
                        .get_scoped_value(&construct_did.to_string(), SIGNED_TRANSACTION_BYTES)
                        .is_some();
                    let signer_already_approved = signer_state
                        .get_scoped_value(&construct_did.to_string(), SIGNATURE_APPROVED)
                        .is_some();

                    if !signer_already_signed && !signer_already_approved {
                        let payload = values.get_value(TRANSACTION_BYTES).unwrap().clone();

                        let (new_signers, new_signer_state, mut signer_actions) =
                            (signer_instance.specification.check_signability)(
                                &construct_did,
                                &instance_name,
                                &description,
                                &payload,
                                &signer_instance.specification,
                                &values,
                                signer_state.clone(),
                                signers,
                                &signers_instances,
                                &supervision_context,
                            )?;
                        signers = new_signers;
                        signers.push_signer_state(new_signer_state.clone());
                        signer_state = new_signer_state.clone();
                        actions.append(&mut signer_actions);
                    }

                    if cursor == signers_count - 1 {
                        return Ok((signers, signer_state.clone(), actions));
                    }
                    cursor += 1;
                }
                panic!("No signers found");
            };
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
        let construct_did = construct_did.clone();
        let values = values.clone();
        let signers_instances = signers_instances.clone();

        let future = async move {
            let title =
                values.get_expected_string("description").unwrap_or("New Transaction".into());

            let is_deployment = values.get_bool(IS_DEPLOYMENT).unwrap_or(false);
            if is_deployment {
                let (
                    (authority_signer_did, authority_signer_state),
                    (payer_signer_did, payer_signer_state),
                ) = get_deployment_signers(&values, &mut signers);

                let authority_signer_instance =
                    signers_instances.get(&authority_signer_did).unwrap();

                let expected_signer_did = values
                    .get_string("expected_signer")
                    .and_then(|did| Some(ConstructDid(Did::from_hex_string(did))));

                let (signer_state, signer_instance) =
                    if let Some(expected_signer) = expected_signer_did {
                        if expected_signer.eq(&authority_signer_did) {
                            (authority_signer_state, authority_signer_instance)
                        } else if expected_signer.eq(&payer_signer_did) {
                            let payer_signer_instance =
                                signers_instances.get(&payer_signer_did).unwrap();
                            (
                                payer_signer_state.expect("Payer signer state not found"),
                                payer_signer_instance,
                            )
                        } else {
                            unreachable!("No expected signer found");
                        }
                    } else {
                        (authority_signer_state, authority_signer_instance)
                    };

                let payload = values.get_value(TRANSACTION_BYTES).unwrap();

                let res = (signer_instance.specification.sign)(
                    &construct_did,
                    title,
                    &payload,
                    &signer_instance.specification,
                    &values,
                    signer_state,
                    signers,
                    &signers_instances,
                )?;
                return res.await;
            } else {
                let signers_dids_with_instances =
                    get_signers_and_instance(&values, &signers_instances).unwrap();

                let signers_count = signers_dids_with_instances.len();

                let (first_signer_did, first_signer_instance) =
                    signers_dids_with_instances.first().unwrap();
                let first_signer_state =
                    signers.get_signer_state(first_signer_did).unwrap().clone();

                let payload = first_signer_state
                    .get_scoped_value(&construct_did.to_string(), TRANSACTION_BYTES)
                    .unwrap();
                let mut combined_transaction = SvmValue::to_transaction(&payload).unwrap();
                let mut cursor = 0;

                for (signer_did, signer_instance) in signers_dids_with_instances {
                    let signer_state = signers.pop_signer_state(&signer_did).unwrap();

                    if let Some(fully_signed_transaction_bytes) = signer_state.get_scoped_value(
                        &construct_did.value().to_string(),
                        SIGNED_TRANSACTION_BYTES,
                    ) {
                        let mut result = CommandExecutionResult::new();
                        result.outputs.insert(
                            SIGNED_TRANSACTION_BYTES.into(),
                            fully_signed_transaction_bytes.clone(),
                        );
                        return Ok((signers, signer_state, result));
                    }

                    match (signer_instance.specification.sign)(
                        &construct_did,
                        title,
                        &payload,
                        &signer_instance.specification,
                        &values,
                        signer_state,
                        signers,
                        &signers_instances,
                    ) {
                        Ok(res) => match res.await {
                            Ok((new_signers, new_signer_state, results)) => {
                                let partial_signed_tx_value = results
                                    .outputs
                                    .get(PARTIALLY_SIGNED_TRANSACTION_BYTES)
                                    .expect("Signed transaction bytes not found");
                                let partial_signed_tx = SvmValue::to_transaction(
                                    partial_signed_tx_value,
                                )
                                .map_err(|e| (new_signers.clone(), new_signer_state.clone(), e))?;

                                for (i, sig) in partial_signed_tx.signatures.iter().enumerate() {
                                    if sig != &Signature::default() {
                                        combined_transaction.signatures[i] = sig.clone();
                                    }
                                }
                                let all_txtx_signers_signed = cursor == signers_count - 1;
                                let is_fully_signed = combined_transaction.is_signed();

                                if is_fully_signed {
                                    let mut result = CommandExecutionResult::new();
                                    result.outputs.insert(
                                        SIGNED_TRANSACTION_BYTES.into(),
                                        SvmValue::transaction(&combined_transaction).map_err(
                                            |e| (new_signers.clone(), new_signer_state.clone(), e),
                                        )?,
                                    );
                                    return Ok((new_signers, new_signer_state, result));
                                }
                                if all_txtx_signers_signed {
                                    return Err((
                                        new_signers,
                                        new_signer_state,
                                        diagnosed_error!("all provided signers have signed the transaction, but the transaction is not fully signed"),
                                    ));
                                }
                                signers = new_signers;
                            }
                            Err((signers, signer_state, diag)) => {
                                return Err((
                                    signers,
                                    signer_state,
                                    diagnosed_error!(
                                        "'{}::{}' signer '{}' failed to sign transaction: {}",
                                        signer_instance.namespace,
                                        signer_instance.specification.matcher,
                                        signer_instance.name,
                                        diag.message
                                    ),
                                ));
                            }
                        },
                        Err((signers, signer_state, diag)) => {
                            return Err((
                                signers,
                                signer_state,
                                diagnosed_error!(
                                    "'{}::{}' signer '{}' failed to sign transaction: {}",
                                    signer_instance.namespace,
                                    signer_instance.specification.matcher,
                                    signer_instance.name,
                                    diag.message
                                ),
                            ));
                        }
                    };

                    cursor += 1;
                }
                unreachable!("no signers found");
            };
        };
        Ok(Box::pin(future))
    }
}

fn get_signers_and_instance(
    values: &ValueStore,
    signers_instances: &HashMap<ConstructDid, SignerInstance>,
) -> Result<Vec<(ConstructDid, SignerInstance)>, Diagnostic> {
    match get_signers_did(values) {
        Ok(signers_did) => {
            let res = signers_did
                .iter()
                .map(|did| {
                    signers_instances
                        .get(did)
                        .ok_or(diagnosed_error!("Signer instance not found"))
                        .map(|s| (did.clone(), s.clone()))
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(res)
        }
        Err(_) => {
            let signer_did = get_signer_did(values)?;
            let signer_instance = signers_instances
                .get(&signer_did)
                .ok_or_else(|| diagnosed_error!("Signer instance not found"))?;
            Ok(vec![(signer_did, signer_instance.clone())])
        }
    }
}
