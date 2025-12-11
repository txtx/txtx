use alloy::primitives::Address;
use std::collections::HashMap;
use txtx_addon_kit::channel;
use txtx_addon_kit::constants::DESCRIPTION;
use txtx_addon_kit::types::frontend::{Actions, BlockEvent, ReviewInputRequest};
use txtx_addon_kit::types::signers::{
    return_synchronous_actions, return_synchronous_result, CheckSignabilityOk, SignerActionErr,
    SignerActionsFutureResult, SignerActivateFutureResult, SignerImplementation, SignerInstance,
    SignerSignFutureResult, SignerSpecification, SignersState,
};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::types::{RunbookSupervisionContext, Type, Value};
use txtx_addon_kit::types::commands::CommandSpecification;
use txtx_addon_kit::types::{diagnostics::Diagnostic, AuthorizationContext, ConstructDid};

use super::common::{activate_signer, check_signability, sign_transaction};
use crate::codec::crypto::{mnemonic_to_secret_key_signer, secret_key_to_secret_key_signer};
use crate::constants::{ACTION_ITEM_CHECK_ADDRESS, CHECKED_ADDRESS, PUBLIC_KEYS};
use crate::typing::EvmValue;

lazy_static! {
    pub static ref EVM_SECRET_KEY_SIGNER: SignerSpecification = define_signer! {
        EvmSecretKeySigner => {
          name: "EVM Secret Key Signer",
          matcher: "secret_key",
          documentation:txtx_addon_kit::indoc! {r#"The `evm::secret_key` signer can be used to synchronously sign a transaction."#},
          inputs: [
            secret_key: {
                documentation: "The secret key used to sign messages and transactions.",
                typing: Type::string(),
                optional: true,
                tainting: true,
                sensitive: true
            },
            mnemonic: {
                documentation: "The mnemonic phrase used to generate the secret key. This input will not be used if the `secret_key` input is provided.",
                typing: Type::string(),
                optional: true,
                tainting: true,
                sensitive: true
            },
            derivation_path: {
                documentation: "The derivation path used to generate the secret key. This input will not be used if the `secret_key` input is provided.",
                typing: Type::string(),
                optional: true,
                tainting: true,
                sensitive: true
            },
            is_encrypted: {
                documentation: "Coming soon",
                typing: Type::bool(),
                optional: true,
                tainting: true,
                sensitive: false
            },
            password: {
                documentation: "Coming soon",
                typing: Type::string(),
                optional: true,
                tainting: true,
                sensitive: true
            }
          ],
          outputs: [
              public_key: {
                documentation: "The public key of the account generated from the secret key.",
                typing: Type::array(Type::buffer())
              },
              address: {
                documentation: "The address generated from the secret key.",
                typing: Type::array(Type::buffer())
              }
          ],
          example: txtx_addon_kit::indoc! {r#"
            // we can create a secret key signer by providing a mnemonic and computing the secret key
            signer "bob" "evm::secret_key" {
                mnemonic = "board list obtain sugar hour worth raven scout denial thunder horse logic fury scorpion fold genuine phrase wealth news aim below celery when cabin"
                derivation_path = "m/44'/5757'/0'/0/0"
            }
            // or we can create one by providing the secret key directly
            signer "bob_again" "evm::secret_key" {
                secret_key = "03b3e0a76b292b2c83fc0ac14ae6160d0438ebe94e14bbb5b7755153628886e08e"
            }
        "#}
      }
    };
}

pub struct EvmSecretKeySigner;
impl SignerImplementation for EvmSecretKeySigner {
    fn check_instantiability(
        _ctx: &SignerSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    #[cfg(not(feature = "wasm"))]
    fn check_activability(
        construct_did: &ConstructDid,
        instance_name: &str,
        _spec: &SignerSpecification,
        values: &ValueStore,
        mut signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        supervision_context: &RunbookSupervisionContext,
        auth_ctx: &AuthorizationContext,
        _is_balance_check_required: bool,
        _is_public_key_required: bool,
    ) -> SignerActionsFutureResult {
        let mut actions = Actions::none();

        if signer_state.get_value(PUBLIC_KEYS).is_some() {
            return return_synchronous_actions(Ok((signers, signer_state, actions)));
        }

        let description = values.get_string(DESCRIPTION).map(|d| d.to_string());
        let markdown = values
            .get_markdown(auth_ctx)
            .map_err(|d| (signers.clone(), signer_state.clone(), d))?;

        let expected_signer =
            if let Ok(secret_key_bytes) = values.get_expected_buffer_bytes("secret_key") {
                secret_key_to_secret_key_signer(&secret_key_bytes)
                    .map_err(|e| (signers.clone(), signer_state.clone(), diagnosed_error!("{e}")))?
            } else {
                let mnemonic = values
                    .get_expected_string("mnemonic")
                    .map_err(|e| (signers.clone(), signer_state.clone(), e))?;
                let derivation_path = values.get_string("derivation_path");
                let is_encrypted = values.get_bool("is_encrypted");
                let password = values.get_string("password");
                mnemonic_to_secret_key_signer(mnemonic, derivation_path, is_encrypted, password)
                    .map_err(|e| (signers.clone(), signer_state.clone(), diagnosed_error!("{e}")))?
            };

        let expected_address: Address = expected_signer.address();

        signer_state.insert(
            "signer_field_bytes",
            EvmValue::signer_field_bytes(expected_signer.to_field_bytes().to_vec()),
        );
        signer_state.insert("signer_address", Value::string(expected_address.to_string()));

        if supervision_context.review_input_values {
            // Only show review action if address hasn't been checked yet
            if signer_state.get_expected_string(CHECKED_ADDRESS).is_err() {
                actions.push_sub_group(
                    None,
                    vec![ReviewInputRequest::new("", &Value::string(expected_address.to_string()))
                        .to_action_type()
                        .to_request(instance_name, ACTION_ITEM_CHECK_ADDRESS)
                        .with_construct_did(construct_did)
                        .with_some_description(description.clone())
                        .with_meta_description(&format!("Check {} expected address", instance_name))
                        .with_some_markdown(markdown.clone())],
                );
            }
        } else {
            signer_state.insert(CHECKED_ADDRESS, Value::string(expected_address.to_string()));
        }

        let future = async move { Ok((signers, signer_state, actions)) };
        Ok(Box::pin(future))
    }

    fn activate(
        _construct_id: &ConstructDid,
        _spec: &SignerSpecification,
        _values: &ValueStore,
        signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        _progress_tx: &channel::Sender<BlockEvent>,
    ) -> SignerActivateFutureResult {
        let (signers, signer_state, result) = activate_signer(signer_state, signers)?;
        return_synchronous_result(Ok((signers, signer_state, result)))
    }

    fn check_signability(
        construct_did: &ConstructDid,
        title: &str,
        description: &Option<String>,
        meta_description: &Option<String>,
        markdown: &Option<String>,
        payload: &Value,
        _spec: &SignerSpecification,
        values: &ValueStore,
        signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        supervision_context: &RunbookSupervisionContext,
        _auth_ctx: &AuthorizationContext,
    ) -> Result<CheckSignabilityOk, SignerActionErr> {
        check_signability(
            construct_did,
            title,
            description,
            meta_description,
            markdown,
            payload,
            values,
            signer_state,
            signers,
            supervision_context,
        )
    }

    fn sign(
        caller_uuid: &ConstructDid,
        _title: &str,
        _payload: &Value,
        _spec: &SignerSpecification,
        values: &ValueStore,
        signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
    ) -> SignerSignFutureResult {
        let caller_uuid = caller_uuid.clone();
        let values = values.clone();

        let future = async move { sign_transaction(&caller_uuid, &values, signer_state, signers).await };
        Ok(Box::pin(future))
    }
}
