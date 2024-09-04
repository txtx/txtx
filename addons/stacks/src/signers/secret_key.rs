use std::collections::HashMap;

use crate::codec::crypto::{
    compute_keypair, secret_key_from_bytes, sign_message, sign_transaction,
};

use txtx_addon_kit::constants::{
    SIGNATURE_APPROVED, SIGNATURE_SKIPPABLE, SIGNED_MESSAGE_BYTES, SIGNED_TRANSACTION_BYTES,
};
use txtx_addon_kit::types::commands::CommandExecutionResult;
use txtx_addon_kit::types::frontend::{
    ActionItemRequest, ActionItemRequestType, ActionItemStatus, ProvideSignedTransactionRequest,
    ReviewInputRequest,
};
use txtx_addon_kit::types::frontend::{Actions, BlockEvent};
use txtx_addon_kit::types::signers::{
    return_synchronous_result, CheckSignabilityOk, SignerActionErr, SignerActionsFutureResult,
    SignerActivateFutureResult, SignerInstance, SignerSignFutureResult, SignersState,
};
use txtx_addon_kit::types::signers::{SignerImplementation, SignerSpecification};
use txtx_addon_kit::types::{
    commands::CommandSpecification,
    diagnostics::Diagnostic,
    types::{Type, Value},
};
use txtx_addon_kit::types::{ConstructDid, ValueStore};
use txtx_addon_kit::{channel, AddonDefaults};

