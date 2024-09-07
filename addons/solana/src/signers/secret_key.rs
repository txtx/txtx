use std::collections::HashMap;

use txtx_addon_kit::constants::{
    SIGNATURE_APPROVED, SIGNATURE_SKIPPABLE, SIGNED_TRANSACTION_BYTES,
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
    ACTION_ITEM_CHECK_ADDRESS, ACTION_ITEM_PROVIDE_SIGNED_TRANSACTION, CHAIN_ID, IS_SIGNABLE,
};
use txtx_addon_kit::types::signers::return_synchronous_actions;
use txtx_addon_kit::types::types::RunbookSupervisionContext;

use crate::typing::SolanaValue;

lazy_static! {
    pub static ref SOLANA_SECRET_KEY: SignerSpecification = define_signer! {
        SolanaSecretKey => {
          name: "Mnemonic Signer",
          matcher: "secret_key",
          documentation:txtx_addon_kit::indoc! {r#"The `mnemonic` signer can be used to synchronously sign a transaction."#},
          inputs: [
            mnemonic: {
                documentation: "The mnemonic phrase used to generate the secret key.",
                typing: Type::string(),
                optional: false,
                tainting: true,
                sensitive: true
            },
            derivation_path: {
                documentation: "The derivation path used to generate the secret key.",
                typing: Type::string(),
                optional: true,
                tainting: true,
                sensitive: true
            },
            is_encrypted: {
                documentation: "Coming soon",
                typing: Type::bool(),
                optional: true,
                tainting: false,
                sensitive: false
            },
            password: {
                documentation: "Coming soon",
                typing: Type::string(),
                optional: true,
                tainting: false,
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
          // Coming soon
    "#},
      }
    };
}

pub struct SolanaSecretKey;
impl SignerImplementation for SolanaSecretKey {
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
        let mut actions = Actions::none();

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
        let result = CommandExecutionResult::new();
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

            let chain_id = match args.get_defaulting_string(CHAIN_ID, defaults) {
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
            let request = ActionItemRequest::new(
                &Some(construct_did.clone()),
                title,
                description.clone(),
                status,
                ProvideSignedTransactionRequest::new(
                    &signer_state.uuid,
                    &payload,
                    "stacks",
                    &chain_id,
                )
                .skippable(skippable)
                .check_expectation_action_uuid(construct_did)
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
        payload: &Value,
        _spec: &SignerSpecification,
        args: &ValueStore,
        signer_state: ValueStore,
        signers: SignersState,
        _signers_instances: &HashMap<ConstructDid, SignerInstance>,
        defaults: &AddonDefaults,
    ) -> SignerSignFutureResult {
        let mut result = CommandExecutionResult::new();

        let payload_bytes = match payload.try_get_buffer_bytes_result() {
            Ok(bytes) => bytes,
            Err(e) => return Err((signers, signer_state, diagnosed_error!("{e}"))),
        }
        .unwrap();

        let signed_transaction_bytes = vec![]; // solana_sdk::sign_transaction_with_mnemonic(payload_bytes, ..);

        result.outputs.insert(
            SIGNED_TRANSACTION_BYTES.to_string(),
            SolanaValue::transaction(signed_transaction_bytes),
        );

        return_synchronous_result(Ok((signers, signer_state, result)))
    }
}
