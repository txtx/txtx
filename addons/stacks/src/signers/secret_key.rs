use std::collections::HashMap;

use crate::codec::crypto::{compute_keypair, sign_message, sign_transaction};

use txtx_addon_kit::channel;
use txtx_addon_kit::constants::{
    SignerKey::SignatureApproved, SignerKey::SignatureSkippable, SIGNED_MESSAGE_BYTES, SignerKey::SignedTransactionBytes,
};
use txtx_addon_kit::crypto::secret_key_from_bytes;
use txtx_addon_kit::types::commands::CommandExecutionResult;
use txtx_addon_kit::types::frontend::{
    ActionItemRequest, ActionItemStatus, ProvideSignedTransactionRequest, ReviewInputRequest,
};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::signers::{
    return_synchronous_result, CheckSignabilityOk, SignerActionErr, SignerActionsFutureResult,
    SignerActivateFutureResult, SignerInstance, SignerSignFutureResult, SignersState,
};
use txtx_addon_kit::types::signers::{SignerImplementation, SignerSpecification};
use txtx_addon_kit::types::stores::ValueStore;
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::types::{
    commands::CommandSpecification,
    diagnostics::Diagnostic,
    types::{Type, Value},
};

use crate::constants::{
    ActionItemKey::CheckAddress, ActionItemKey::ProvideSignedTransaction, CHECKED_ADDRESS,
    FORMATTED_TRANSACTION, IS_SIGNABLE, MESSAGE_BYTES,
};
use txtx_addon_kit::types::signers::return_synchronous_actions;
use txtx_addon_kit::types::types::RunbookSupervisionContext;

use crate::constants::{NETWORK_ID, PUBLIC_KEYS};
use crate::typing::StacksValue;
use crate::typing::STACKS_TRANSACTION;

lazy_static! {
    pub static ref STACKS_SECRET_KEY: SignerSpecification = define_signer! {
        StacksSecretKey => {
          name: "Secret Key Signer",
          matcher: "secret_key",
          documentation:txtx_addon_kit::indoc! {r#"The `stacks::secret_key` signer can be used to synchronously sign a transaction."#},
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
              }
          ],
          example: txtx_addon_kit::indoc! {r#"
            // we can create a secret key signer by providing a mnemonic and computing the secret key
            signer "bob" "stacks::secret_key" {
                mnemonic = "board list obtain sugar hour worth raven scout denial thunder horse logic fury scorpion fold genuine phrase wealth news aim below celery when cabin"
                derivation_path = "m/44'/5757'/0'/0/0"
            }
            // or we can create one by providing the secret key directly
            signer "bob_again" "stacks::secret_key" {
                secret_key = "03b3e0a76b292b2c83fc0ac14ae6160d0438ebe94e14bbb5b7755153628886e08e"
            }
        "#}
      }
    };
}

pub struct StacksSecretKey;
impl SignerImplementation for StacksSecretKey {
    fn check_instantiability(
        _ctx: &SignerSpecification,
        _args: Vec<Type>,
    ) -> Result<Type, Diagnostic> {
        unimplemented!()
    }

