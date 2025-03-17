use std::collections::HashMap;

use txtx_addon_kit::channel;
use txtx_addon_kit::constants::{SIGNATURE_APPROVED, SIGNATURE_SKIPPABLE};
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
use txtx_addon_kit::types::AuthorizationContext;
use txtx_addon_kit::types::ConstructDid;
use txtx_addon_kit::types::{
    commands::CommandSpecification,
    diagnostics::Diagnostic,
    types::{Type, Value},
};

use crate::constants::{
    ACTION_ITEM_CHECK_ADDRESS, ACTION_ITEM_PROVIDE_SIGNED_TRANSACTION, ADDRESS, CHECKED_ADDRESS,
    CHECKED_PUBLIC_KEY, FORMATTED_TRANSACTION, IS_SIGNABLE, NAMESPACE, NETWORK_ID, PUBLIC_KEY,
    TRANSACTION_BYTES,
};
use crate::typing::SvmValue;
use txtx_addon_kit::types::signers::return_synchronous_actions;
use txtx_addon_kit::types::types::RunbookSupervisionContext;

lazy_static! {
    pub static ref SVM_SQUADS: SignerSpecification = define_signer! {
        SvmSecretKey => {
            name: "Squads Signer",
            matcher: "squads",
            documentation:txtx_addon_kit::indoc! {r#"The `svm::squads` signer can be used to synchronously sign a transaction."#},
            inputs: [
                address: {
                    documentation: "The squad vault address.",
                    typing: Type::string(),
                    optional: true,
                    tainting: true,
                    sensitive: true
                }
            ],
            outputs: [
                public_key: {
                    documentation: "The public key of the squad vault.",
                    typing: Type::string()
                },
                address: {
                    documentation: "The SVM address generated from the secret key, mnemonic, or keypair file. This is an alias for the `public_key` output.",
                    typing: Type::string()
                }
            ],
            example: txtx_addon_kit::indoc! {r#"
            signer "deployer" "svm::squads" {
                address = input.address
            }
        "#},
        }
    };
}

pub struct SvmSecretKey;
impl SignerImplementation for SvmSecretKey {
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
        _auth_ctx: &AuthorizationContext,
        _is_balance_check_required: bool,
        _is_public_key_required: bool,
    ) -> SignerActionsFutureResult {
        let mut actions = Actions::none();

        if signer_state.get_value(CHECKED_PUBLIC_KEY).is_some() {
            return return_synchronous_actions(Ok((signers, signer_state, actions)));
        }

        let pubkey = SvmValue::to_pubkey(
            values
                .get_expected_value(ADDRESS)
                .map_err(|e| (signers.clone(), signer_state.clone(), e))?,
        )
        .map_err(|e| {
            (signers.clone(), signer_state.clone(), diagnosed_error!("invalid pubkey: {e}"))
        })?;

        let pubkey_value = SvmValue::pubkey(pubkey.to_bytes().to_vec());
        let pubkey_string_value = Value::string(pubkey.to_string());

        if supervision_context.review_input_values {
            if let Ok(_) = values.get_expected_string(CHECKED_ADDRESS) {
                signer_state.insert(CHECKED_PUBLIC_KEY, pubkey_value.clone());
                signer_state.insert(CHECKED_ADDRESS, pubkey_string_value.clone());
            } else {
                actions.push_sub_group(
                    None,
                    vec![ActionItemRequest::new(
                        &Some(construct_did.clone()),
                        &format!("Check {} expected address", instance_name),
                        None,
                        ActionItemStatus::Todo,
                        ReviewInputRequest::new("", &pubkey_string_value).to_action_type(),
                        ACTION_ITEM_CHECK_ADDRESS,
                    )],
                );
            }
        } else {
            signer_state.insert(CHECKED_PUBLIC_KEY, pubkey_value.clone());
            signer_state.insert(CHECKED_ADDRESS, pubkey_string_value.clone());
        }

        let future = async move { Ok((signers, signer_state, actions)) };
        Ok(Box::pin(future))
    }

    fn activate(
        _construct_did: &ConstructDid,
        _spec: &SignerSpecification,
        _values: &ValueStore,
        signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        _progress_tx: &channel::Sender<BlockEvent>,
    ) -> SignerActivateFutureResult {
        let mut result = CommandExecutionResult::new();
        let public_key = signer_state.get_value(CHECKED_PUBLIC_KEY).unwrap();
        let address = signer_state.get_value(CHECKED_ADDRESS).unwrap();
        result.outputs.insert(ADDRESS.into(), address.clone());
        result.outputs.insert(PUBLIC_KEY.into(), public_key.clone());
        return_synchronous_result(Ok((signers, signer_state, result)))
    }

    fn check_signability(
        construct_did: &ConstructDid,
        title: &str,
        description: &Option<String>,
        payload: &Value,
        _spec: &SignerSpecification,
        values: &ValueStore,
        mut signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        supervision_context: &RunbookSupervisionContext,
    ) -> Result<CheckSignabilityOk, SignerActionErr> {
        signer_state.insert_scoped_value(
            &construct_did.to_string(),
            TRANSACTION_BYTES,
            payload.clone(),
        );

        let actions = if supervision_context.review_input_values {
            let construct_did_str = &construct_did.to_string();
            if let Some(_) = signer_state.get_scoped_value(&construct_did_str, SIGNATURE_APPROVED) {
                return Ok((signers, signer_state, Actions::none()));
            }

            let network_id = match values.get_expected_string(NETWORK_ID) {
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
                    NAMESPACE,
                    &network_id,
                )
                .skippable(skippable)
                .check_expectation_action_uuid(construct_did)
                .formatted_payload(formatted_payload)
                .only_approval_needed()
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
        _payload: &Value,
        _spec: &SignerSpecification,
        _values: &ValueStore,
        signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
    ) -> SignerSignFutureResult {
        let result = CommandExecutionResult::new();

        return_synchronous_result(Ok((signers, signer_state, result)))
    }
}