use crate::constants::{
    ACTION_ITEM_CHECK_ADDRESS, ACTION_ITEM_PROVIDE_SIGNED_TRANSACTION, CHECKED_ADDRESS,
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
                optional: false,
                tainting: true,
                sensitive: true
            },
            mnemonic: {
                documentation: "The mnemonic phrase used to generate the secret key. This input will not be used if the `secret_key` input is provided.",
                typing: Type::string(),
                optional: false,
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
        signer "bob" "stacks::secret_key" {
            mnemonic = "board list obtain sugar hour worth raven scout denial thunder horse logic fury scorpion fold genuine phrase wealth news aim below celery when cabin"
            derivation_path = "m/44'/5757'/0'/0/0"
        }
    "#},
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
        args: &ValueStore,
        mut signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        defaults: &AddonDefaults,
        supervision_context: &RunbookSupervisionContext,
        _is_balance_check_required: bool,
        _is_public_key_required: bool,
    ) -> SignerActionsFutureResult {
        use crate::constants::CHECKED_ADDRESS;
        use crate::{
            codec::crypto::{secret_key_from_bytes, secret_key_from_mnemonic},
            constants::CHECKED_PUBLIC_KEY,
        };

        let mut actions = Actions::none();

        if signer_state.get_value(PUBLIC_KEYS).is_some() {
            return return_synchronous_actions(Ok((signers, signer_state, actions)));
        }
        let network_id = args
            .get_defaulting_string(NETWORK_ID, defaults)
            .map_err(|diag| (signers.clone(), signer_state.clone(), diag))?;

        let secret_key =
            if let Ok(secret_key_bytes) = args.get_expected_buffer_bytes("secret_key") {
                secret_key_from_bytes(&secret_key_bytes).map_err(|e| {
                    (
                        signers.clone(),
                        signer_state.clone(),
                        diagnosed_error!("signer 'stacks::secret_key': {e}"),
                    )
                })?
            } else {
                let mnemonic = args
                    .get_expected_string("mnemonic")
                    .map_err(|diag| (signers.clone(), signer_state.clone(), diag))?;
                let derivation_path = args.get_string("derivation_path");
                let is_encrypted = args.get_bool("is_encrypted").unwrap_or(false);
                let password = args.get_string("password");
                secret_key_from_mnemonic(mnemonic, derivation_path, is_encrypted, password)
                    .map_err(|e| {
                        (
                            signers.clone(),
                            signer_state.clone(),
                            diagnosed_error!("signer 'stacks::secret_key': {e}"),
                        )
                    })?
            };

        let (_, public_key, expected_address) =
            compute_keypair(secret_key, network_id).map_err(|e| {
                (
                    signers.clone(),
                    signer_state.clone(),
                    diagnosed_error!("signer 'stacks::secret_key': {e}"),
                )
            })?;

        signer_state.insert(PUBLIC_KEYS, Value::array(vec![public_key.clone()]));
        signer_state.insert(CHECKED_PUBLIC_KEY, public_key.clone());
        signer_state.insert(CHECKED_ADDRESS, Value::string(expected_address.to_string()));

        if supervision_context.review_input_values {
            actions.push_sub_group(
                None,
                vec![ActionItemRequest::new(
                    &None,
                    &format!("Check {} expected address", instance_name),
                    None,
                    ActionItemStatus::Todo,
                    ActionItemRequestType::ReviewInput(ReviewInputRequest {
                        input_name: "".into(),
                        value: Value::string(expected_address.to_string()),
                    }),
                    ACTION_ITEM_CHECK_ADDRESS,
                )],
            );
        }
        let future = async move { Ok((signers, signer_state, actions)) };
        Ok(Box::pin(future))
    }

    fn activate(
        _construct_id: &ConstructDid,
        _spec: &SignerSpecification,
        _args: &ValueStore,
        signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        _defaults: &AddonDefaults,
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
        args: &ValueStore,
        signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        defaults: &AddonDefaults,
        supervision_context: &RunbookSupervisionContext,
    ) -> Result<CheckSignabilityOk, SignerActionErr> {
        let actions = if supervision_context.review_input_values {
            let construct_did_str = &construct_did.to_string();
            if let Some(_) = signer_state.get_scoped_value(&construct_did_str, SIGNATURE_APPROVED) {
                return Ok((signers, signer_state, Actions::none()));
            }

            let network_id = match args.get_defaulting_string(NETWORK_ID, defaults) {
                Ok(value) => value,
                Err(diag) => return Err((signers, signer_state, diag)),
            };
            let signable = signer_state
                .get_scoped_value(&construct_did_str, IS_SIGNABLE)
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let status = match signable {
                true => ActionItemStatus::Todo,
                false => ActionItemStatus::Blocked,
            };
            let skippable = signer_state
                .get_scoped_value(&construct_did_str, SIGNATURE_SKIPPABLE)
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let formatted_payload = signer_state
                .get_scoped_value(&construct_did_str, FORMATTED_TRANSACTION)
                .and_then(|v| v.as_string())
                .and_then(|v| Some(v.to_string()));

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
                ACTION_ITEM_PROVIDE_SIGNED_TRANSACTION,
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
        args: &ValueStore,
        signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        defaults: &AddonDefaults,
    ) -> SignerSignFutureResult {
        let mut result = CommandExecutionResult::new();

        let network_id = args
            .get_defaulting_string(NETWORK_ID, defaults)
            .map_err(|diag| (signers.clone(), signer_state.clone(), diag))?;
        let secret_key_bytes = args
            .get_expected_buffer_bytes("secret_key")
            .map_err(|diag| (signers.clone(), signer_state.clone(), diag))?;
        let secret_key = secret_key_from_bytes(&secret_key_bytes).map_err(|e| {
            (
                signers.clone(),
                signer_state.clone(),
                diagnosed_error!("signer 'stacks::secret_key': {e}"),
            )
        })?;
        let (_, public_key_value, _) = compute_keypair(secret_key, network_id).map_err(|e| {
            (
                signers.clone(),
                signer_state.clone(),
                diagnosed_error!("signer 'stacks::secret_key': {e}"),
            )
        })?;

        let payload_buffer = payload.expect_addon_data();
        if payload_buffer.id.eq(&STACKS_TRANSACTION) {
            let signed_transaction_bytes =
                sign_transaction(&payload_buffer.bytes, secret_key_bytes).map_err(|e| {
                    (
                        signers.clone(),
                        signer_state.clone(),
                        diagnosed_error!("signer 'stacks::secret_key': {e}"),
                    )
                })?;

            result.outputs.insert(
                SIGNED_TRANSACTION_BYTES.into(),
                StacksValue::transaction(signed_transaction_bytes),
            );
        } else {
            let (message_bytes, signature_bytes) =
                sign_message(&payload_buffer.bytes, secret_key_bytes, public_key_value.to_bytes())
                    .map_err(|e| {
                        (
                            signers.clone(),
                            signer_state.clone(),
                            diagnosed_error!("signer 'stacks::secret_key': {e}"),
                        )
                    })?;

            let message = StacksValue::signature(message_bytes);
            let signature = StacksValue::signature(signature_bytes);
            result.outputs.insert(MESSAGE_BYTES.into(), message);
            result.outputs.insert(SIGNED_MESSAGE_BYTES.into(), signature);
        }

        return_synchronous_result(Ok((signers, signer_state, result)))
    }
}