    #[cfg(not(feature = "wasm"))]
    fn check_activability(
        _construct_id: &ConstructDid,
        instance_name: &str,
        _spec: &SignerSpecification,
        values: &ValueStore,
        mut signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        supervision_context: &RunbookSupervisionContext,
        _auth_ctx: &txtx_addon_kit::types::AuthorizationContext,
        _is_balance_check_required: bool,
        _is_public_key_required: bool,
    ) -> SignerActionsFutureResult {
        use txtx_addon_kit::crypto::{secret_key_bytes_from_mnemonic, secret_key_from_bytes};

        use crate::codec::crypto::version_from_network_id;
        use crate::constants::CHECKED_ADDRESS;
        use crate::constants::CHECKED_PUBLIC_KEY;
        use crate::signers::DEFAULT_DERIVATION_PATH;

        let mut actions = Actions::none();

        if signer_state.get_value(PUBLIC_KEYS).is_some() {
            return return_synchronous_actions(Ok((signers, signer_state, actions)));
        }
        let network_id = values
            .get_expected_string(NETWORK_ID)
            .map_err(|e| (signers.clone(), signer_state.clone(), e))?;
        let version = version_from_network_id(&network_id);

        let secret_key_bytes = match values.get_expected_buffer_bytes("secret_key") {
            Ok(value) => value,
            Err(_) => {
                let mnemonic = values
                    .get_expected_string("mnemonic")
                    .map_err(|diag| (signers.clone(), signer_state.clone(), diag))?;
                let derivation_path =
                    values.get_string("derivation_path").unwrap_or(DEFAULT_DERIVATION_PATH);
                let is_encrypted = values.get_bool("is_encrypted").unwrap_or(false);
                let password = values.get_string("password");
                secret_key_bytes_from_mnemonic(mnemonic, derivation_path, is_encrypted, password)
                    .map_err(|e| (signers.clone(), signer_state.clone(), diagnosed_error!("{e}")))?
                    .to_vec()
            }
        };

        let secret_key = secret_key_from_bytes(&secret_key_bytes)
            .map_err(|e| (signers.clone(), signer_state.clone(), diagnosed_error!("{e}")))?;

        let (secret_key, public_key, expected_address) = compute_keypair(secret_key, network_id)
            .map_err(|e| (signers.clone(), signer_state.clone(), diagnosed_error!("{e}")))?;

        signer_state.insert("hash_flag", Value::integer(version.into()));
        signer_state.insert("multi_sig", Value::bool(false));
        signer_state.insert(PUBLIC_KEYS, Value::array(vec![public_key.clone()]));
        signer_state.insert(CHECKED_PUBLIC_KEY, public_key.clone());
        signer_state.insert(CHECKED_ADDRESS, Value::string(expected_address.to_string()));
        signer_state.insert("secret_key", secret_key);

        if supervision_context.review_input_values {
            actions.push_sub_group(
                None,
                vec![ActionItemRequest::new(
                    &None,
                    &format!("Check {} expected address", instance_name),
                    None,
                    ActionItemStatus::Todo,
                    ReviewInputRequest::new("", &Value::string(expected_address.to_string()))
                        .to_action_type(),
                    ActionItemKey::CheckAddress,
                )],
            );
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
        let mut result = CommandExecutionResult::new();
        let address = signer_state.get_value(CHECKED_ADDRESS).unwrap();
        result.outputs.insert("address".into(), address.clone());
        return_synchronous_result(Ok((signers, signer_state, result)))
    }

    fn check_signability(
        construct_did: &ConstructDid,
        title: &str,
        description: &Option<String>,
        payload: &Value,
        _spec: &SignerSpecification,
        values: &ValueStore,
        signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        supervision_context: &RunbookSupervisionContext,
    ) -> Result<CheckSignabilityOk, SignerActionErr> {
        let actions = if supervision_context.review_input_values {
            let construct_did_str = &construct_did.to_string();
            if let Some(_) = signer_state.get_scoped_value(&construct_did_str, SignerKey::SignatureApproved) {
                return Ok((signers, signer_state, Actions::none()));
            }

            let network_id = values
                .get_expected_string(NETWORK_ID)
                .map_err(|e| (signers.clone(), signer_state.clone(), e))?;

            let signable = signer_state
                .get_scoped_value(&construct_did_str, IS_SIGNABLE)
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let status = match signable {
                true => ActionItemStatus::Todo,
                false => ActionItemStatus::Blocked,
            };
            let skippable = signer_state
                .get_scoped_value(&construct_did_str, SignerKey::SignatureSkippable)
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let formatted_payload =
                signer_state.get_scoped_value(&construct_did_str, FORMATTED_TRANSACTION);

            let request = ActionItemRequest::new(
                &Some(construct_did.clone()),
                title,
                description.clone(),
                status,
                ProvideSignedTransactionRequest::new(
                    &signer_state.uuid,
                    &payload,
                    "stacks",
                    &network_id,
                )
                .skippable(skippable)
                .check_expectation_action_uuid(construct_did)
                .only_approval_needed()
                .formatted_payload(formatted_payload)
                .to_action_type(),
                ActionItemKey::ProvideSignedTransaction,
            );
            Actions::append_item(
                request,
                Some("Review and sign the transactions from the list below"),
                Some("Transaction Signing"),
            )
        } else {
            Actions::none()
        };
        Ok((signers, signer_state, actions))
    }

    fn sign(
        _caller_uuid: &ConstructDid,
        _title: &str,
        payload: &Value,
        _spec: &SignerSpecification,
        values: &ValueStore,
        signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
    ) -> SignerSignFutureResult {
        let mut result = CommandExecutionResult::new();

        let network_id = values
            .get_expected_string(NETWORK_ID)
            .map_err(|e| (signers.clone(), signer_state.clone(), e))?;
        let secret_key_bytes = signer_state
            .get_expected_buffer_bytes("secret_key")
            .map_err(|e| (signers.clone(), signer_state.clone(), e))?;

        let secret_key = secret_key_from_bytes(&secret_key_bytes)
            .map_err(|e| (signers.clone(), signer_state.clone(), diagnosed_error!("{e}")))?;

        let (_, public_key_value, _) = compute_keypair(secret_key, network_id)
            .map_err(|e| (signers.clone(), signer_state.clone(), diagnosed_error!("{e}")))?;

        let payload_buffer = payload.expect_addon_data();
        if payload_buffer.id.eq(&STACKS_TRANSACTION) {
            let signed_transaction_bytes =
                sign_transaction(&payload_buffer.bytes, secret_key_bytes).map_err(|e| {
                    (signers.clone(), signer_state.clone(), diagnosed_error!("{e}"))
                })?;

            result.outputs.insert(
                SignerKey::SignedTransactionBytes,
                StacksValue::transaction(signed_transaction_bytes),
            );
        } else {
            let (message_bytes, signature_bytes) =
                sign_message(&payload_buffer.bytes, secret_key_bytes, public_key_value.to_bytes())
                    .map_err(|e| {
                        (signers.clone(), signer_state.clone(), diagnosed_error!("{e}"))
                    })?;

            let message = StacksValue::signature(message_bytes);
            let signature = StacksValue::signature(signature_bytes);
            result.outputs.insert(MESSAGE_BYTES.into(), message);
            result.outputs.insert(SIGNED_MESSAGE_BYTES.into(), signature);
        }

        return_synchronous_result(Ok((signers, signer_state, result)))
    }
}
