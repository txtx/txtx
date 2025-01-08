use crate::commands::get_signers_did;
use std::collections::HashMap;
use txtx_addon_kit::constants::SIGNED_TRANSACTION_BYTES;
use txtx_addon_kit::types::commands::{
    CommandExecutionResult, CommandImplementation, PreCommandSpecification,
};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::signers::{
    return_synchronous_ok, SignerActionsFutureResult, SignerInstance, SignerSignFutureResult,
    SignersState,
};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::RunbookSupervisionContext;
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::types::{commands::CommandSpecification, diagnostics::Diagnostic, types::Type};

use crate::constants::{IS_ARRAY, TRANSACTION_BYTES};

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
                documentation: "Description of the transaction",
                typing: Type::string(),
                optional: true,
                tainting: false,
                internal: false
            },
            transaction_payload_bytes: {
                documentation: "The transaction payload bytes, encoded as a clarity buffer.",
                typing: Type::string(),
                optional: true,
                tainting: true,
                internal: false
            },
            chain_id: {
                documentation: indoc!{r#"Coming soon"#},
                typing: Type::string(),
                optional: true,
                tainting: true,
                internal: false
            },
            signer: {
                documentation: "A reference to a signer construct, which will be used to sign the transaction payload.",
                typing: Type::string(),
                optional: false,
                tainting: true,
                internal: false
            },
            nonce: {
                documentation: "The account nonce of the signer. This value will be retrieved from the network if omitted.",
                typing: Type::integer(),
                optional: true,
                tainting: false,
                internal: false
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
        use txtx_addon_kit::constants::SIGNATURE_APPROVED;

        // todo: we need to make getting either "signers" or "signer" key more robust, and actually call `check_signability` for each signer
        let (signer_did, signer_instance) =
            get_signer_and_instance(values, signers_instances).unwrap();
        let construct_did = construct_did.clone();
        let instance_name = instance_name.to_string();
        let args = values.clone();
        let supervision_context = supervision_context.clone();
        let signers_instances = signers_instances.clone();

        let future = async move {
            let mut actions = Actions::none();

            let signer_state = signers.pop_signer_state(&signer_did).unwrap();

            if signer_state
                .get_scoped_value(&construct_did.to_string(), SIGNED_TRANSACTION_BYTES)
                .is_some()
                || signer_state
                    .get_scoped_value(&construct_did.to_string(), SIGNATURE_APPROVED)
                    .is_some()
            {
                return Ok((signers, signer_state, Actions::none()));
            }

            let payload = args.get_value(TRANSACTION_BYTES).unwrap().clone();

            let description =
                args.get_expected_string("description").ok().and_then(|d| Some(d.to_string()));

            let (signers, signer_state, mut signer_actions) =
                (signer_instance.specification.check_signability)(
                    &construct_did,
                    &instance_name,
                    &description,
                    &payload,
                    &signer_instance.specification,
                    &args,
                    signer_state,
                    signers,
                    &signers_instances,
                    &supervision_context,
                )?;
            actions.append(&mut signer_actions);
            return Ok((signers, signer_state, actions));
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
        // todo: we need to make getting either "signers" or "signer" key more robust, and actually call `check_signability` for each signer
        let (signer_did, _signer_instance) =
            get_signer_and_instance(values, signers_instances).unwrap();

        let signer_state = signers.pop_signer_state(&signer_did).unwrap();

        if let Some(signed_transaction_bytes) = signer_state
            .get_scoped_value(&construct_did.value().to_string(), SIGNED_TRANSACTION_BYTES)
        {
            let mut result = CommandExecutionResult::new();
            result
                .outputs
                .insert(SIGNED_TRANSACTION_BYTES.into(), signed_transaction_bytes.clone());
            return return_synchronous_ok(signers, signer_state, result);
        }

        let signer_instance = signers_instances.get(&signer_did).unwrap();

        let payload = if values.get_bool(IS_ARRAY).unwrap_or(false) {
            values.get_value(TRANSACTION_BYTES).unwrap().clone()
        } else {
            signer_state
                .get_scoped_value(&construct_did.to_string(), TRANSACTION_BYTES)
                .unwrap()
                .clone()
        };

        let title = values.get_expected_string("description").unwrap_or("New Transaction".into());

        let res = (signer_instance.specification.sign)(
            construct_did,
            title,
            &payload,
            &signer_instance.specification,
            &values,
            signer_state,
            signers,
            signers_instances,
        );
        res
    }
}

// todo: we need to make getting either "signers" or "signer" key more robust, and actually call `check_signability` for each signer
fn get_signer_and_instance(
    values: &ValueStore,
    signers_instances: &HashMap<ConstructDid, SignerInstance>,
) -> Result<(ConstructDid, SignerInstance), Diagnostic> {
    match get_signers_did(values) {
        Ok(signers_did) => {
            let first_signer_did =
                signers_did.first().ok_or_else(|| diagnosed_error!("No signers found"))?;
            let first_signer = signers_instances
                .get(first_signer_did)
                .ok_or_else(|| diagnosed_error!("Signer instance not found"))?;
            Ok((first_signer_did.clone(), first_signer.clone()))
        }
        Err(_) => {
            let signer_did = get_signer_did(values)?;
            let signer_instance = signers_instances
                .get(&signer_did)
                .ok_or_else(|| diagnosed_error!("Signer instance not found"))?;
            Ok((signer_did, signer_instance.clone()))
        }
    }
}
